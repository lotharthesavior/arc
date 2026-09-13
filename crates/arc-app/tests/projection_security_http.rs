//! Actual HTTP projection boundary and duplicate delivery; forged-event acceptance is an OPEN GAP.
#[path = "support/env_guard.rs"]
mod env_guard;
use actix_web::{web, App, HttpServer};
use arc::domain::user::projector::{UserProjector, USERS_VIEW};
use arc::http::controllers::internal_projection_controller::handle_user_projection;
use arc_core::{
    audit::AuditMetadata,
    event::{Event, NewEvent},
    event_store::InMemoryEventStore,
    projection::ProjectionEngine,
    read_model_store::{InMemoryReadModelStore, ReadModelStore},
};
use std::{
    io::{Read, Write},
    sync::Arc,
};

#[actix_web::test]
async fn projection_credentials_duplicates_and_forgery_gap() {
    let _env = env_guard::EnvGuard::new(&[("INTERNAL_PROJECTION_TOKEN", Some("fixture-token"))]);
    let models: Arc<dyn ReadModelStore> = Arc::new(InMemoryReadModelStore::new());
    // Empty authoritative store: the submitted event cannot possibly be persisted.
    let mut engine = ProjectionEngine::new(Box::new(InMemoryEventStore::new()));
    engine.register_projector(Box::new(UserProjector::new()), models.clone(), USERS_VIEW);
    let engine = web::Data::new(engine);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(engine.clone())
            .service(handle_user_projection)
    })
    .workers(1)
    .listen(listener)
    .unwrap()
    .run();
    let handle = server.handle();
    actix_web::rt::spawn(server);
    let event = Event::new(NewEvent { aggregate_type: "User", aggregate_id: "forged", sequence: 1, event_type: "UserRegistered", payload: serde_json::json!({"name":"Forged", "email":"forged@example.test", "password_hash":"fake"}) }).with_audit(AuditMetadata::test_default());
    for credential in ["", "Bearer wrong", "Basic fixture-token"] {
        let response = post(addr, credential, &event).await;
        assert!(response.starts_with("HTTP/1.1 401"), "{response}");
        assert!(models.get(USERS_VIEW, "forged").await.unwrap().is_none());
    }
    let first = post(addr, "Bearer fixture-token", &event).await;
    let first_row = models.get(USERS_VIEW, "forged").await.unwrap();
    let second = post(addr, "Bearer fixture-token", &event).await;
    let second_row = models.get(USERS_VIEW, "forged").await.unwrap();
    handle.stop(true).await;
    if std::env::var("ARC_SECURITY_CONTRACTS").as_deref() == Ok("1") {
        assert!(
            first.starts_with("HTTP/1.1 4") && first_row.is_none(),
            "OPEN GAP: forged event accepted with valid bearer: {first}"
        );
    } else {
        assert!(first.starts_with("HTTP/1.1 204"), "{first}");
        assert!(second.starts_with("HTTP/1.1 204"), "{second}");
        assert_eq!(
            first_row, second_row,
            "duplicate delivery must not change projection state"
        );
        assert_eq!(
            second_row.unwrap()["email"],
            "forged@example.test",
            "OPEN GAP reproduction: bearer holder can inject unpersisted event"
        );
    }
}
async fn post(addr: std::net::SocketAddr, authorization: &str, event: &Event) -> String {
    let body = serde_json::to_string(event).unwrap();
    let authorization = authorization.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut socket = std::net::TcpStream::connect(addr).unwrap();
        socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        write!(socket,"POST /users/handle HTTP/1.1\r\nHost: localhost\r\nAuthorization: {authorization}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        let mut response = String::new(); socket.read_to_string(&mut response).unwrap(); response
    }).await.unwrap()
}
