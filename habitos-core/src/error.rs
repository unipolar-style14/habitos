#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("could not resolve home/data directory")]
    NoHome,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("focus session already active (started {start_at})")]
    FocusAlreadyActive { start_at: String },
    #[error("no active focus session")]
    NoActiveFocus,
}
