use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::broadcast;
use uuid::Uuid;
use anyhow::{anyhow, Result};

use crate::ws::Event;


pub struct RoomWrapper {
    pub started: i64,
    pub messenger: broadcast::Sender<Event>,
}


#[derive(Clone)]
pub struct EmitterManager {
    rooms: Arc<DashMap<Uuid, RoomWrapper>>,
}

impl EmitterManager {
    pub fn new() -> Self {
        Self {
            rooms: Default::default(),
        }
    }

    pub fn register_room(&self, room_id: Uuid) {
        if self.rooms.contains_key(&room_id) {
            return;
        }

        let wrapped = RoomWrapper {
            started: chrono::Utc::now().timestamp(),
            messenger: broadcast::channel(32).0,
        };

        self.rooms.insert(room_id, wrapped);
    }

    pub fn get_subscriber(&self, room_id: &Uuid) -> broadcast::Receiver<Event> {
        let room = self.rooms
            .get(room_id)
            .unwrap();

        room.messenger.subscribe()
    }

    #[instrument(name = "room-event", skip(self), level = "info")]
    pub fn emit(&self, room_id: &Uuid, event: Event) -> Result<()> {
        if let Some(room) = self.rooms.get(room_id) {
            let amount = room.messenger.send(event)?;
            info!("Broadcasting event to room {} with {} active receivers", room_id, amount);
            Ok(())
        } else {
            Err(anyhow!("no room exists with id {}", room_id))
        }
    }
}