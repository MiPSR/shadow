use sqlx::SqlitePool;

pub async fn connect(url: &str) -> SqlitePool {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("db connect")
}

pub async fn migrate(pool: &SqlitePool) {
    let stmts = [
        r#"CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            auth_methods TEXT NOT NULL,
            is_admin INTEGER NOT NULL DEFAULT 0
        )"#,
        r#"CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            mldsa_pk TEXT NOT NULL,
            mlkem_pk TEXT NOT NULL,
            revoked INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            device_id TEXT NOT NULL,
            expires_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS channels (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            members TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            channel_id TEXT NOT NULL,
            sender_device_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            payload TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS key_announcements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_device TEXT NOT NULL,
            to_device TEXT NOT NULL,
            channel_id TEXT,
            kem_ciphertext TEXT NOT NULL,
            nonce TEXT NOT NULL,
            wrapped_key TEXT NOT NULL,
            signature TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS signals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_device TEXT NOT NULL,
            to_device TEXT NOT NULL,
            channel_id TEXT,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS attachments (
            id TEXT PRIMARY KEY,
            message_id TEXT,
            size INTEGER NOT NULL,
            media_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            actor TEXT,
            target TEXT,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            mldsa_pk TEXT NOT NULL,
            mlkem_pk TEXT NOT NULL,
            revoked INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_seen INTEGER
        )"#,
    ];
    for s in stmts {
        sqlx::query(s).execute(pool).await.expect("migrate");
    }
    // add last_activity column if missing
    let _ = sqlx::query("ALTER TABLE channels ADD COLUMN last_activity INTEGER").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN last_seen INTEGER").execute(pool).await;
}
