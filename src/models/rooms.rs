use anyhow::{anyhow, Result};
use scylla::{IntoTypedRows, FromRow};
use uuid::Uuid;
use poem_openapi::Object;

use crate::db::Session;
use crate::utils::JsSafeBigInt;

#[derive(Object, FromRow)]
pub struct Room {
    pub id: Uuid,
    pub owner_id: JsSafeBigInt,
    pub active: bool,
    pub active_playlist: Option<Uuid>,
    pub banner: Option<String>,
    pub guild_id: Option<JsSafeBigInt>,
    pub invite_only: bool,
    pub is_public: bool,
    pub playing_now: Option<Uuid>,
    pub title: String,
    pub topic: Option<String>,
}

pub async fn get_room_by_id(sess: &Session, room_id: Uuid) -> Result<Option<Room>> {
    let result = sess.query_prepared(
        r#"
        SELECT
            id,
            owner_id,
            active,
            active_playlist,
            banner,
            guild_id,
            invite_only,
            is_public,
            playing_now,
            title,
            topic
        FROM rooms
        WHERE id = ?;
        "#,
        (room_id,)
    ).await?;

    let rows = result.rows
        .ok_or_else(|| anyhow!("expected returned rows"))?;

    let room = match rows.into_typed::<Room>().next() {
        None => return Ok(None),
        Some(v) => v?,
    };

    Ok(Some(room))
}