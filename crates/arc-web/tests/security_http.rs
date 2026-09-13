//! Real TCP HTTP checks: middleware must reject before calling the protected service.
use actix_web::{dev::Service, web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer};
use arc_core::session::{InMemorySessionStore, SessionRecord, SessionStore, SessionStoreError};
use arc_web::helpers::jwt::{create_token, get_jwt_secret};
use arc_web::http::middlewares::jwt_middleware::JwtMiddleware;
use async_trait::async_trait;
use std::io::{Read, Write};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

#[path = "support/env_guard.rs"]
mod env_guard;
struct BrokenStore(u8);
#[async_trait]
impl SessionStore for BrokenStore {
    async fn is_valid(&self, _: Uuid, _: i64) -> Result<bool, SessionStoreError> {
        Err(match self.0 {
            0 => SessionStoreError::Sink("private-jwt-sink-detail".into()),
            1 => SessionStoreError::Validation("private-jwt-validation-detail".into()),
            _ => SessionStoreError::NotFound(Uuid::nil()),
        })
    }
    async fn record_session(&self, _: SessionRecord) -> Result<(), SessionStoreError> {
        unreachable!()
    }
    async fn revoke(&self, _: Uuid, _: i64) -> Result<(), SessionStoreError> {
        unreachable!()
    }
    async fn revoke_all_for_actor(&self, _: &str, _: i64) -> Result<usize, SessionStoreError> {
        unreachable!()
    }
    async fn prune_expired(&self, _: i64) -> Result<usize, SessionStoreError> {
        unreachable!()
    }
}

async fn request(addr: std::net::SocketAddr, token: String) -> String {
    tokio::task::spawn_blocking(move || {
        let mut socket = std::net::TcpStream::connect(addr).unwrap();
        socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        write!(socket, "GET /protected HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        response
    }).await.unwrap()
}

#[actix_web::test]
async fn jwt_http_security_contract() {
    let _env = env_guard::EnvGuard::new(&[
        ("JWT_SECRET", Some("security-test-secret-at-least-32-bytes")),
        ("JWT_GRANDFATHER_LEGACY", None),
    ]);
    let store = Arc::new(InMemorySessionStore::new());
    let (valid, jti) = create_token("alice").unwrap();
    store
        .record_session(SessionRecord {
            jti,
            actor_id: "alice".into(),
            created_at_us: 1,
            expires_at_us: i64::MAX,
            revoked_at_us: None,
        })
        .await
        .unwrap();
    let (revoked, revoked_id) = create_token("alice").unwrap();
    store
        .record_session(SessionRecord {
            jti: revoked_id,
            actor_id: "alice".into(),
            created_at_us: 1,
            expires_at_us: i64::MAX,
            revoked_at_us: None,
        })
        .await
        .unwrap();
    store.revoke(revoked_id, 2).await.unwrap();
    let legacy = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &serde_json::json!({"sub":"alice","exp":4102444800u64}),
        &jsonwebtoken::EncodingKey::from_secret(get_jwt_secret()),
    )
    .unwrap();
    let unknown = create_token("alice").unwrap().0;
    let sign = |exp: u64, secret: &[u8]| {
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &serde_json::json!({"sub":"alice", "exp":exp, "jti":jti}),
            &jsonwebtoken::EncodingKey::from_secret(secret),
        )
        .unwrap()
    };
    type Case = (
        &'static str,
        Option<Arc<dyn SessionStore>>,
        String,
        u16,
        bool,
    );
    let cases: Vec<Case> = vec![
        ("valid", Some(store.clone()), valid.clone(), 200, false),
        (
            "missing jti",
            Some(store.clone()),
            legacy.clone(),
            401,
            false,
        ),
        ("missing store", None, valid.clone(), 503, false),
        ("revoked", Some(store.clone()), revoked, 401, false),
        ("unknown", Some(store.clone()), unknown, 401, false),
        (
            "malformed",
            Some(store.clone()),
            "invalid".into(),
            401,
            false,
        ),
        (
            "wrong signing key",
            Some(store.clone()),
            sign(4102444800, b"wrong-key-at-least-thirty-two-bytes"),
            401,
            false,
        ),
        (
            "expired signed JWT",
            Some(store.clone()),
            sign(1, get_jwt_secret()),
            401,
            false,
        ),
        (
            "store failure",
            Some(Arc::new(BrokenStore(0))),
            valid.clone(),
            503,
            false,
        ),
        (
            "store validation failure",
            Some(Arc::new(BrokenStore(1))),
            valid.clone(),
            503,
            false,
        ),
        (
            "store missing record error",
            Some(Arc::new(BrokenStore(2))),
            valid,
            503,
            false,
        ),
        (
            "legacy opt-in wired",
            Some(store),
            legacy.clone(),
            200,
            true,
        ),
        ("legacy opt-in missing store", None, legacy, 503, true),
    ];
    let mut failures = Vec::new();
    for (name, store, token, expected, legacy_enabled) in cases {
        let _case_env = env_guard::EnvGuard::new(&[(
            "JWT_GRANDFATHER_LEGACY",
            if legacy_enabled { Some("1") } else { None },
        )]);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = HttpServer::new(move || {
            let mut app = App::new();
            if let Some(store) = store.clone() {
                app = app.app_data(web::Data::from(store));
            }
            app.wrap_fn({
                let observed = observed.clone();
                move |req, service| {
                    observed.fetch_add(1, Ordering::SeqCst);
                    service.call(req)
                }
            })
            .wrap(JwtMiddleware)
            .route(
                "/protected",
                web::get().to(|req: HttpRequest| async move {
                    assert_eq!(
                        req.extensions().get::<String>().map(String::as_str),
                        Some("alice")
                    );
                    HttpResponse::Ok().body("protected-data")
                }),
            )
        })
        .workers(1)
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let response = request(addr, token).await;
        handle.stop(true).await;
        assert!(
            !response.contains("private-jwt-"),
            "sink error must not leak over HTTP"
        );
        let status = response
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse::<u16>()
            .unwrap();
        if status != expected
            || calls.load(Ordering::SeqCst) != usize::from(expected == 200)
            || (expected != 200 && response.contains("protected-data"))
        {
            failures.push(format!(
                "{name}: expected {expected}, got {status}; handler calls={}",
                calls.load(Ordering::SeqCst)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

use arc_core::access_log::{
    AccessLogError, AccessLogger, AccessedResource, Identity, PurposeOfUse, Sensitivity,
};
use arc_web::helpers::access_log::{record_read, Sensitive};
struct TestLogger {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    stall: bool,
}
#[async_trait]
impl AccessLogger for TestLogger {
    async fn log_access(
        &self,
        _: Identity,
        _: AccessedResource,
        _: PurposeOfUse,
        _: Option<Uuid>,
    ) -> Result<(), AccessLogError> {
        self.entered.notify_one();
        if self.stall {
            self.release.notified().await;
        }
        Err(AccessLogError::Sink("injected private sink error".into()))
    }
}

#[actix_web::test]
async fn audit_http_failures_and_stalled_sink_gap() {
    for (sensitivity, expected, stall) in [
        (Sensitivity::Phi, 503, false),
        (Sensitivity::Pci, 503, false),
        (Sensitivity::Pii, 200, false),
        (Sensitivity::Phi, 503, true),
    ] {
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let logger: Arc<dyn AccessLogger> = Arc::new(TestLogger {
            entered: entered.clone(),
            release: release.clone(),
            stall,
        });
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = HttpServer::new(move || {
            App::new().app_data(web::Data::from(logger.clone())).route(
                "/protected",
                web::get().to(
                    move |req: HttpRequest, logger: web::Data<dyn AccessLogger>| async move {
                        record_read(
                            logger.get_ref(),
                            &req,
                            "alice",
                            AccessedResource::new("record", "1", sensitivity),
                            PurposeOfUse::UserInitiated,
                            Sensitive::new("protected-data", sensitivity),
                        )
                        .await
                    },
                ),
            )
        })
        .workers(1)
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let mut pending = Box::pin(request(addr, String::new()));
        if stall {
            // Observation window only: no production deadline is chosen by this test.
            tokio::select! { _ = entered.notified() => {}, response = &mut pending => panic!("sink was bypassed: {response}") }
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(100), &mut pending)
                    .await
                    .is_err(),
                "stalled sink must not leak PHI"
            );
            release.notify_one();
        }
        let response = pending.await;
        handle.stop(true).await;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected}")),
            "{response}"
        );
        assert_eq!(response.contains("protected-data"), expected == 200);
        assert!(!response.contains("injected private sink error"));
    }
}
