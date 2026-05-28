use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use tracing::info;
use uuid::Uuid;

/// Message to connect a new client
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub id: Uuid,
    pub user_id: Option<i32>,
    pub addr: Recipient<WsMessage>,
}

/// Message to disconnect a client
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid,
}

/// Message to subscribe to a room/channel
#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe {
    pub id: Uuid,
    pub room: String,
}

/// Message to unsubscribe from a room/channel
#[derive(Message)]
#[rtype(result = "()")]
pub struct Unsubscribe {
    pub id: Uuid,
    pub room: String,
}

/// Message to broadcast to a specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastToRoom {
    pub room: String,
    pub message: String,
    pub skip_id: Option<Uuid>,
}

/// Message to broadcast to a specific user
#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastToUser {
    pub user_id: i32,
    pub message: String,
}

/// Message to broadcast to all connected clients
#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastAll {
    pub message: String,
    pub skip_id: Option<Uuid>,
}

/// WebSocket message to send to a client
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

/// Connection info stored in the server
struct ConnectionInfo {
    addr: Recipient<WsMessage>,
    user_id: Option<i32>,
    rooms: HashSet<String>,
}

/// WebSocket broadcast server
/// Maintains registry of all active connections and handles message broadcasting
pub struct WsServer {
    /// Map of connection ID to connection info
    connections: HashMap<Uuid, ConnectionInfo>,
    /// Map of room name to set of connection IDs
    rooms: HashMap<String, HashSet<Uuid>>,
    /// Map of user ID to set of connection IDs (for user-specific broadcasts)
    user_connections: HashMap<i32, HashSet<Uuid>>,
}

impl WsServer {
    pub fn new() -> Self {
        WsServer {
            connections: HashMap::new(),
            rooms: HashMap::new(),
            user_connections: HashMap::new(),
        }
    }

    /// Send a message to a specific connection
    fn send_message(&self, id: &Uuid, message: &str) {
        if let Some(conn) = self.connections.get(id) {
            conn.addr.do_send(WsMessage(message.to_string()));
        }
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for WsServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Actor for WsServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        // Add connection to registry
        self.connections.insert(
            msg.id,
            ConnectionInfo {
                addr: msg.addr,
                user_id: msg.user_id,
                rooms: HashSet::new(),
            },
        );

        // Track user connections if authenticated
        if let Some(user_id) = msg.user_id {
            self.user_connections
                .entry(user_id)
                .or_default()
                .insert(msg.id);
        }

        info!(connection_id = %msg.id, user_id = ?msg.user_id, "WebSocket connection established");
    }
}

impl Handler<Disconnect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        if let Some(conn) = self.connections.remove(&msg.id) {
            // Remove from all rooms
            for room in &conn.rooms {
                if let Some(room_members) = self.rooms.get_mut(room) {
                    room_members.remove(&msg.id);
                    if room_members.is_empty() {
                        self.rooms.remove(room);
                    }
                }
            }

            // Remove from user connections
            if let Some(user_id) = conn.user_id {
                if let Some(user_conns) = self.user_connections.get_mut(&user_id) {
                    user_conns.remove(&msg.id);
                    if user_conns.is_empty() {
                        self.user_connections.remove(&user_id);
                    }
                }
            }

            info!(connection_id = %msg.id, "WebSocket connection closed");
        }
    }
}

impl Handler<Subscribe> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _: &mut Context<Self>) {
        // Add connection to room
        self.rooms
            .entry(msg.room.clone())
            .or_default()
            .insert(msg.id);

        // Track room in connection
        if let Some(conn) = self.connections.get_mut(&msg.id) {
            conn.rooms.insert(msg.room.clone());
        }

        // Debug: Connection subscribed to room
    }
}

impl Handler<Unsubscribe> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _: &mut Context<Self>) {
        // Remove connection from room
        if let Some(room_members) = self.rooms.get_mut(&msg.room) {
            room_members.remove(&msg.id);
            if room_members.is_empty() {
                self.rooms.remove(&msg.room);
            }
        }

        // Remove room from connection tracking
        if let Some(conn) = self.connections.get_mut(&msg.id) {
            conn.rooms.remove(&msg.room);
        }

        // Debug: Connection unsubscribed from room
    }
}

impl Handler<BroadcastToRoom> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastToRoom, _: &mut Context<Self>) {
        if let Some(members) = self.rooms.get(&msg.room) {
            for id in members {
                // Skip sender if specified
                if let Some(skip_id) = msg.skip_id {
                    if *id == skip_id {
                        continue;
                    }
                }
                self.send_message(id, &msg.message);
            }
        }
    }
}

impl Handler<BroadcastToUser> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastToUser, _: &mut Context<Self>) {
        if let Some(user_conns) = self.user_connections.get(&msg.user_id) {
            for id in user_conns {
                self.send_message(id, &msg.message);
            }
        }
    }
}

impl Handler<BroadcastAll> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastAll, _: &mut Context<Self>) {
        for id in self.connections.keys() {
            // Skip sender if specified
            if let Some(skip_id) = msg.skip_id {
                if *id == skip_id {
                    continue;
                }
            }
            self.send_message(id, &msg.message);
        }
    }
}
