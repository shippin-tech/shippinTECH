//! Internal installer authority. Never printed or accepted for ordinary sign-in.
use registry::{
    schema::{
        enums::AccountType,
        prelude::{ObjectType, Property},
        structs::{Account, UserAccount, UserRoles},
    },
    types::EnumImpl,
};
use store::{
    RegistryStore,
    rand::{RngExt, distr::Alphanumeric, rng},
    registry::RegistryQuery,
};
use types::id::Id;

/// Whether a system administrator exists; tenant administrators do not own the host.
pub async fn has_admin_account(
    registry: &RegistryStore,
    admin_role_ids: &[Id],
) -> trc::Result<bool> {
    let ids = registry
        .query::<Vec<Id>>(
            RegistryQuery::new(ObjectType::Account)
                .equal(Property::Type, AccountType::User.to_id()),
        )
        .await?;
    for id in ids {
        if let Some(Account::User(user)) = registry.object::<Account>(id).await?
            && holds_system_admin_role(&user, admin_role_ids)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn holds_system_admin_role(user: &UserAccount, admin_role_ids: &[Id]) -> bool {
    user.member_tenant_id.is_none()
        && match &user.roles {
            UserRoles::Admin => true,
            UserRoles::Custom(custom) => custom
                .role_ids
                .iter()
                .any(|role| admin_role_ids.contains(role)),
            _ => false,
        }
}

/// Turns the first-run key off and removes it from disk.
pub fn retire_setup_key(registry: &RegistryStore) {
    registry.set_setup_key(None);
    let _ = std::fs::remove_file(registry.setup_key_path());
}

fn claim_marker() -> store::ValueKey<store::write::ValueClass> {
    store::ValueKey::from(store::write::ValueClass::InMemory(
        store::write::InMemoryClass::Key(b"\xffshippintech.owner-claimed.v1".to_vec()),
    ))
}

/// Durable ownership is independent of a local token's expiry or recovery login.
pub async fn owner_claimed(registry: &RegistryStore, admin_role_ids: &[Id]) -> trc::Result<bool> {
    if registry
        .store()
        .get_value::<u64>(claim_marker())
        .await?
        .is_some()
    {
        return Ok(true);
    }
    if has_admin_account(registry, admin_role_ids).await? {
        mark_owner_claimed(registry).await?;
        return Ok(true);
    }
    Ok(false)
}

pub async fn mark_owner_claimed(registry: &RegistryStore) -> trc::Result<()> {
    use store::write::{BatchBuilder, Operation, ValueOp};
    let mut batch = BatchBuilder::new();
    // The in-memory subspace's first eight bytes are expiry: never expire this marker.
    batch.any_op(Operation::Value {
        class: claim_marker().class,
        op: ValueOp::Set(u64::MAX.to_be_bytes().to_vec()),
    });
    registry.store().write(batch.build_all()).await.map(|_| ())
}

fn write_private(path: &std::path::Path, key: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(format!("{key}\n").as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

const HOUR: u64 = 3600;

/// How long an install key lasts: `SETUP_KEY_HOURS`, from 1 hour to 30 days, 24 by default.
pub fn lifetime(hours: Option<&str>) -> u64 {
    hours
        .and_then(|hours| hours.trim().parse::<u64>().ok())
        .filter(|hours| (1..=720).contains(hours))
        .unwrap_or(24)
        * HOUR
}

pub fn still_valid(made: u64, now: u64, lifetime: u64) -> bool {
    now < made.saturating_add(lifetime)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// When the saved key was made: the file's last change.
fn made_at(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// Called once at startup on a configured server.
pub async fn prepare_setup_key(registry: &RegistryStore, admin_role_ids: &[Id]) {
    if registry.is_bootstrap_mode() {
        return;
    }
    match owner_claimed(registry, admin_role_ids).await {
        Ok(true) => {
            retire_setup_key(registry);
            return;
        }
        Ok(false) => {}
        Err(err) => {
            trc::error!(err.details("Could not check for an administrator account"));
            return;
        }
    }
    let path = registry.setup_key_path();
    let lifetime = lifetime(
        std::env::var("shippin.tech_SETUP_KEY_HOURS")
            .ok()
            .as_deref(),
    );
    let now = unix_now();
    let saved = std::fs::read_to_string(&path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|key| key.len() >= 16)
        .zip(made_at(&path))
        .filter(|(_, made)| still_valid(*made, now, lifetime));
    let (key, made) = match saved {
        Some(saved) => saved,
        None => {
            let key = rng()
                .sample_iter(Alphanumeric)
                .take(24)
                .map(char::from)
                .collect::<String>();
            if let Err(err) = write_private(&path, &key) {
                eprintln!(
                    "Could not save internal install authority to {}: {err}",
                    path.display()
                );
                return;
            }
            (key, now)
        }
    };
    let until = made.saturating_add(lifetime);
    registry.set_setup_key(Some((key.clone(), until)));
    eprintln!(
        "ShippinTECH is unclaimed. Complete installation or run bash claim.sh in the instance directory."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_administrator_does_not_claim_host_ownership() {
        let mut user = UserAccount::default();
        assert!(!holds_system_admin_role(&user, &[]));
        user.roles = UserRoles::Admin;
        assert!(holds_system_admin_role(&user, &[]));
        user.member_tenant_id = Some(Id::from(1u32));
        assert!(!holds_system_admin_role(&user, &[]));
    }

    #[test]
    fn an_install_key_lasts_24_hours_unless_told_otherwise() {
        assert_eq!(lifetime(None), 24 * HOUR);
        assert_eq!(lifetime(Some("2")), 2 * HOUR);
        assert_eq!(lifetime(Some(" 48 ")), 48 * HOUR);
        assert_eq!(lifetime(Some("0")), 24 * HOUR);
        assert_eq!(lifetime(Some("100000")), 24 * HOUR);
        assert_eq!(lifetime(Some("soon")), 24 * HOUR);
    }

    #[test]
    fn an_install_key_stops_working_when_it_runs_out() {
        let made = 1_000_000;
        assert!(still_valid(made, made, HOUR));
        assert!(still_valid(made, made + HOUR - 1, HOUR));
        assert!(!still_valid(made, made + HOUR, HOUR));
    }
}
