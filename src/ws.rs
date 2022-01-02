use futures_util::SinkExt;
use poem::{handler, web::{
    websocket::{Message, WebSocket},
    Data, Query,
}, IntoResponse, Response, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::db::Session;



#[derive(Serialize, Debug, Clone)]
pub struct Event {
    #[serde(rename = "type")]
    pub type_: String,

    pub data: Value,
}

#[derive(Deserialize)]
pub struct QueryParams {
    room_id: Uuid,
    token: String,
}

#[handler]
pub async fn gateway(
    Query(QueryParams { room_id, token }): Query<QueryParams>,
    ws: WebSocket,
    session: Data<&Session>,
    emitter: Data<&crate::emitter::EmitterManager>,
) -> Result<Response> {
    let user = match crate::models::get_user_from_token(&session, &token).await? {
        None => return Ok((StatusCode::UNAUTHORIZED, "unauthorized user").into_response()),
        Some(user) => user,
    };

    let room = match crate::models::get_room_by_id(&session, room_id).await? {
        None => return Ok((StatusCode::BAD_REQUEST, "no room exists").into_response()),
        Some(room) => room,
    };

    if !room.active {
        return Ok((StatusCode::BAD_REQUEST, "room closed").into_response())
    }

    let has_guild_access = if let Some(guild_id) = room.guild_id.as_ref() {
      user.access_servers.contains_key(&*guild_id)
    } else {
        false
    };

    if (!room.is_public)                // The room is not public
        & (!room.invite_only)           // The room is not invite only
        & (room.owner_id != user.id)    // They are not the owner of the room
        & (!has_guild_access)           // They don't have access via guilds.
    {
        return Ok((StatusCode::FORBIDDEN, "no access").into_response())
    }

    emitter.register_room(room_id.clone());
    let mut receiver = emitter.get_subscriber(&room_id);

    let resp = ws.on_upgrade(move |mut socket| async move {
        let mut lag_count = 0;
        loop {
            while let Ok(event) = receiver.try_recv() {
                let msg = Message::Binary(serde_json::to_vec(&event).unwrap());
                if socket.feed(msg).await.is_err() {
                    break;
                };
            }

            match receiver.recv().await {
                Ok(event) => {
                    let msg = Message::Binary(serde_json::to_vec(&event).unwrap());
                    if socket.feed(msg).await.is_err() {
                        break;
                    };
                },
                Err(RecvError::Lagged(n)) => {
                    warn!("User {} connection is lagging behind, {} events skipped.", &user.id, n);

                    lag_count += 1;

                    if lag_count > 3 {
                        warn!("Aborting user connection {} due to too many lagged events.", &user.id);
                        let _ = socket.send(
                            Message::Binary(serde_json::to_vec(&Event {
                                type_: "CLOSE".to_string(),
                                data: Value::Null,
                            }).unwrap())
                        ).await;
                        break;
                    }

                    continue;
                }
                Err(_) => break,
            };

            if let Err(e) = socket.flush().await {
                error!("Aborting connection due to flush error {}", e);
                break;
            };
        }

        let _ = socket.close().await;
    }).into_response();

    Ok(resp)
}
