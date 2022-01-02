use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::broadcast;
use uuid::Uuid;
use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use crate::ws::Event;

const KEEP_ALIVE_PING: u64 = 30;
const MAX_INTERVAL_MISSES: u64 = 2 * 10;  // 10 minutes of in-activity.

pub struct RoomWrapper {
    pub started: i64,
    pub messenger: broadcast::Sender<Event>,
    handle: JoinHandle<()>,
}

impl Drop for RoomWrapper {
    fn drop(&mut self) {
        self.handle.abort();
    }
}


#[derive(Clone)]
pub struct EmitterManager {
    rooms: Arc<DashMap<Uuid, RoomWrapper>>,
    shutdown_requests: Sender<Uuid>,
}

impl EmitterManager {
    pub fn start() -> Self {

        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let inst = Self {
            rooms: Default::default(),
            shutdown_requests: tx,
        };
        let manager = inst.clone();

        // housekeeping
        tokio::spawn(async move {
            while let Some(id) = rx.recv().await {
                info!("Housekeeping - Closing room {}", &id);
                manager.close_room(&id, false)
            }
        });

        inst
    }

    pub fn close_room(&self, room_id: &Uuid, warn_clients: bool) {
        if warn_clients {
            let _ = self.emit(room_id, Event { type_: "CLOSE".to_string(), data: Value::Null });
        }

        self.rooms.remove(room_id);
    }

    pub fn register_room(&self, room_id: Uuid) {
        if self.rooms.contains_key(&room_id) {
            return;
        }

        let housekeeper = self.shutdown_requests.clone();
        let sender = broadcast::channel(32).0;

        let emitter = sender.clone();
        let handle = tokio::spawn(async move {
            let id = room_id.clone();

            let mut interval = tokio::time::interval(Duration::from_secs(KEEP_ALIVE_PING));

            let mut intervals_elapsed: u64 = 0;

            loop {
                interval.tick().await;
                let connections_alive = emitter.send(Event {
                    type_: "".to_string(),
                    data: Default::default()
                }).is_ok();

                if connections_alive {
                    if intervals_elapsed != 0 {
                        info!(
                            "Room {} has became active again! Got to {} seconds idle.",
                            &id,
                            intervals_elapsed * KEEP_ALIVE_PING,
                        );
                    }

                    intervals_elapsed = 0;
                } else {
                    if intervals_elapsed == 0 {
                        info!(
                            "Room {} has became in-active! Begging timeout checks...",
                            &id,
                        );
                    }

                    intervals_elapsed += 1;
                }

                if intervals_elapsed >= MAX_INTERVAL_MISSES {
                    info!("A room timeout has expired emitting shutdown for room {}", &id);
                    let _ = housekeeper.send(id).await;
                }
            }
        });

        let wrapped = RoomWrapper {
            started: chrono::Utc::now().timestamp(),
            messenger: sender,
            handle
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