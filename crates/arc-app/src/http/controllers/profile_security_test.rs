use crate::domain::user::projector::USERS_VIEW;
use actix_web::{web, App, HttpServer};
use arc_core::read_model_store::{InMemoryReadModelStore, ReadModelStore};
use std::{
    io::{Read, Write},
    sync::Arc,
};
// Restore process environment in the integration-test executable, including on assertion panic.
// Tests sharing a key must still be serialized; separate integration executables have separate envs.
pub struct ProfileEnvGuard(Vec<(String, Option<std::ffi::OsString>)>);
impl ProfileEnvGuard {
    pub fn new(values: &[(&str, Option<&str>)]) -> Self {
        let previous = values
            .iter()
            .map(|(key, _)| ((*key).to_owned(), std::env::var_os(key)))
            .collect();
        for (key, value) in values {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self(previous)
    }
}
impl Drop for ProfileEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn profile_http_is_bound_to_actor_and_filters_secrets() {
    use crate::http::controllers::api_controller::profile;
    use arc_core::access_log::{AccessLogger, RecordingAccessLogger};
    use arc_core::read_model_store::Upsert;
    use arc_core::session::{InMemorySessionStore, SessionRecord, SessionStore};
    use arc_web::{helpers::jwt::create_token, http::middlewares::jwt_middleware::JwtMiddleware};
    let _env = ProfileEnvGuard::new(&[(
        "JWT_SECRET",
        Some("profile-security-test-at-least-32-bytes"),
    )]);
    let models: Arc<dyn ReadModelStore> = Arc::new(InMemoryReadModelStore::new());
    for id in ["alice", "bob"] {
        models.upsert(Upsert::new(USERS_VIEW, id, serde_json::json!({"id":id,"name":id,"email":format!("{id}@example.test"),"password_hash":"NEVER-RETURN-THIS"}))).await.unwrap();
    }
    let sessions: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let (token, jti) = create_token("alice").unwrap();
    sessions
        .record_session(SessionRecord {
            jti,
            actor_id: "alice".into(),
            created_at_us: 1,
            expires_at_us: i64::MAX,
            revoked_at_us: None,
        })
        .await
        .unwrap();
    let recorder = RecordingAccessLogger::new();
    let logger: Arc<dyn AccessLogger> = Arc::new(recorder.clone());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(models.clone()))
            .app_data(web::Data::from(sessions.clone()))
            .app_data(web::Data::from(logger.clone()))
            .wrap(JwtMiddleware)
            .service(profile)
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run();
    let handle = server.handle();
    actix_web::rt::spawn(server);
    let response = tokio::task::spawn_blocking(move || {
        let mut socket = std::net::TcpStream::connect(addr).unwrap();
        socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        write!(socket,"GET /profile?id=bob&user_id=bob HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = String::new(); socket.read_to_string(&mut response).unwrap(); response
    }).await.unwrap();
    handle.stop(true).await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("alice@example.test"));
    assert!(!response.contains("bob"));
    assert!(!response.contains("NEVER-RETURN-THIS"));
    let entries = recorder.entries().await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].actor.actor_id, "alice");
    assert_eq!(entries[0].resource.identifier, "alice");
}
