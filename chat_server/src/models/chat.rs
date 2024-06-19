use serde::{Deserialize, Serialize};

use crate::AppError;

use super::{Chat, ChatType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
}

#[allow(dead_code)]
impl Chat {
    pub async fn create(
        input: CreateChat,
        ws_id: u64,
        pool: &sqlx::PgPool,
    ) -> Result<Self, AppError> {
        let chat = sqlx::query_as(
            r#"insert into chats (ws_id, name, type, members) values ($1, $2, $3, $4) returning id, ws_id, name, type, members, created_at"#,
        )
        .bind(ws_id as i64)
        .bind(input.name)
        .bind(ChatType::Group)
        .bind(&input.members)
        .fetch_one(pool)
        .await?;
        Ok(chat)
    }
}
