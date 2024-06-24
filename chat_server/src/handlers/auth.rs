use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{
    models::{CreateUser, SigninUser},
    AppError, AppState, ErrorOutput,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthOutput {
    token: String,
}

pub(crate) async fn signup_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.create_user(&input).await?;
    let token = state.ek.sign(user)?;
    /* let mut header = HeaderMap::new();
    header.insert("X-Token", HeaderValue::from_str(&token)?);
    Ok((StatusCode::CREATED, header)) */

    let body = Json(AuthOutput { token });
    Ok((StatusCode::CREATED, body))
}

pub(crate) async fn signin_handler(
    State(state): State<AppState>,
    Json(input): Json<SigninUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.verify_user(&input).await?;
    match user {
        Some(user) => {
            let token = state.ek.sign(user)?;
            Ok((StatusCode::OK, Json(AuthOutput { token })).into_response())
        }
        None => Ok((
            StatusCode::FORBIDDEN,
            Json(ErrorOutput::new("Invalid email or password")),
        )
            .into_response()),
    }
}

#[cfg(test)]
mod tests {
    use http_body_util::BodyExt;

    use super::*;

    #[tokio::test]
    async fn signup_duplicate_should_409() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateUser::new("none", "zzq21", "zzq21@zzq.com", "zzq");
        signup_handler(State(state.clone()), Json(input.clone()))
            .await?
            .into_response();
        let ret = signup_handler(State(state.clone()), Json(input.clone()))
            .await
            .into_response();
        assert_eq!(ret.status(), StatusCode::CONFLICT);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "email already exists: zzq21@zzq.com");
        Ok(())
    }

    #[tokio::test]
    async fn signup_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateUser::new("none", "zzq21", "zzq21@zzq.com", "zzq");
        let ret = signup_handler(State(state), Json(input))
            .await?
            .into_response();

        assert_eq!(ret.status(), StatusCode::CREATED);

        let body = ret.into_body().collect().await?.to_bytes();
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");
        Ok(())
    }

    #[tokio::test]
    async fn signin_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let user = CreateUser::new("none", "zzq21", "zzq21@zzq.com", "zzq");
        state.create_user(&user).await?;
        let input = SigninUser::new("zzq21@zzq.com", "zzq");

        let ret = signin_handler(State(state), Json(input))
            .await?
            .into_response();

        assert_eq!(ret.status(), StatusCode::OK);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");
        Ok(())
    }
    #[tokio::test]
    async fn signin_with_non_exist_user_should_403() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = SigninUser::new("zzq21@zzq.com", "zzq");

        let ret = signin_handler(State(state), Json(input))
            .await?
            .into_response();

        assert_eq!(ret.status(), StatusCode::FORBIDDEN);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret.error, "Invalid email or password");
        Ok(())
    }
}
