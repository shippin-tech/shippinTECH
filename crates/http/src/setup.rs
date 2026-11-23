//! First-run setup creates the reserved control-plane administrator and retires
//! internal installer authority. Hosted mail domains are configured separately.

use crate::{
    auth::authenticate::HttpHeaders,
    panel::{acceptable_password, internal_call, set_created},
};
use common::{
    Server,
    auth::AccessToken,
    manager::{
        setup_key::{mark_owner_claimed, owner_claimed, retire_setup_key},
        system_domain::{ROOT_ACCOUNT, ensure_system_domain},
    },
};
use http_proto::{HttpRequest, HttpResponse, HttpSessionData, JsonResponse, ToHttpResponse};
use hyper::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateRoot {
    password: String,
}

fn reply(status: StatusCode, value: Value) -> trc::Result<HttpResponse> {
    Ok(JsonResponse::with_status(status, value)
        .no_cache()
        .into_http_response())
}

fn problem(status: StatusCode, code: &str, message: &str) -> trc::Result<HttpResponse> {
    reply(status, json!({ "error": code, "message": message }))
}

/// Invoked inside the mail container by the installer, with the password on stdin.
/// No secret is accepted in argv, printed, or sent outside container loopback.
pub async fn run_local_claim(status_only: bool) -> i32 {
    match local_claim(status_only).await {
        Ok(code) => code,
        Err(message) => {
            eprintln!("Claim failed: {message}");
            1
        }
    }
}

async fn local_claim(status_only: bool) -> Result<i32, String> {
    use std::io::Read;
    use utils::HttpLimitResponse;
    let client = utils::http::local_http_client_builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "Could not initialize local HTTP client.")?;
    let url = "http://127.0.0.1:8080/api/v1/setup";
    let response = client.get(url).send().await.map_err(
        |_| "The local server is not ready on port 8080. Start it and retry bash claim.sh.",
    )?;
    if !response.status().is_success() {
        return Err("Cannot read local claim state.".into());
    }
    let bytes = response
        .bytes_with_limit(8192)
        .await
        .ok()
        .flatten()
        .ok_or("Invalid local claim response.")?;
    let status: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid local claim response.")?;
    match status["pending"].as_bool() {
        Some(false) => {
            println!("This server is already claimed; existing administrators are unchanged.");
            return Ok(3);
        }
        Some(true) => {}
        None => return Err("Missing local claim state.".into()),
    }
    if status_only {
        return Ok(0);
    }
    let mut password = String::new();
    std::io::stdin()
        .take(1025)
        .read_to_string(&mut password)
        .map_err(|_| "Could not read administrator password from stdin.")?;
    if password.ends_with('\n') {
        password.pop();
        if password.ends_with('\r') {
            password.pop();
        }
    }
    if password.contains(['\n', '\r']) {
        return Err("Provide one password line.".into());
    }
    acceptable_password(&password).map_err(str::to_owned)?;
    let key = std::fs::read_to_string("/var/lib/shippin.tech/setup-key").map_err(
        |_| "Local install authority is unavailable. Restart the mail container and retry.",
    )?;
    let response = client.post(format!("{url}/admin"))
        .basic_auth("admin", Some(key.trim()))
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&json!({"password": password})).unwrap())
        .send().await.map_err(|_| "Claim response was interrupted. Retry; an existing administrator will not be replaced.")?;
    if response.status().is_success() {
        println!("Administrator root@system.local created. Sign in with the password you chose.");
        Ok(0)
    } else if response.status() == StatusCode::CONFLICT {
        Err("This server was already claimed; no password was changed.".into())
    } else if response.status() == StatusCode::FORBIDDEN {
        Err(
            "Local install authority expired. Restart the mail container and retry bash claim.sh."
                .into(),
        )
    } else if response.status() == StatusCode::BAD_REQUEST {
        Err(
            "Administrator was not created. Use a stronger password and retry bash claim.sh."
                .into(),
        )
    } else {
        Err("The server could not finish the claim. Check its logs and retry bash claim.sh; an existing administrator will not be replaced.".into())
    }
}

pub trait SetupApi: Sync + Send {
    fn handle_setup_request(
        &self,
        req: &HttpRequest,
        session: &HttpSessionData,
        path: &[String],
        body: Option<Vec<u8>>,
    ) -> impl Future<Output = trc::Result<HttpResponse>> + Send;
}

impl SetupApi for Server {
    async fn handle_setup_request(
        &self,
        req: &HttpRequest,
        session: &HttpSessionData,
        path: &[String],
        body: Option<Vec<u8>>,
    ) -> trc::Result<HttpResponse> {
        let pending = !owner_claimed(
            self.registry(),
            &self.core.network.security.default_role_ids_admin,
        )
        .await?;
        match (
            path.first().map(String::as_str).unwrap_or_default(),
            req.method(),
        ) {
            ("", &Method::GET) => reply(
                StatusCode::OK,
                json!({ "pending": pending, "administrator": ROOT_ACCOUNT }),
            ),
            ("admin", &Method::POST) => {
                if !pending {
                    return problem(
                        StatusCode::CONFLICT,
                        "done",
                        "This server already has an administrator.",
                    );
                }
                self.is_http_anonymous_request_allowed(session.remote_ip)
                    .await?;
                // This credential authorizes exactly one claim, never a general login.
                let valid = if let Some(expected) = self.registry().setup_key()
                    && let Some((scheme, value)) = req.authorization()
                    && scheme.eq_ignore_ascii_case("basic")
                    && let Some(decoded) =
                        mail_parser::decoders::base64::base64_decode(value.as_bytes())
                    && let Ok(decoded) = String::from_utf8(decoded)
                    && let Some(("admin", secret)) = decoded.split_once(':')
                {
                    directory::core::secret::verify_secret_hash(&expected, secret.as_bytes())
                        .await?
                } else {
                    false
                };
                if !valid {
                    // A concurrent successful claim may have retired the token since
                    // the initial status read. Report closure, not bad credentials.
                    if owner_claimed(
                        self.registry(),
                        &self.core.network.security.default_role_ids_admin,
                    )
                    .await?
                    {
                        return problem(
                            StatusCode::CONFLICT,
                            "done",
                            "This server is already claimed.",
                        );
                    }
                    return problem(
                        StatusCode::FORBIDDEN,
                        "notSetup",
                        "Claim this server through its local installer (bash claim.sh).",
                    );
                }
                let Some(body) = body
                    .as_deref()
                    .and_then(|bytes| serde_json::from_slice::<CreateRoot>(bytes).ok())
                else {
                    return problem(
                        StatusCode::BAD_REQUEST,
                        "invalidRequest",
                        "Provide the password for the system administrator. Mail domains are configured separately.",
                    );
                };
                if let Err(reason) = acceptable_password(&body.password) {
                    return problem(StatusCode::BAD_REQUEST, "weakPassword", reason);
                }
                let domain_id = ensure_system_domain(self.registry()).await?;
                let response = internal_call(self, &AccessToken::new_admin(), session, "x:Account/set", json!({
                    "create": { "a": {
                        "@type": "User", "name": "root", "domainId": domain_id.to_string(),
                        "credentials": { "0": { "@type": "Password", "secret": body.password } },
                        "roles": { "@type": "Admin" }, "description": "System administrator"
                    } }
                })).await;
                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        // The account's unique key is the cross-node claim arbiter.
                        // A losing transaction must not surface as a broken install.
                        if owner_claimed(
                            self.registry(),
                            &self.core.network.security.default_role_ids_admin,
                        )
                        .await?
                        {
                            return problem(
                                StatusCode::CONFLICT,
                                "done",
                                "This server is already claimed.",
                            );
                        }
                        return Err(error);
                    }
                };
                if let Err(reason) = set_created(&response, "a") {
                    if owner_claimed(
                        self.registry(),
                        &self.core.network.security.default_role_ids_admin,
                    )
                    .await?
                    {
                        return problem(
                            StatusCode::CONFLICT,
                            "done",
                            "This server is already claimed.",
                        );
                    }
                    return problem(StatusCode::BAD_REQUEST, "adminRejected", &reason);
                }
                mark_owner_claimed(self.registry()).await?;
                retire_setup_key(self.registry());
                reply(StatusCode::OK, json!({ "email": ROOT_ACCOUNT }))
            }
            _ => Err(trc::ResourceEvent::NotFound.into_err()),
        }
    }
}

#[cfg(test)]
mod system_domain_tests {
    use super::*;

    #[test]
    fn setup_accepts_only_a_password_not_a_mail_domain_or_identity() {
        assert!(
            serde_json::from_value::<CreateRoot>(json!({"password":"example fixture"})).is_ok()
        );
        for request in [
            json!({}),
            json!({"password":7}),
            json!({"password":"example fixture","email":"owner@example.test"}),
            json!({"password":"example fixture","domainId":"1"}),
        ] {
            assert!(serde_json::from_value::<CreateRoot>(request).is_err());
        }
    }
}
