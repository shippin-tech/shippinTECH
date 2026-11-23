

use crate::utils::{jmap::JmapUtils, server::TestServer};
use common::auth::credential::{ApiKey, AppPassword};
use jmap_proto::error::set::SetErrorType;
use registry::{
    schema::{
        enums::StorageQuota,
        prelude::{ObjectType, Property},
        structs::{
            self, Account, Credential, Http, PasswordCredential, SecondaryCredential, UserAccount,
        },
    },
    types::{EnumImpl, datetime::UTCDateTime, ipmask::IpAddrOrMask, list::List, map::Map},
};
use serde_json::json;
use std::str::FromStr;
use store::write::now;
use types::id::Id;

#[tokio::test]
async fn system_domain_bootstrap_is_durable_and_idempotent() {
    use crate::utils::{storage::build_data_store, temp_dir::TempDir};
    use common::manager::system_domain::{ensure_system_domain, seed_system_identity, SYSTEM_DOMAIN};
    use common::manager::setup_key::{mark_owner_claimed, owner_claimed, prepare_setup_key};
    use registry::schema::structs::{Domain, SystemSettings};
    use store::{RegistryStore, Store};
    let temp = TempDir::new("system-domain-bootstrap", true);
    let path = temp.path.to_string_lossy().to_string();
    let backend = std::env::var("STORE").unwrap_or_else(|_| "Sqlite".into());
    let store = Store::build(build_data_store(&backend, &path).await).await.unwrap();
    store.create_tables().await.unwrap();
    crate::utils::cleanup::store_destroy(&store).await;
    let registry = RegistryStore::new(&path, store.clone(), "mail.example.test".into(), 1, None).await;
    seed_system_identity(&registry).await.unwrap();
    let id = ensure_system_domain(&registry).await.unwrap();
    let settings = registry.object::<SystemSettings>(Id::singleton()).await.unwrap().unwrap();
    assert_eq!(settings.default_domain_id, id);
    assert_eq!(registry.object::<Domain>(id).await.unwrap().unwrap().name, SYSTEM_DOMAIN);
    assert!(!owner_claimed(&registry, &[]).await.unwrap());
    mark_owner_claimed(&registry).await.unwrap();
    drop(registry);
    drop(store);
    let store = Store::build(build_data_store(&backend, &path).await).await.unwrap();
    let reopened = RegistryStore::new(&path, store, "mail.example.test".into(), 1, None).await;
    seed_system_identity(&reopened).await.unwrap();
    assert_eq!(ensure_system_domain(&reopened).await.unwrap(), id);
    assert_eq!(reopened.count_object(ObjectType::Domain).await.unwrap(), 1);
    assert_eq!(reopened.object::<SystemSettings>(Id::singleton()).await.unwrap().unwrap(), settings);
    // Ownership stays closed even when there are no administrator accounts or local key.
    assert!(owner_claimed(&reopened, &[]).await.unwrap());
    prepare_setup_key(&reopened, &[]).await;
    assert!(reopened.setup_key().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn system_domain_root_provisions_mail_without_becoming_a_mailbox() {
    use crate::utils::{account::Account as TestAccount, server::TestServerBuilder};
    use common::{auth::AuthRequest, manager::system_domain::{ensure_system_domain, ROOT_ACCOUNT}, network::RcptResolution};
    use registry::schema::enums::Permission;

    let test = TestServerBuilder::new("system-domain-lifecycle").await
        .with_http_listener(18898).await.build().await;
    let recovery = test.account("admin");
    let registry = test.server.registry();
    let system_id = ensure_system_domain(registry).await.unwrap();
    assert!(registry.has_env_recovery_admin());
    common::manager::setup_key::prepare_setup_key(registry, &test.server.core.network.security.default_role_ids_admin).await;
    assert!(registry.setup_key().is_some(), "optional recovery must not block owner claim");
    let install_key = "synthetic setup fixture";
    registry.set_setup_key(Some((install_key.into(), now() + 600)));
    let count_before = registry.count_object(ObjectType::Domain).await.unwrap();
    let client = reqwest::Client::builder().danger_accept_invalid_certs(true).build().unwrap();
    let password = "independent system administrator fixture";
    let endpoint = "https://127.0.0.1:18898/api/v1/setup/admin";
    registry.set_setup_key(Some((install_key.into(), now() - 1)));
    let expired_status: serde_json::Value = client.get("https://127.0.0.1:18898/api/v1/setup")
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(expired_status["pending"], true, "expiry does not mean claimed");
    assert_eq!(client.post(endpoint).basic_auth("admin", Some(install_key))
        .json(&json!({"password":password})).send().await.unwrap().status(), 403);
    registry.set_setup_key(Some((install_key.into(), now() + 600)));
    assert_eq!(client.get("https://127.0.0.1:18898/api/v1/account")
        .basic_auth("admin", Some(install_key)).send().await.unwrap().status(), 401);
    // Recovery login neither consumes ownership nor grants installer authority.
    assert_eq!(client.post(endpoint)
        .basic_auth(recovery.name(), Some(recovery.secret()))
        .json(&json!({"password":password})).send().await.unwrap().status(), 403);
    assert_eq!(client.post(endpoint).basic_auth("admin", Some(install_key))
        .json(&json!({"password":"123"})).send().await.unwrap().status(), 400);
    let status: serde_json::Value = client.get("https://127.0.0.1:18898/api/v1/setup")
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(status["pending"], true);
    // Two simultaneous claims can create only one root; neither can replace it.
    let request = || client.post(endpoint).basic_auth("admin", Some(install_key))
        .json(&json!({"password":password})).send();
    let (first, second) = tokio::join!(request(), request());
    let (first, second) = (first.unwrap(), second.unwrap());
    let (response, refused) = if first.status() == 200 { (first, second) } else { (second, first) };
    assert_eq!(response.status(), 200, "root setup failed");
    assert_eq!(refused.status(), 409, "concurrent claim must be refused");
    assert_eq!(response.json::<serde_json::Value>().await.unwrap()["email"], ROOT_ACCOUNT);
    assert!(registry.setup_key().is_none());
    assert_eq!(client.post(endpoint).basic_auth("admin", Some(install_key))
        .json(&json!({"password":"attempted replacement password"})).send().await.unwrap().status(), 409);
    assert_eq!(registry.count_object(ObjectType::Domain).await.unwrap(), count_before);

    let auth = |name: &str| AuthRequest::from_plain(name, password, 0, "127.0.0.1".parse().unwrap());
    let token = test.server.authenticate(&auth(ROOT_ACCOUNT)).await.unwrap();
    let root_id = Id::from(token.account_id());
    assert_eq!(test.server.authenticate(&auth("root")).await.unwrap().account_id(), token.account_id());
    for permission in [Permission::ImapAuthenticate, Permission::EmailSend, Permission::EmailReceive,
        Permission::Pop3Authenticate, Permission::SieveAuthenticate] {
        assert!(!token.has_permission(permission));
    }
    assert!(matches!(test.server.rcpt_resolve(ROOT_ACCOUNT, true, 0).await.unwrap(), RcptResolution::UnknownRecipient));
    let mut root = TestAccount::new(ROOT_ACCOUNT, password, &[], "Root", root_id);
    root.http_listener_port = 18898;

    // Root creates a normal mail user with send/receive rights it cannot exercise itself.
    let user = root.create_user_account("reader@independent.test", "normal mail account fixture", "Reader", &[], vec![]).await;
    let user_token = test.server.authenticate(&AuthRequest::from_plain(user.name(), user.secret(), 0, "127.0.0.1".parse().unwrap())).await.unwrap();
    assert!(user_token.has_permission(Permission::EmailSend));
    let mail_domain = root.find_or_create_domain("independent.test").await;
    root.registry_update_object_expect_err(ObjectType::Domain, system_id, json!({"name":"stolen.test"})).await.assert_type(SetErrorType::Forbidden);
    root.registry_destroy_object_expect_err(ObjectType::Domain, system_id).await.assert_type(SetErrorType::Forbidden);
    root.registry_update_object_expect_err(ObjectType::Domain, mail_domain, json!({"aliases":{"system.local":true}})).await.assert_type(SetErrorType::Forbidden);
    root.registry_update_object_expect_err(ObjectType::Account, root_id, json!({"domainId":mail_domain.to_string()})).await.assert_type(SetErrorType::Forbidden);
    root.registry_update_object(ObjectType::SystemSettings, Id::singleton(), json!({"defaultDomainId":system_id.to_string()})).await;
    root.reload_settings().await;
    {
        use common::BuildServer;
        let current = test.server.inner.build_server();
        assert_eq!(current.core.email.default_domain_name, "system.local");
        assert_eq!(current.mail_domain_name(), current.core.network.server_name);
        assert_ne!(current.mail_domain_name(), "system.local");
        assert_eq!(current.authenticate(&auth("root")).await.unwrap().account_id(), token.account_id());
    }
    root.registry_update_object_expect_err(ObjectType::SystemSettings, Id::singleton(), json!({"defaultDomainId":mail_domain.to_string()})).await.assert_type(SetErrorType::Forbidden);

    root.registry_destroy(ObjectType::Account, [user.id()]).await.assert_destroyed(&[user.id()]);
    root.registry_destroy(ObjectType::Domain, [mail_domain]).await.assert_destroyed(&[mail_domain]);
    assert_eq!(test.server.authenticate(&auth(ROOT_ACCOUNT)).await.unwrap().account_id(), token.account_id());
    assert!(registry.object::<registry::schema::structs::Domain>(system_id).await.unwrap().is_some());
    test.shutdown();
}

pub async fn test(test: &TestServer) {
    println!("Running Authentication tests...");

    let admin = test.account("admin@example.org");
    let domain_id = admin.find_or_create_domain("example.org").await;

    // Enable X-Forwarded-For processing to test IP-based access restrictions
    admin
        .registry_update_setting(
            Http {
                use_x_forwarded: true,
                ..Default::default()
            },
            &[Property::UseXForwarded],
        )
        .await;
    admin.reload_settings().await;

    // Weak passwords should be rejected
    admin
        .registry_create_object_expect_err(Account::User(UserAccount {
            name: "user".to_string(),
            domain_id,
            credentials: List::from_iter([Credential::Password(PasswordCredential {
                secret: "12345".to_string(),
                ..Default::default()
            })]),
            ..Default::default()
        }))
        .await
        .assert_type(SetErrorType::InvalidProperties)
        .assert_description_contains("Password must be at least 8 characters long.");
    admin
        .registry_create_object_expect_err(Account::User(UserAccount {
            name: "user".to_string(),
            domain_id,
            credentials: List::from_iter([Credential::Password(PasswordCredential {
                secret: "12345678".to_string(),
                ..Default::default()
            })]),
            ..Default::default()
        }))
        .await
        .assert_type(SetErrorType::InvalidProperties)
        .assert_description_contains(concat!(
            "Password is too weak. This is a top-10 common password. ",
            "Add another word or two. Uncommon words are better."
        ));

    // Adding secondary credentials should not be allowed
    admin
        .registry_create_object_expect_err(Account::User(UserAccount {
            name: "user".to_string(),
            domain_id,
            credentials: List::from_iter([Credential::AppPassword(SecondaryCredential {
                description: "Test app password".to_string(),
                ..Default::default()
            })]),
            ..Default::default()
        }))
        .await
        .assert_type(SetErrorType::InvalidProperties)
        .assert_description_contains("Secondary credentials cannot be set directly");
    admin
        .registry_create_object_expect_err(Account::User(UserAccount {
            name: "user".to_string(),
            domain_id,
            credentials: List::from_iter([Credential::ApiKey(SecondaryCredential {
                description: "Test API key".to_string(),
                ..Default::default()
            })]),
            ..Default::default()
        }))
        .await
        .assert_type(SetErrorType::InvalidProperties)
        .assert_description_contains("Secondary credentials cannot be set directly");

    // Creating a user with a valid password should succeed
    let user_id = admin
        .registry_create_object(Account::User(UserAccount {
            name: "user".to_string(),
            domain_id,
            credentials: List::from_iter([Credential::Password(PasswordCredential {
                secret: "this is a very strong password".to_string(),
                ..Default::default()
            })]),
            ..Default::default()
        }))
        .await;
    validate_password("user@example.org", "this is a very strong password", true).await;
    validate_password("user@example.org", "wrong password", false).await;

    // Change password as admin
    admin
        .registry_update_object_expect_err(
            ObjectType::Account,
            user_id,
            json!({
                "credentials/0/secret": "12345"
            }),
        )
        .await
        .assert_type(SetErrorType::InvalidProperties)
        .assert_description_contains("Password must be at least 8 characters long.");
    admin
        .registry_update_object(
            ObjectType::Account,
            user_id,
            json!({
                "credentials/0/secret": "very strong password indeed"
            }),
        )
        .await;
    validate_password("user@example.org", "this is a very strong password", false).await;
    validate_password("user@example.org", "very strong password indeed", true).await;

    // Set password expiration in two seconds and verify it works
    admin
        .registry_update_object(
            ObjectType::Account,
            user_id,
            json!({
               "credentials/0/expiresAt": UTCDateTime::from_timestamp((now() + 2) as i64)
            }),
        )
        .await;
    let mut user = crate::utils::account::Account::new(
        "user@example.org",
        "very strong password indeed",
        &[],
        "User",
        user_id,
    );
    user.registry_query_ids(
        ObjectType::PublicKey,
        Vec::<(&str, &str)>::new(),
        Vec::<&str>::new(),
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    assert_eq!(
        user.registry_query(
            ObjectType::PublicKey,
            Vec::<(&str, &str)>::new(),
            Vec::<&str>::new(),
        )
        .await
        .method_response()
        .text_field("type"),
        "forbidden"
    );

    // Password updates should require the old password
    user.registry_update_object_expect_err(
        ObjectType::AccountPassword,
        Id::singleton(),
        json!({
            Property::Secret: "12345"
        }),
    )
    .await
    .assert_type(SetErrorType::Forbidden)
    .assert_description_contains(
        "Current secret must be provided to change the password or OTP auth.",
    );

    // Password policies should be enforced when changing password
    user.registry_update_object_expect_err(
        ObjectType::AccountPassword,
        Id::singleton(),
        json!({
            Property::CurrentSecret: "very strong password indeed",
            Property::Secret: "12345"
        }),
    )
    .await
    .assert_type(SetErrorType::InvalidProperties)
    .assert_description_contains("Password must be at least 8 characters long.");

    // Perform a valid password update
    user.registry_update_object(
        ObjectType::AccountPassword,
        Id::singleton(),
        json!({
            Property::CurrentSecret: "very strong password indeed",
            Property::Secret: "user provided strong password"
        }),
    )
    .await;
    validate_password("user@example.org", "very strong password indeed", false).await;
    validate_password("user@example.org", "user provided strong password", true).await;
    user.update_secret("user provided strong password");

    // After a successful password change, the user permissions should be restored
    user.registry_query_ids(
        ObjectType::PublicKey,
        Vec::<(&str, &str)>::new(),
        Vec::<&str>::new(),
    )
    .await;

    // Limit login to specific IPs and set credential quotas
    admin
        .registry_update_object(
            ObjectType::Account,
            user_id,
            json!({
                "credentials/0/allowedIps": {"192.168.1.1": true},
                Property::Quotas: {
                    StorageQuota::MaxApiKeys.as_str(): 1,
                    StorageQuota::MaxAppPasswords.as_str(): 1,
                 }
            }),
        )
        .await;
    validate_password_with_ip(
        "user@example.org",
        "user provided strong password",
        "192.168.1.1",
        true,
    )
    .await;
    validate_password_with_ip(
        "user@example.org",
        "user provided strong password",
        "192.168.1.2",
        false,
    )
    .await;
    admin
        .registry_update_object(
            ObjectType::Account,
            user_id,
            json!({
                "credentials/0/allowedIps": {},
            }),
        )
        .await;

    // Create an IP-restricted App Password and verify it works
    let response = user
        .registry_create([structs::AppPassword {
            allowed_ips: Map::new(vec![IpAddrOrMask::from_str("10.0.0.2").unwrap()]),
            description: "My app password".to_string(),
            ..Default::default()
        }])
        .await;
    let app_password = response.created(0);
    let app_password_id = app_password.object_id();
    let app_password_secret = app_password.text_field("secret").to_string();
    let _ = AppPassword::parse(&app_password_secret).unwrap();
    validate_password_with_ip("user@example.org", &app_password_secret, "10.0.0.2", true).await;
    validate_password_with_ip("user@example.org", &app_password_secret, "10.0.0.3", false).await;

    // Create an IP-restricted API key and verify it works
    let response = user
        .registry_create([structs::ApiKey {
            allowed_ips: Map::new(vec![IpAddrOrMask::from_str("10.0.0.2").unwrap()]),
            description: "My API key".to_string(),
            ..Default::default()
        }])
        .await;
    let api_key = response.created(0);
    let api_key_id = api_key.object_id();
    let api_key_secret = api_key.text_field("secret").to_string();
    let _ = ApiKey::parse(&api_key_secret).unwrap();
    validate_token_with_ip(&api_key_secret, "10.0.0.2", true).await;
    validate_token_with_ip(&api_key_secret, "10.0.0.3", false).await;

    // Creating more API keys or app passwords should fail due to quota
    user.registry_create_object_expect_err(structs::AppPassword {
        description: "Another app password".to_string(),
        ..Default::default()
    })
    .await
    .assert_type(SetErrorType::OverQuota)
    .assert_description_contains("You have exceeded your quota of 1 app passwords.");
    user.registry_create_object_expect_err(structs::ApiKey {
        description: "Another API key".to_string(),
        ..Default::default()
    })
    .await
    .assert_type(SetErrorType::OverQuota)
    .assert_description_contains("You have exceeded your quota of 1 API keys.");

    // Set a credential expiration in the past and verify it is rejected
    for (credential_id, object_type) in [
        (app_password_id, ObjectType::AppPassword),
        (api_key_id, ObjectType::ApiKey),
    ] {
        user.registry_update_object(
            object_type,
            credential_id,
            json!({
                Property::ExpiresAt: UTCDateTime::now()
            }),
        )
        .await;
    }
    validate_token_with_ip(&api_key_secret, "10.0.0.2", false).await;
    validate_password_with_ip("user@example.org", &app_password_secret, "10.0.0.2", false).await;

    // Destroy the API key and app password, then verify they no longer work
    for (credential_id, object_type) in [
        (app_password_id, ObjectType::AppPassword),
        (api_key_id, ObjectType::ApiKey),
    ] {
        let response = user.registry_destroy(object_type, [credential_id]).await;
        assert_eq!(
            vec![credential_id],
            response.destroyed_ids().collect::<Vec<_>>()
        );
    }
    validate_token_with_ip(&api_key_secret, "10.0.0.2", false).await;
    validate_password_with_ip("user@example.org", &app_password_secret, "10.0.0.2", false).await;
    validate_password("user@example.org", "user provided strong password", true).await;

    // Clean up
    assert_eq!(
        admin
            .registry_destroy(ObjectType::Account, [user_id])
            .await
            .destroyed_ids()
            .collect::<Vec<_>>(),
        vec![user_id]
    );
    validate_password("user@example.org", "user provided strong password", false).await;

    // Disable X-Forwarded-For processing
    admin
        .registry_update_setting(
            Http {
                use_x_forwarded: false,
                ..Default::default()
            },
            &[Property::UseXForwarded],
        )
        .await;
    admin.reload_settings().await;

    test.cleanup().await;
}

pub async fn validate_password(username: &str, password: &str, is_valid: bool) {
    validate_password_with_ip(username, password, "127.0.0.1", is_valid).await;
}

pub async fn validate_password_with_ip(
    username: &str,
    password: &str,
    remote_ip: &str,
    is_valid: bool,
) {
    let response = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get("https://127.0.0.1:8899/.well-known/jmap")
        .basic_auth(username, Some(password))
        .header("X-Forwarded-For", remote_ip)
        .send()
        .await
        .unwrap();

    let status = response.status();
    if status.is_success() != is_valid {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        panic!(
            "Expected password to be {}. Server responded with status {}: {}",
            if is_valid { "valid" } else { "invalid" },
            status,
            text
        );
    }
}

pub async fn validate_token_with_ip(token: &str, remote_ip: &str, is_valid: bool) {
    let response = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get("https://127.0.0.1:8899/.well-known/jmap")
        .bearer_auth(token)
        .header("X-Forwarded-For", remote_ip)
        .send()
        .await
        .unwrap();

    let status = response.status();
    if status.is_success() != is_valid {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        panic!(
            "Expected token to be {}. Server responded with status {}: {}",
            if is_valid { "valid" } else { "invalid" },
            status,
            text
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn install_defects_short_names_route_without_cross_account_fallback() {
    use crate::utils::server::TestServerBuilder;
    use common::auth::AuthRequest;

    let test = TestServerBuilder::new("install_defects_short_names")
        .await
        .with_http_listener(18897)
        .await
        .build()
        .await;
    let admin = test.account("admin");
    let first = admin.create_user_account(
        "unique@other.test", "first independent account secret", "First", &[], vec![],
    ).await;
    let auth = |name: &str, secret: &str| AuthRequest::from_plain(
        name, secret, 0, "127.0.0.1".parse().unwrap(),
    );
    for name in ["unique", "UNIQUE", "unique@other.test"] {
        let token = test.server.authenticate(&auth(name, first.secret())).await.unwrap();
        assert_eq!(token.account_id(), first.id().document_id());
    }
    assert!(test.server.authenticate(&auth("unique", "wrong password")).await.is_err());

    // A same-name group is not an authentication candidate.
    let group = admin.create_group_account("unique@group.test", "Group", &[]).await;
    assert_eq!(test.server.authenticate(&auth("unique", first.secret())).await.unwrap().account_id(), first.id().document_id());

    let second = admin.create_user_account(
        "unique@another.test", "second independent account secret", "Second", &[], vec![],
    ).await;
    // Neither password resolves an ambiguous name. Full addresses still work.
    for user in [&first, &second] {
        assert!(test.server.authenticate(&auth("unique", user.secret())).await.is_err());
        assert_eq!(test.server.authenticate(&auth(user.name(), user.secret())).await.unwrap().account_id(), user.id().document_id());
    }

    // Existing default-domain behavior wins; a failed password is not tried
    // against the same name on another domain.
    let default = admin.create_user_account(
        "unique@example.org", "default independent account secret", "Default", &[], vec![],
    ).await;
    assert_eq!(test.server.authenticate(&auth("unique", default.secret())).await.unwrap().account_id(), default.id().document_id());
    assert!(test.server.authenticate(&auth("unique", first.secret())).await.is_err());
    assert!(test.server.authenticate(&auth("absent", first.secret())).await.is_err());

    admin.registry_destroy(ObjectType::Account, [first.id(), second.id(), default.id(), group.id()]).await;
    // Cleanup needs the task worker running. These auth fixtures run with
    // shippin.tech_KEEP_DELETED_DAYS=0, so deletion is due immediately.
    tokio::time::timeout(std::time::Duration::from_secs(30), test.cleanup())
        .await.expect("authentication fixture cleanup did not finish");
    test.shutdown();
}
