use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{AppError, AppState};

use chat_core::{Chat, ChatType};

#[derive(Debug, Serialize, Deserialize, Clone, Default, ToSchema)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, ToSchema)]
pub struct UpdateChat {
    pub name: Option<String>,
    pub members: Option<Vec<i64>>,
    pub public: Option<bool>,
}

#[allow(dead_code)]
impl AppState {
    pub async fn create_chat(
        &self,
        input: CreateChat,
        user_id: u64,
        ws_id: u64,
    ) -> Result<Chat, AppError> {
        let len = input.members.len();
        if len < 2 {
            return Err(AppError::CreateChatError(
                "A chat must have at least two members".to_string(),
            ));
        }
        // if user id is not in members, reject
        if !input.members.contains(&(user_id as i64)) {
            return Err(AppError::CreateChatError(
                "You must be a member of the chat".to_string(),
            ));
        }
        if let Some(name) = &input.name {
            if name.len() < 3 {
                return Err(AppError::CreateChatError(
                    "Chat name must be at least 3 characters".to_string(),
                ));
            }
        }
        if len > 8 && input.name.is_none() {
            return Err(AppError::CreateChatError(
                "A group chat with more than 8 members must have a name".to_string(),
            ));
        }
        let users = self.fetch_chat_user_by_ids(&input.members).await?;
        if users.len() != len {
            return Err(AppError::CreateChatError(
                "One or more members do not exist".to_string(),
            ));
        }
        let chat_type = match (&input.name, len) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };
        let chat = sqlx::query_as(
            r#"insert into chats (ws_id, name, type, members) values ($1, $2, $3, $4) returning id, ws_id, name, type, members, created_at"#,
        )
        .bind(ws_id as i64)
        .bind(&input.name)
        .bind(chat_type)
        .bind(&input.members)
        .fetch_one(&self.pool)
        .await?;
        Ok(chat)
    }

    pub async fn fetch_chats(&self, user_id: u64, ws_id: u64) -> Result<Vec<Chat>, AppError> {
        let chats = sqlx::query_as(
            r#"select id, ws_id, name, type, members, created_at from chats where ws_id = $1 AND $2 = ANY(members)"#,
        )
        .bind(ws_id as i64)
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(chats)
    }
    pub async fn delete_chat_by_id(&self, id: u64) -> Result<Option<Chat>, AppError> {
        let chat = sqlx::query_as(
            r#"delete from chats where id = $1 returning id, ws_id, name, type, members, created_at"#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;
        Ok(chat)
    }
    pub async fn update_chat_by_id(
        &self,
        input: UpdateChat,
        id: u64,
    ) -> Result<Option<Chat>, AppError> {
        let mut old_chat = match self.get_chat_by_id(id).await? {
            Some(chat) => chat,
            None => return Err(AppError::ChatDoesNotExist),
        };
        if let Some(name) = input.name {
            old_chat.name = Some(name);
        }

        if let Some(members) = input.members {
            old_chat.members = members;
        }

        if let Some(public) = input.public {
            old_chat.r#type = if public {
                ChatType::PublicChannel
            } else {
                ChatType::PrivateChannel
            };
        }
        let chat = sqlx::query_as(
            r#"update chats set name = $1, type = $2, members = $3 where id = $4 returning id, ws_id, name, type, members, created_at"#,
        )
        .bind(&old_chat.name)
        .bind(old_chat.r#type)
        .bind(&old_chat.members)
        .bind(id as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(chat))
    }
    pub async fn get_chat_by_id(&self, id: u64) -> Result<Option<Chat>, AppError> {
        let chat = sqlx::query_as(
            r#"select id, ws_id, name, type, members, created_at from chats where id = $1"#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;
        Ok(chat)
    }
    pub async fn is_chat_member(&self, chat_id: u64, user_id: u64) -> Result<bool, AppError> {
        let is_member = sqlx::query(r#"select 1 from chats where id = $1 and $2 = ANY(members)"#)
            .bind(chat_id as i64)
            .bind(user_id as i64)
            .fetch_optional(&self.pool)
            .await?;
        Ok(is_member.is_some())
    }
}

#[cfg(test)]
impl CreateChat {
    pub fn new(name: Option<String>, members: &[i64], public: bool) -> Self {
        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
impl UpdateChat {
    pub fn new(name: Option<String>, members: Option<Vec<i64>>, public: Option<bool>) -> Self {
        Self {
            name,
            members,
            public,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateChat::new(None, &[1, 2], false);
        let chat = state
            .create_chat(input, 1, 1)
            .await
            .expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);
        Ok(())
    }

    #[tokio::test]
    async fn create_public_named_chat_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateChat::new(Some("general123".to_string()), &[1, 2, 3], true);
        let chat = state
            .create_chat(input, 1, 1)
            .await
            .expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 3);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        Ok(())
    }

    #[tokio::test]
    async fn delete_chat_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateChat::new(Some("aaa".to_string()), &[1, 2, 3], true);
        let chat = state
            .create_chat(input, 1, 1)
            .await
            .expect("create chat failed");

        let chat = state
            .delete_chat_by_id(chat.id as u64)
            .await
            .expect("delete chat failed")
            .unwrap();
        assert_eq!(chat.name, Some("aaa".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn update_chat_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateChat::new(Some("aaa".to_string()), &[1, 2, 3], true);
        let chat = state
            .create_chat(input, 1, 1)
            .await
            .expect("create chat failed");
        let input = UpdateChat::new(
            Some("general chat".to_string()),
            Some(vec![5, 6, 7, 8]),
            Some(false),
        );
        let chat = state
            .update_chat_by_id(input, chat.id as u64)
            .await
            .expect("update chat failed")
            .unwrap();

        assert_eq!(chat.name, Some("general chat".to_string()));
        assert_eq!(chat.members.len(), 4);
        assert_eq!(chat.r#type, ChatType::PrivateChannel);
        Ok(())
    }

    #[tokio::test]
    async fn chat_get_by_id_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let chat = state
            .get_chat_by_id(1)
            .await
            .expect("get chat by id failed")
            .unwrap();
        assert_eq!(chat.id, 1);
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 5);
        Ok(())
    }
    #[tokio::test]
    async fn chat_fetch_all_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let chats = state
            .fetch_chats(1, 1)
            .await
            .expect("fetch all chats failed");
        assert_eq!(chats.len(), 4);
        Ok(())
    }

    #[tokio::test]
    async fn chat_is_member_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let is_member = state
            .is_chat_member(1, 1)
            .await
            .expect("is chat member failed");
        assert!(is_member);

        // user 6 is not a member of chat 1
        let is_member = state
            .is_chat_member(1, 6)
            .await
            .expect("is chat member failed");
        assert!(!is_member);

        // chat 10 does not exist
        let is_member = state
            .is_chat_member(10, 1)
            .await
            .expect("is chat member failed");
        assert!(!is_member);

        // user 4 is not a member of chat 2
        let is_member = state
            .is_chat_member(2, 4)
            .await
            .expect("is chat member failed");
        assert!(!is_member);
        Ok(())
    }
}
