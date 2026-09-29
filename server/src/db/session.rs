//! セッションの問い合わせ（docs/schema.md の「セッション」）。

use super::Pool;

pub async fn create(pool: &Pool, id_hash: &str, now: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO session (id_hash, created_at, last_used_at) VALUES (?, ?, ?)",
        id_hash,
        now,
        now
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// 最終利用日時を返す。
pub async fn last_used_at(pool: &Pool, id_hash: &str) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT last_used_at FROM session WHERE id_hash = ?",
        id_hash
    )
    .fetch_optional(pool)
    .await
}

pub async fn touch(pool: &Pool, id_hash: &str, now: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE session SET last_used_at = ? WHERE id_hash = ?",
        now,
        id_hash
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &Pool, id_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM session WHERE id_hash = ?", id_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// 最終利用日時が `before` より前のセッションを消す。
pub async fn delete_unused_since(pool: &Pool, before: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM session WHERE last_used_at < ?", before)
        .execute(pool)
        .await?;
    Ok(())
}
