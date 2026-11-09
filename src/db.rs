use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

pub async fn init(url: &str) -> anyhow::Result<SqlitePool> {
    if url.contains("data/") {
        std::fs::create_dir_all("data").ok();
    }
    let opts = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/0002_seed.sql"))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/0003_reader_notes.sql"))
        .execute(&pool)
        .await?;
    Ok(pool)
}
