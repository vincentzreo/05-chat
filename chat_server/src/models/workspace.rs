use sqlx::PgPool;

use crate::AppError;

use super::{ChatUser, Workspace};

impl Workspace {
    pub async fn create(name: &str, user_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        let ws = sqlx::query_as(r#"insert into workspaces (name, owner_id) values ($1, $2) returning id, name, owner_id, created_at"#)
            .bind(name)
            .bind(user_id as i64)
            .fetch_one(pool)
            .await?;
        Ok(ws)
    }
    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let ws = sqlx::query_as(
            r#"select id, name, owner_id, created_at from workspaces where name = $1"#,
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(ws)
    }
    #[allow(unused)]
    pub async fn find_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let ws = sqlx::query_as(
            r#"select id, name, owner_id, created_at from workspaces where id = $1"#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;
        Ok(ws)
    }
    pub async fn update_owner(&self, owner_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        // update owner_id in two cases 1) owner_id = 0 2) owner's ws_id = id
        let ws = sqlx::query_as(
            r#"update workspaces
            set owner_id = $1
            where id = $2 and (select ws_id FROM users where id = $1) = $2
            returning id, name, owner_id, created_at"#,
        )
        .bind(owner_id as i64)
        .bind(self.id)
        .fetch_one(pool)
        .await?;
        Ok(ws)
    }
    #[allow(dead_code)]
    pub async fn fetch_all_chat_users(id: u64, pool: &PgPool) -> Result<Vec<ChatUser>, AppError> {
        let users =
            sqlx::query_as(r#"select id, fullname, email from users where ws_id = $1 order by id"#)
                .bind(id as i64)
                .fetch_all(pool)
                .await?;
        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sqlx_db_tester::TestPg;

    use crate::{models::CreateUser, User};

    use super::*;
    #[tokio::test]
    async fn workspace_should_create_and_set_owner() -> anyhow::Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:postgres@localhost:5432".to_string(),
            Path::new("../migrations"),
        );
        let pool = tdb.get_pool().await;
        let ws = Workspace::create("test", 0, &pool).await?;
        let input = CreateUser::new(&ws.name, "zzq", "zzq@zzq.com", "zzq");
        let user = User::create(&input, &pool).await?;

        assert_eq!(ws.name, "test");

        // let user = user.add_to_workspace(ws.id, &pool).await?;
        assert_eq!(user.ws_id, ws.id);

        let ws = ws.update_owner(user.id as _, &pool).await?;
        assert_eq!(ws.owner_id, user.id);
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> anyhow::Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:postgres@localhost:5432".to_string(),
            Path::new("../migrations"),
        );
        let pool = tdb.get_pool().await;
        Workspace::create("test", 0, &pool).await?;
        let ws = Workspace::find_by_name("test", &pool).await?;
        assert_eq!(ws.unwrap().name, "test");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> anyhow::Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:postgres@localhost:5432".to_string(),
            Path::new("../migrations"),
        );
        let pool = tdb.get_pool().await;
        let ws = Workspace::create("test", 0, &pool).await?;
        let input = CreateUser::new(&ws.name, "zzq", "zzq@zzq.com", "zzq");
        let user1 = User::create(&input, &pool).await?;
        let input = CreateUser::new(&ws.name, "alice", "alice@alice.com", "alice");
        let user2 = User::create(&input, &pool).await?;

        let users = Workspace::fetch_all_chat_users(ws.id as _, &pool).await?;
        assert_eq!(users.len(), 2);

        assert_eq!(users[0].id, user1.id);
        assert_eq!(users[1].id, user2.id);
        Ok(())
    }
}
