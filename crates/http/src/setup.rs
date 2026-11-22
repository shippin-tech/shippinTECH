//! First-run setup creates the reserved control-plane administrator and retires
//! the printed key. Hosted mail domains are configured separately.

use crate::{
    auth::authenticate::Authenticator,
    panel::{acceptable_password, internal_call, set_created},
};
use common::{
    Server,
    auth::{AccessToken, RECOVERY_ADMIN_ID},
    manager::{
        setup_key::retire_setup_key,
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
        let pending = self.registry().setup_key().is_some();
        match (
            path.first().map(String::as_str).unwrap_or_default(),
            req.method(),
        ) {
            ("", &Method::GET) => reply(
                StatusCode::OK,
                json!({ "pending": pending, "administrator": ROOT_ACCOUNT }),
            ),
            ("admin", &Method::POST) => {
                let (_in_flight, access_token) = self.authenticate_headers(req, session).await?;
                if !pending {
                    return problem(
                        StatusCode::CONFLICT,
                        "done",
                        "This server already has an administrator.",
                    );
                }
                if access_token.account_id() != RECOVERY_ADMIN_ID {
                    return problem(
                        StatusCode::FORBIDDEN,
                        "notSetup",
                        "Sign in as admin with the setup key first.",
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
                })).await?;
                if let Err(reason) = set_created(&response, "a") {
                    return problem(StatusCode::BAD_REQUEST, "adminRejected", &reason);
                }
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
