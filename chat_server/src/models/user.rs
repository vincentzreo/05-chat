use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{AppError, User};

use super::{ChatUser, Workspace};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateUser {
    pub fullname: String,
    pub email: String,
    pub workspace: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SigninUser {
    pub email: String,
    pub password: String,
}

impl User {
    /// Find a user by email
    pub async fn find_by_email(email: &str, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as(
            "select id, ws_id, fullname, email, created_at from users where email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;
        Ok(user)
    }
    /// Create a new user
    pub async fn create(input: &CreateUser, pool: &PgPool) -> Result<Self, AppError> {
        // check if the email is already in use
        let user = User::find_by_email(&input.email, pool).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }
        // check if workspace exists, if not create one
        let ws = match Workspace::find_by_name(&input.workspace, pool).await? {
            Some(ws) => ws,
            None => Workspace::create(&input.workspace, 0, pool).await?,
        };

        let password_hash = hash_password(&input.password)?;
        let user: User = sqlx::query_as(
            "insert into users (ws_id, email, fullname, password_hash) values ($1, $2, $3, $4) returning id, ws_id, fullname, email, created_at",
        )
        .bind(ws.id)
        .bind(&input.email)
        .bind(&input.fullname)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;
        if ws.owner_id == 0 {
            ws.update_owner(user.id as _, pool).await?;
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
    pub async fn verify(input: &SigninUser, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "select id, ws_id, fullname, email, password_hash, created_at from users where email = $1",
        )
        .bind(&input.email)
        .fetch_optional(pool)
        .await?;
        match user {
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let matches = verify_password(&input.password, &password_hash.unwrap_or_default())?;
                if matches {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

#[allow(dead_code)]
impl ChatUser {
    pub async fn fetch_by_ids(ids: &[i64], pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let users =
            sqlx::query_as("select id, fullname, email from users where id = any($1) order by id")
                .bind(ids)
                .fetch_all(pool)
                .await?;
        Ok(users)
    }
    pub async fn fetch_all(ws_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let users =
            sqlx::query_as("select id, fullname, email from users where ws_id = $1 order by id")
                .bind(ws_id as i64)
                .fetch_all(pool)
                .await?;
        Ok(users)
    }
}

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
impl User {
    pub fn new(id: i64, fullname: &str, email: &str) -> Self {
        Self {
            id,
            ws_id: 0,
            fullname: fullname.to_string(),
            email: email.to_string(),
            password_hash: None,
            created_at: chrono::Utc::now(),
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

    use crate::test_util::get_test_pool;

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
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateUser::new("none", "zhouzhangqi", "zzq.gmail.com", "zhouzhangqi");
        let _user = User::create(&input, &pool).await?;
        let ret = User::create(&input, &pool).await;
        match ret {
            Err(AppError::EmailAlreadyExists(email)) => assert_eq!(email, input.email),
            _ => panic!("should fail"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> anyhow::Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateUser::new("none", "zhouzhangqi", "zzq.gmail.com", "zhouzhangqi");
        let user = User::create(&input, &pool).await?;
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = User::find_by_email(&input.email, &pool).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);
        let user = User::verify(&input, &pool).await?;
        assert!(user.is_some());
        Ok(())
    }
}
