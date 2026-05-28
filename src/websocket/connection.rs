use actix::prelude::*;
use actix_session::Session;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::websocket::server::{Connect, Disconnect, Subscribe, Unsubscribe, WsMessage, WsServer};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// WebSocket connection actor
pub struct WsConnection {
    /// Unique connection ID
    id: Uuid,
    /// User ID if authenticated (optional)
    user_id: Option<i32>,
    /// Client heartbeat tracking
    heartbeat: Instant,
    /// Address of the broadcast server
    server_addr: Addr<WsServer>,
}

impl WsConnection {
    pub fn new(user_id: Option<i32>, server_addr: Addr<WsServer>) -> Self {
        WsConnection {
            id: Uuid::new_v4(),
            user_id,
            heartbeat: Instant::now(),
            server_addr,
        }
    }

    /// Start heartbeat process
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check client heartbeat
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                warn!(connection_id = %act.id, "WebSocket client heartbeat timeout, disconnecting");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    /// Handle text messages from client
    fn handle_text_message(&mut self, text: &str, ctx: &mut ws::WebsocketContext<Self>) {
        // Parse JSON commands from client
        if let Ok(cmd) = serde_json::from_str::<ClientCommand>(text) {
            match cmd {
                ClientCommand::Subscribe { room } => {
                    self.server_addr.do_send(Subscribe { id: self.id, room });
                }
                ClientCommand::Unsubscribe { room } => {
                    self.server_addr.do_send(Unsubscribe { id: self.id, room });
                }
                ClientCommand::Ping => {
                    ctx.text(r#"{"type":"pong"}"#);
                }
            }
        }
    }
}

/// Client commands that can be sent via WebSocket
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientCommand {
    Subscribe { room: String },
    Unsubscribe { room: String },
    Ping,
}

impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Start heartbeat
        self.start_heartbeat(ctx);

        // Register with server
        let addr = ctx.address();
        self.server_addr.do_send(Connect {
            id: self.id,
            user_id: self.user_id,
            addr: addr.recipient(),
        });

        info!(connection_id = %self.id, user_id = ?self.user_id, "WebSocket connection actor started");
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // Notify server of disconnect
        self.server_addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages from WebSocket server
impl Handler<WsMessage> for WsConnection {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// Handle WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                self.heartbeat = Instant::now();
                self.handle_text_message(&text, ctx);
            }
            Ok(ws::Message::Binary(bin)) => {
                // Binary messages not used for Turbo Streams, but handle gracefully
                ctx.binary(bin);
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

/// WebSocket handler endpoint
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    session: Session,
    server: web::Data<Addr<WsServer>>,
) -> Result<HttpResponse, Error> {
    // Get user_id from session if authenticated (optional auth)
    let user_id = session.get::<i32>("user_id").ok().flatten();

    let ws_connection = WsConnection::new(user_id, server.get_ref().clone());

    ws::start(ws_connection, &req, stream)
}
