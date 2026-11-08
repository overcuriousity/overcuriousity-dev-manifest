use std::path::Path;

use oc_kit::OcError;
use sqlx::SqlitePool;

pub async fn open(data_dir: &Path) -> Result<SqlitePool, OcError> {
    let pool = oc_kit::db::connect(data_dir, "demo.sqlite").await?;
    migrate(&pool).await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), OcError> {
    oc_kit::db::migrate_kit(pool).await?;
    let mut m = sqlx::migrate!("./migrations");
    m.set_ignore_missing(true);
    m.run(pool).await.map_err(OcError::internal)
}
