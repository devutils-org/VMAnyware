//! Optional single-operator bearer authentication, never a Proxmox token.
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct AccessToken([u8; 32]);
impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccessToken { [REDACTED] }")
    }
}
#[derive(Debug)]
pub struct InvalidAccessToken;
impl std::fmt::Display for InvalidAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VMANYWARE_ACCESS_TOKEN must contain 32–256 URL-safe token characters")
    }
}
impl std::error::Error for InvalidAccessToken {}
fn valid(value: &[u8]) -> bool {
    (32..=256).contains(&value.len())
        && value
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(b))
}
impl AccessToken {
    pub fn parse(value: &str) -> Result<Self, InvalidAccessToken> {
        if !valid(value.as_bytes()) {
            return Err(InvalidAccessToken);
        }
        Ok(Self(Sha256::digest(value.as_bytes()).into()))
    }
    pub fn from_environment() -> Result<Option<Self>, InvalidAccessToken> {
        match std::env::var("VMANYWARE_ACCESS_TOKEN") {
            Ok(value) => Self::parse(&value).map(Some),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(_) => Err(InvalidAccessToken),
        }
    }
}
/// Keep existing local development behavior only when no token was configured.
/// This does not expose a listener or provide TLS; remote access requires HTTPS.
pub fn protect(router: Router, token: Option<AccessToken>) -> Router {
    match token {
        None => router,
        Some(token) => router.layer(middleware::from_fn_with_state(token, authorize)),
    }
}
async fn authorize(State(expected): State<AccessToken>, request: Request, next: Next) -> Response {
    let mut values = request.headers().get_all(header::AUTHORIZATION).iter();
    let supplied = values
        .next()
        .filter(|_| values.next().is_none())
        .map(|v| v.as_bytes());
    let accepted = supplied
        .filter(|v| v.len() >= 7 && v[..7].eq_ignore_ascii_case(b"Bearer "))
        .map(|v| &v[7..])
        .filter(|v| valid(v))
        .is_some_and(|token| {
            let hash: [u8; 32] = Sha256::digest(token).into();
            bool::from(expected.0.ct_eq(&hash))
        });
    if accepted {
        let mut response = next.run(request).await;
        response.headers_mut().insert(
            "x-vmanyware-authenticated",
            axum::http::HeaderValue::from_static("bearer"),
        );
        response
    } else {
        let mut response = crate::error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "A valid control-plane access token is required.",
        )
        .into_response();
        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            axum::http::HeaderValue::from_static("Bearer"),
        );
        response
    }
}
