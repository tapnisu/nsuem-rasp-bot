use sqlx::{Pool, Sqlite};

use crate::Schedule;

pub trait ScheduleCache {
    fn fetch_cached(
        group: &str,
        pool: &Pool<Sqlite>,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<Schedule>>> + Send;
    fn write_to_cache(
        &self,
        group: &str,
        pool: &Pool<Sqlite>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

impl ScheduleCache for Schedule {
    async fn fetch_cached(group: &str, pool: &Pool<Sqlite>) -> anyhow::Result<Option<Schedule>> {
        let schedule = sqlx::query!(
            r#"SELECT schedule_json FROM schedules WHERE group_name = ?"#,
            group
        )
        .fetch_optional(pool)
        .await?
        .map(|r| serde_json::from_str::<Schedule>(&r.schedule_json))
        .transpose()?;

        Ok(schedule)
    }

    async fn write_to_cache(&self, group: &str, pool: &Pool<Sqlite>) -> anyhow::Result<()> {
        let json = serde_json::to_string(self)?;

        sqlx::query!(
            "INSERT OR REPLACE INTO schedules(group_name, schedule_json) VALUES (?, ?)",
            group,
            json
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
