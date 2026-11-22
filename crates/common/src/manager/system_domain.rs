//! Bootstrap identity namespace, independent of hosted mail domains.
use registry::schema::{
    prelude::{ObjectType, Property},
    structs::{
        CertificateManagement, DkimManagement, DnsManagement, Domain, SubAddressing, SystemSettings,
    },
};
use store::{
    RegistryStore,
    registry::write::{RegistryWrite, RegistryWriteResult},
};
use types::id::Id;

pub const SYSTEM_DOMAIN: &str = "system.local";
pub const ROOT_ACCOUNT: &str = "root@system.local";

pub fn is_system_domain(name: &str) -> bool {
    name.trim_end_matches('.')
        .eq_ignore_ascii_case(SYSTEM_DOMAIN)
}

/// Public report senders must not expose the reserved authentication namespace.
pub fn mail_domain_name<'a>(identity_domain: &'a str, hostname: &'a str) -> &'a str {
    if is_system_domain(identity_domain) {
        hostname
    } else {
        identity_domain
    }
}

pub fn reserved_domain() -> Domain {
    Domain {
        name: SYSTEM_DOMAIN.into(),
        description: Some("Reserved ShippinTECH control-plane identity domain".into()),
        is_enabled: false,
        certificate_management: CertificateManagement::Manual,
        dkim_management: DkimManagement::Manual,
        dns_management: DnsManagement::Manual,
        sub_addressing: SubAddressing::Disabled,
        report_address_uri: None,
        ..Default::default()
    }
}

pub fn reserved_domain_is_valid(domain: &Domain) -> bool {
    let mut expected = reserved_domain();
    expected.created_at = domain.created_at;
    domain == &expected
}

pub fn domain_change_is_reserved(domain: &Domain, old: Option<&Domain>) -> bool {
    is_system_domain(&domain.name)
        || domain.aliases.iter().any(|name| is_system_domain(name))
        || old.is_some_and(|old| is_system_domain(&old.name))
}

/// Idempotent on a configured persistent store. Never repurpose a colliding domain.
pub async fn ensure_system_domain(registry: &RegistryStore) -> trc::Result<Id> {
    let existing = registry
        .primary_key(
            ObjectType::Domain.into(),
            Property::Name,
            SYSTEM_DOMAIN.as_bytes().to_vec(),
        )
        .await?;
    let id = if let Some(existing) = existing {
        existing.id()
    } else {
        if registry
            .primary_key(
                ObjectType::Domain.into(),
                Property::Aliases,
                SYSTEM_DOMAIN.as_bytes().to_vec(),
            )
            .await?
            .is_some()
        {
            return Err(trc::ResourceEvent::Error
                .into_err()
                .details("Reserved system domain conflicts with a domain alias"));
        }
        match registry
            .write(RegistryWrite::insert(&reserved_domain().into()))
            .await
        {
            Ok(RegistryWriteResult::Success(id)) => id,
            Ok(RegistryWriteResult::PrimaryKeyConflict { existing_id, .. }) => existing_id.id(),
            Ok(other) => {
                return Err(trc::ResourceEvent::Error
                    .into_err()
                    .details(format!("Cannot seed system domain: {other}")));
            }
            Err(error) => {
                // Another node may have won the unique-name assertion. Read and
                // validate that object; otherwise preserve the actual store error.
                match registry
                    .primary_key(
                        ObjectType::Domain.into(),
                        Property::Name,
                        SYSTEM_DOMAIN.as_bytes().to_vec(),
                    )
                    .await?
                {
                    Some(existing) => existing.id(),
                    None => return Err(error),
                }
            }
        }
    };
    if !registry
        .object::<Domain>(id)
        .await?
        .as_ref()
        .is_some_and(reserved_domain_is_valid)
    {
        return Err(trc::ResourceEvent::Error
            .into_err()
            .details("Reserved system domain has conflicting configuration"));
    }
    Ok(id)
}

pub async fn seed_system_identity(registry: &RegistryStore) -> trc::Result<()> {
    let domain_id = ensure_system_domain(registry).await?;
    if registry
        .object::<SystemSettings>(Id::singleton())
        .await?
        .is_none()
    {
        match registry
            .write(RegistryWrite::insert(
                &SystemSettings {
                    default_hostname: registry.local_hostname().into(),
                    default_domain_id: domain_id,
                    ..Default::default()
                }
                .into(),
            ))
            .await?
        {
            RegistryWriteResult::Success(_) => {}
            other => {
                return Err(trc::ResourceEvent::Error
                    .into_err()
                    .details(format!("Cannot seed system settings: {other}")));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod system_domain_tests {
    use super::*;

    #[test]
    fn reports_use_public_hostname_without_changing_mail_domain_defaults() {
        assert_eq!(
            mail_domain_name(SYSTEM_DOMAIN, "mail.example.test"),
            "mail.example.test"
        );
        assert_eq!(
            mail_domain_name("example.test", "mail.example.test"),
            "example.test"
        );
    }

    #[test]
    fn reserved_domain_cannot_acquire_mail_or_tenant_configuration() {
        let domain = reserved_domain();
        assert!(reserved_domain_is_valid(&domain));
        assert!(domain_change_is_reserved(&domain, None));
        let mut renamed = domain.clone();
        renamed.name = "mail.example.test".into();
        assert!(domain_change_is_reserved(&renamed, Some(&domain)));
        assert!(!domain_change_is_reserved(&renamed, None));
        renamed.aliases = registry::types::map::Map::new(vec!["SYSTEM.LOCAL.".into()]);
        assert!(domain_change_is_reserved(&renamed, None));
        let mut tenant = domain.clone();
        tenant.member_tenant_id = Some(Id::new(42));
        assert!(!reserved_domain_is_valid(&tenant));
        let mut enabled = domain;
        enabled.is_enabled = true;
        assert!(!reserved_domain_is_valid(&enabled));
    }
}
