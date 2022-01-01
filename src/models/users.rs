use std::collections::HashMap;
use anyhow::anyhow;
use scylla::IntoTypedRows;
use poem_openapi::Object;

use crate::db::Session;
use crate::utils::JsSafeBigInt;


#[derive(Object)]
pub struct User {
    pub id: JsSafeBigInt,
    #[oai(skip)]
    pub access_servers: HashMap<i64, bool>,
    pub avatar: Option<String>,
    pub updated_on: i64,
    pub username: String,
}

/// Gets a user_id from the given access token if it's valid otherwise return None.
pub async fn get_user_id_from_token(sess: &Session, token: &str) -> anyhow::Result<Option<i64>> {
    let result = sess.query_prepared(
        "SELECT user_id FROM access_tokens WHERE access_token = ?;",
        (token.to_string(),)
    ).await?;

    let user_id = match result.rows {
        None => None,
        Some(rows) => {
            if let Some(row) = rows.into_typed::<(i64,)>().next() {
                Some(row?.0)
            } else {
                None
            }
        }
    };

    Ok(user_id)
}

/// Gets a full user object from the given access token.
pub async fn get_user_from_token(sess: &Session, token: &str) -> anyhow::Result<Option<User>> {
    let user_id = match get_user_id_from_token(sess, token).await? {
        None => return Ok(None),
        Some(user_id) => user_id,
    };

    get_user_from_id(sess, user_id).await
}

pub async fn get_user_from_id(sess: &Session, user_id: i64) -> anyhow::Result<Option<User>> {
    let result = sess.query_prepared(
        "SELECT id, access_servers, avatar, updated_on, username FROM users WHERE id = ?;",
        (user_id,)
    ).await?;

    let rows = result.rows
        .ok_or_else(|| anyhow!("expected returned rows"))?;

    type UserInfo = (JsSafeBigInt, HashMap<i64, bool>, Option<String>, chrono::Duration, String);

    let res = match rows.into_typed::<UserInfo>().next() {
        None => None,
        Some(v) => {
            let v = v?;
            Some(User {
                id: v.0,
                access_servers: v.1,
                avatar: v.2,
                updated_on: v.3.num_milliseconds(),
                username: v.4
            })
        },
    };

    Ok(res)
}