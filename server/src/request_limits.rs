//! Process-wide admission limits for the single-operator HTTP service.
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::Semaphore;

const BURST: f64 = 120.0;
const PER_SECOND: f64 = 20.0;
const IN_FLIGHT: usize = 32;

struct Bucket {
    available: f64,
    updated: Instant,
}
impl Bucket {
    fn take(&mut self, now: Instant) -> bool {
        self.available = (self.available
            + now.saturating_duration_since(self.updated).as_secs_f64() * PER_SECOND)
            .min(BURST);
        self.updated = now;
        if self.available < 1.0 {
            return false;
        }
        self.available -= 1.0;
        true
    }
}

#[derive(Clone)]
struct Limits {
    bucket: Arc<Mutex<Bucket>>,
    handlers: Arc<Semaphore>,
}
impl Limits {
    fn new() -> Self {
        Self {
            bucket: Arc::new(Mutex::new(Bucket {
                available: BURST,
                updated: Instant::now(),
            })),
            handlers: Arc::new(Semaphore::new(IN_FLIGHT)),
        }
    }
}

/// Apply outside authentication so failed authentication and unknown routes count.
/// State is bounded and shared by all connections; forwarding headers are ignored.
pub fn protect(router: Router) -> Router {
    router.layer(middleware::from_fn_with_state(Limits::new(), admit))
}

fn overloaded() -> Response {
    let mut response = crate::error(
        StatusCode::TOO_MANY_REQUESTS,
        "request_limit_exceeded",
        "Control-plane request capacity exceeded. Wait before retrying.",
    )
    .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    response
}

async fn admit(State(limits): State<Limits>, request: Request, next: Next) -> Response {
    // Never hold the synchronous mutex across an await or queue excess requests.
    let allowed = limits
        .bucket
        .lock()
        .map(|mut b| b.take(Instant::now()))
        .unwrap_or(false);
    if !allowed {
        return overloaded();
    }
    let Ok(_permit) = limits.handlers.try_acquire() else {
        return overloaded();
    };
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::get};
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };
    use tower::ServiceExt;

    #[test]
    fn bucket_refills_and_caps_without_waiting_or_using_wall_clock() {
        let start = Instant::now();
        let mut bucket = Bucket {
            available: BURST,
            updated: start,
        };
        for _ in 0..120 {
            assert!(bucket.take(start));
        }
        assert!(!bucket.take(start));
        assert!(!bucket.take(start + Duration::from_millis(25)));
        assert!(bucket.take(start + Duration::from_millis(50)));
        assert!(!bucket.take(start + Duration::from_millis(50)));
        let later = start + Duration::from_secs(60);
        for _ in 0..120 {
            assert!(bucket.take(later));
        }
        assert!(!bucket.take(later));
    }

    fn request() -> Request {
        Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn shared_admission_rejects_before_handlers_and_releases_capacity() {
        let limits = Limits::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let app = Router::new()
            .route(
                "/api/v1/health",
                get(move || {
                    observed.fetch_add(1, Ordering::SeqCst);
                    async { StatusCode::OK }
                }),
            )
            .layer(middleware::from_fn_with_state(limits.clone(), admit));
        let permits = limits
            .handlers
            .acquire_many(IN_FLIGHT as u32)
            .await
            .unwrap();
        let response = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(permits);
        assert_eq!(
            app.clone().oneshot(request()).await.unwrap().status(),
            StatusCode::OK
        );
        limits.bucket.lock().unwrap().available = 0.0;
        limits.bucket.lock().unwrap().updated = Instant::now() + Duration::from_secs(10);
        assert_eq!(
            app.oneshot(request()).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn authentication_and_fallbacks_remain_inside_admission() {
        let limits = Limits::new();
        let token =
            crate::auth::AccessToken::parse("fixture-only-access-token-0123456789abcdef").unwrap();
        let app = crate::auth::protect(crate::router(), Some(token))
            .layer(middleware::from_fn_with_state(limits.clone(), admit));
        assert_eq!(
            app.clone().oneshot(request()).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        limits.bucket.lock().unwrap().available = 0.0;
        limits.bucket.lock().unwrap().updated = Instant::now() + Duration::from_secs(10);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(!response.headers().contains_key("x-vmanyware-authenticated"));
    }
}
