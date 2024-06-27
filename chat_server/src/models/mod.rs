mod chat;
mod file;
mod messages;
mod user;
mod workspace;

pub use chat::*;
pub use messages::*;
use serde::{Deserialize, Serialize};
pub use user::{CreateUser, SigninUser};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatFile {
    pub ws_id: u64,
    pub ext: String,
    pub hash: String,
}
