mod config;
mod error;
mod notify;
mod sse;
use std::{ops::Deref, sync::Arc};

use axum::{
    middleware::from_fn_with_state,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use chat_core::{verify_token, DecodingKey, TokenVerify, User};
use dashmap::DashMap;
use sse::sse_handler;
use tokio::sync::broadcast;

pub use config::AppConfig;
pub use error::AppError;
pub use notify::{setup_pg_listener, AppEvent};

const INDEX_HTML: &str = include_str!("../index.html");
pub type UserMap = Arc<DashMap<u64, broadcast::Sender<Arc<AppEvent>>>>;

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);
pub struct AppStateInner {
    pub config: AppConfig,
    users: UserMap,
    dk: DecodingKey,
}

pub fn get_router() -> (Router, AppState) {
    let config = AppConfig::load().expect("Failed to load config");
    let state = AppState::new(config);
    let app = Router::new()
        .route("/events", get(sse_handler))
        .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
        .route("/", get(index_handler))
        .with_state(state.clone());
    (app, state)
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

impl Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TokenVerify for AppState {
    type Error = AppError;
    fn verify(&self, token: &str) -> Result<User, Self::Error> {
        Ok(self.dk.verify(token)?)
    }
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let dk = DecodingKey::load(&config.auth.pk).expect("Failed to load pk");
        let users = Arc::new(DashMap::new());
        Self(Arc::new(AppStateInner { config, dk, users }))
    }
}
