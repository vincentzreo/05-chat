use std::{collections::HashSet, sync::Arc};

use chat_core::{Chat, Message};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgListener;
use tokio_stream::StreamExt;
use tracing::{info, warn};

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum AppEvent {
    NewChat(Chat),
    AddToChat(Chat),
    RemoveFromChat(Chat),
    NewMessage(Message),
    ChatNameUpdated(ChatNameUpdated),
}

#[derive(Debug)]
struct Notification {
    user_ids: HashSet<u64>,
    event: Arc<AppEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatUpdated {
    op: String,
    old: Option<Chat>,
    new: Option<Chat>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageAdded {
    members: Vec<i64>,
    message: Message,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatNameUpdated {
    chat_id: i64,
    old_name: String,
    new_name: String,
    members: Vec<i64>,
}

pub async fn setup_pg_listener(state: AppState) -> anyhow::Result<()> {
    let mut listener = PgListener::connect(&state.config.server.db_url).await?;
    listener.listen("chat_updated").await?;
    listener.listen("chat_message_added").await?;
    listener.listen("chat_name_updated").await?;
    let mut stream = listener.into_stream();
    tokio::spawn(async move {
        while let Some(Ok(notif)) = stream.next().await {
            info!("Received notification: {:?}", notif);
            let notification = Notification::load(notif.channel(), notif.payload()).unwrap();
            info!("Notification: {:?}", notification);
            let users = &state.users;
            for user_id in notification.user_ids {
                if let Some(tx) = users.get(&user_id) {
                    info!("Sending notification to user {}", user_id);
                    if let Err(e) = tx.send(notification.event.clone()) {
                        warn!("Failed to send notification to user {}: {}", user_id, e);
                        users.remove(&user_id);
                    }
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    });
    Ok(())
}

impl Notification {
    fn load(r#type: &str, payload: &str) -> anyhow::Result<Self> {
        match r#type {
            "chat_name_updated" => {
                let payload = serde_json::from_str::<ChatNameUpdated>(payload)?;
                let user_ids = payload.members.iter().map(|v| *v as u64).collect();
                Ok(Self {
                    user_ids,
                    event: Arc::new(AppEvent::ChatNameUpdated(payload)),
                })
            }
            "chat_updated" => {
                let payload = serde_json::from_str::<ChatUpdated>(payload)?;
                let user_ids =
                    get_affected_chat_user_ids(payload.old.as_ref(), payload.new.as_ref());
                let event = match payload.op.as_str() {
                    "INSERT" => AppEvent::NewChat(payload.new.expect("new should exist")),
                    "UPDATE" => AppEvent::AddToChat(payload.new.expect("new should exist")),
                    "DELETE" => AppEvent::RemoveFromChat(payload.old.expect("old should exist")),
                    _ => return Err(anyhow::anyhow!("Unknown operation: {}", payload.op)),
                };
                Ok(Self {
                    user_ids,
                    event: Arc::new(event),
                })
            }
            "chat_message_added" => {
                let payload = serde_json::from_str::<ChatMessageAdded>(payload)?;
                let user_ids = payload.members.iter().map(|v| *v as u64).collect();
                Ok(Self {
                    user_ids,
                    event: Arc::new(AppEvent::NewMessage(payload.message)),
                })
            }
            _ => Err(anyhow::anyhow!("Unknown notification type: {}", r#type)),
        }
    }
}

fn get_affected_chat_user_ids(old: Option<&Chat>, new: Option<&Chat>) -> HashSet<u64> {
    match (old, new) {
        (Some(old), Some(new)) => {
            // diff old/new members, if identical, no need to notify, otherwise notify the union of both
            let old_user_ids: HashSet<_> = old.members.iter().map(|v| *v as u64).collect();
            let new_user_ids: HashSet<_> = new.members.iter().map(|v| *v as u64).collect();
            if old_user_ids == new_user_ids {
                HashSet::new()
            } else {
                old_user_ids.union(&new_user_ids).copied().collect()
            }
        }
        (Some(old), None) => old.members.iter().map(|v| *v as u64).collect(),
        (None, Some(new)) => new.members.iter().map(|v| *v as u64).collect(),
        (None, None) => HashSet::new(),
    }
}
