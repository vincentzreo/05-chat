use crate::{AppError, AppState};

use chat_core::Workspace;

impl AppState {
    pub async fn create_workspace(&self, name: &str, user_id: u64) -> Result<Workspace, AppError> {
        let ws = sqlx::query_as(r#"insert into workspaces (name, owner_id) values ($1, $2) returning id, name, owner_id, created_at"#)
            .bind(name)
            .bind(user_id as i64)
            .fetch_one(&self.pool)
            .await?;
        Ok(ws)
    }
    pub async fn find_workspace_by_name(&self, name: &str) -> Result<Option<Workspace>, AppError> {
        let ws = sqlx::query_as(
            r#"select id, name, owner_id, created_at from workspaces where name = $1"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(ws)
    }
    #[allow(unused)]
    pub async fn find_workspace_by_id(&self, id: u64) -> Result<Option<Workspace>, AppError> {
        let ws = sqlx::query_as(
            r#"select id, name, owner_id, created_at from workspaces where id = $1"#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;
        Ok(ws)
    }
    pub async fn update_workspace_owner(
        &self,
        id: u64,
        owner_id: u64,
    ) -> Result<Workspace, AppError> {
        // update owner_id in two cases 1) owner_id = 0 2) owner's ws_id = id
        let ws = sqlx::query_as(
            r#"update workspaces
            set owner_id = $1
            where id = $2 and (select ws_id FROM users where id = $1) = $2
            returning id, name, owner_id, created_at"#,
        )
        .bind(owner_id as i64)
        .bind(id as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(ws)
    }
}

#[cfg(test)]
mod tests {

    use crate::models::CreateUser;

    use super::*;
    #[tokio::test]
    async fn workspace_should_create_and_set_owner() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let ws = state.create_workspace("test", 0).await?;
        let input = CreateUser::new(&ws.name, "zzq12121", "zzq1212121@zzq.com", "zzq");
        let user = state.create_user(&input).await?;

        assert_eq!(ws.name, "test");

        // let user = user.add_to_workspace(ws.id, &pool).await?;
        assert_eq!(user.ws_id, ws.id);

        let ws = state
            .update_workspace_owner(ws.id as _, user.id as _)
            .await?;
        assert_eq!(ws.owner_id, user.id);
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let ws = state.find_workspace_by_name("acme").await?;
        assert_eq!(ws.unwrap().name, "acme");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let users = state.fetch_chat_users(1).await?;

        assert_eq!(users.len(), 5);
        Ok(())
    }
}
