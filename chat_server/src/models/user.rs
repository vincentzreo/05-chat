use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{AppError, AppState};

use chat_core::{ChatUser, User};

/// Create a new user with email and password
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateUser {
    /// Full name of the user
    pub fullname: String,
    /// Email of the user
    pub email: String,
    /// Workspace name - if not exists, create one
    pub workspace: String,
    /// Password of the user
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SigninUser {
    pub email: String,
    pub password: String,
}

#[allow(dead_code)]
impl AppState {
    /// Find a user by email
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as(
            "select id, ws_id, fullname, email, created_at from users where email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }
    /// Find user by id
    pub async fn find_user_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as(
            "select id, ws_id, fullname, email, created_at from users where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }
    /// Create a new user
    pub async fn create_user(&self, input: &CreateUser) -> Result<User, AppError> {
        // check if the email is already in use
        let user = self.find_user_by_email(&input.email).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }
        // check if workspace exists, if not create one
        let ws = match self.find_workspace_by_name(&input.workspace).await? {
            Some(ws) => ws,
            None => self.create_workspace(&input.workspace, 0).await?,
        };

        let password_hash = hash_password(&input.password)?;
        let mut user: User = sqlx::query_as(
            "insert into users (ws_id, email, fullname, password_hash) values ($1, $2, $3, $4) returning id, ws_id, fullname, email, created_at",
        )
        .bind(ws.id)
        .bind(&input.email)
        .bind(&input.fullname)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;
        user.ws_name = ws.name.clone();
        if ws.owner_id == 0 {
            self.update_workspace_owner(ws.id as _, user.id as _)
                .await?;
        }
        Ok(user)
    }
    // /// add user to workspace
    // pub async fn add_to_workspace(&self, ws_id: i64, pool: &PgPool) -> Result<User, AppError> {
    //     let user = sqlx::query_as("update users set ws_id = $1 where id = $2 and ws_id = 0 RETURNING id, ws_id, fullname, email, created_at")
    //         .bind(ws_id)
    //         .bind(self.id)
    //         .fetch_one(pool)
    //         .await?;
    //     Ok(user)
    // }
    /// Verify email and password
    pub async fn verify_user(&self, input: &SigninUser) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "select id, ws_id, fullname, email, password_hash, created_at from users where email = $1",
        )
        .bind(&input.email)
        .fetch_optional(&self.pool)
        .await?;
        match user {
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let matches = verify_password(&input.password, &password_hash.unwrap_or_default())?;
                if matches {
                    let ws = self.find_workspace_by_id(user.ws_id as _).await?.unwrap();
                    user.ws_name = ws.name;
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
    pub async fn fetch_chat_user_by_ids(&self, ids: &[i64]) -> Result<Vec<ChatUser>, AppError> {
        let users =
            sqlx::query_as("select id, fullname, email from users where id = any($1) order by id")
                .bind(ids)
                .fetch_all(&self.pool)
                .await?;
        Ok(users)
    }
    pub async fn fetch_chat_users(&self, ws_id: u64) -> Result<Vec<ChatUser>, AppError> {
        let users =
            sqlx::query_as("select id, fullname, email from users where ws_id = $1 order by id")
                .bind(ws_id as i64)
                .fetch_all(&self.pool)
                .await?;
        Ok(users)
    }
}

// #[allow(dead_code)]
// impl ChatUser {}

fn hash_password(password: &str) -> Result<String, AppError> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(password_hash)?;
    let matches = argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    Ok(matches)
}

#[cfg(test)]
impl CreateUser {
    pub fn new(ws: &str, fullname: &str, email: &str, password: &str) -> Self {
        Self {
            fullname: fullname.to_string(),
            workspace: ws.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
impl SigninUser {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn hash_password_and_verify_should_work() -> anyhow::Result<()> {
        let password = "zhouzhangqi";
        let hash = hash_password(password)?;
        assert_eq!(hash.len(), 97);
        assert!(verify_password(password, &hash)?);
        Ok(())
    }

    #[tokio::test]
    async fn create_duplicate_user_should_fail() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = CreateUser::new("none", "zhouzhangqi", "zzq.gmail.com", "zhouzhangqi");
        let _user = state.create_user(&input).await?;
        let ret = state.create_user(&input).await;
        match ret {
            Err(AppError::EmailAlreadyExists(email)) => assert_eq!(email, input.email),
            _ => panic!("should fail"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = CreateUser::new("none", "zhouzhangqi", "zzq.gmail.com", "zhouzhangqi");
        let user = state.create_user(&input).await?;
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = state.find_user_by_email(&input.email).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);
        let user = state.verify_user(&input).await?;
        assert!(user.is_some());
        Ok(())
    }
    #[tokio::test]
    async fn find_user_by_id_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let user = state.find_user_by_id(1).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.id, 1);
        Ok(())
    }
}
