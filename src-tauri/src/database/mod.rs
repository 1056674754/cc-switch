//! 数据库模块 - SQLite 数据持久化
//!
//! 此模块提供应用的核心数据存储功能，包括：
//! - 供应商配置管理
//! - MCP 服务器配置
//! - 提示词管理
//! - Skills 管理
//! - 通用设置存储
//!
//! ## 架构设计
//!
//! ```text
//! database/
//! ├── mod.rs        - Database 结构体 + 初始化
//! ├── schema.rs     - 表结构定义 + Schema 迁移
//! ├── backup.rs     - SQL 导入导出 + 快照备份
//! ├── migration.rs  - JSON → SQLite 数据迁移
//! └── dao/          - 数据访问对象
//!     ├── providers.rs
//!     ├── mcp.rs
//!     ├── prompts.rs
//!     ├── skills.rs
//!     └── settings.rs
//! ```

pub(crate) mod backup;
mod dao;
mod migration;
mod pool;
mod schema;

#[cfg(test)]
mod tests;

// DAO 类型导出供外部使用
pub use dao::custom_icons::{CustomIconEntry, CUSTOM_ICONS_KEY};
pub(crate) use dao::providers_seed::{
    is_official_seed_id, CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID, CODEX_OFFICIAL_PROVIDER_ID,
    GROKBUILD_OFFICIAL_PROVIDER_ID,
};
pub(crate) use dao::proxy::{PRICING_SOURCE_REQUEST, PRICING_SOURCE_RESPONSE};
pub use dao::FailoverQueueItem;
pub use dao::Profile;

use crate::config::get_app_config_dir;
use crate::error::AppError;
use rusqlite::{hooks::Action, Connection};
use serde::Serialize;
use std::sync::Mutex;

// DAO 方法通过 impl Database 提供，无需额外导出

/// 当前 Schema 版本号
/// 每次修改表结构时递增，并在 schema.rs 中添加相应的迁移逻辑
pub(crate) const SCHEMA_VERSION: i32 = 31;

/// 安全地序列化 JSON，避免 unwrap panic
pub(crate) fn to_json_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|e| AppError::Config(format!("JSON serialization failed: {e}")))
}

/// provider → 网关级上游绑定（熔断键 + host），供代理层使用
pub(crate) use dao::providers::UpstreamBinding;
pub(crate) use pool::ConnectionPool;

/// 安全地获取一条数据库连接，避免 unwrap panic
macro_rules! lock_conn {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|e| AppError::Database(format!("获取数据库连接失败: {}", e)))?
    };
}

// 导出宏供子模块使用
pub(crate) use lock_conn;

/// 数据库连接封装
///
/// `conn` 是一个连接池（见 [`pool`]）：字段名、`lock()` 方法和守卫的解引用都刻意与
/// 原来的 `Mutex<Connection>` 保持一致，`lock_conn!` 与各处 `.lock()` 无需改动。
/// 池化之后读写不再互相排队——写仍然由 SQLite 串行，但排在 `busy_timeout` 上。
pub struct Database {
    pub(crate) conn: ConnectionPool,
    /// 请求日志总数缓存，见 `services::usage_stats::LogCountCache`
    pub(crate) log_count_cache: Mutex<Option<crate::services::usage_stats::LogCountCache>>,
}

/// 每一条新连接都要跑的配置。
///
/// `busy_timeout` 与 `foreign_keys` 是**连接级**的，必须逐条设置；`journal_mode=WAL`
/// 是文件级的，重复执行是幂等的（内存库会返回 `memory`，这不是错误）。
fn configure_connection(conn: &Connection) {
    let _ = conn.busy_timeout(pool::BUSY_TIMEOUT);
    if let Err(e) = conn.execute("PRAGMA foreign_keys = ON;", []) {
        log::warn!("设置 foreign_keys 失败: {e}");
    }
    if let Err(e) = conn.pragma_update(None, "synchronous", "NORMAL") {
        log::warn!("设置 synchronous 失败: {e}");
    }
    if let Err(e) = conn.query_row("PRAGMA journal_mode=WAL;", [], |row| {
        row.get::<_, String>(0)
    }) {
        log::warn!("设置 journal_mode 失败: {e}");
    }
    register_db_change_hook(conn);
}

fn register_db_change_hook(conn: &Connection) {
    conn.update_hook(Some(
        |action: Action, _database: &str, table: &str, _row_id: i64| match action {
            Action::SQLITE_INSERT | Action::SQLITE_UPDATE | Action::SQLITE_DELETE => {
                DB_WRITE_EPOCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                crate::services::webdav_auto_sync::notify_db_changed(table);
                crate::services::s3_auto_sync::notify_db_changed(table);
            }
            _ => {}
        },
    ));
}

/// 进程内的写入纪元：任何一次行级增删改都会让它加一，**跨连接可见**。
///
/// `Connection::total_changes()` 是每条连接自己的计数器，池化之后换一条连接就归零，
/// 不能再用来判断「上次统计完之后库有没有被写过」。钩子在每条连接上都注册了，
/// 这里补一个全局计数顶替它（使用方见 `services::usage_stats::LogCountCache`）。
static DB_WRITE_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 当前写入纪元；与上次记录的值不同即说明期间有过写入。
pub(crate) fn db_write_epoch() -> u64 {
    DB_WRITE_EPOCH.load(std::sync::atomic::Ordering::Relaxed)
}

impl Database {
    /// 初始化数据库连接并创建表
    ///
    /// 数据库文件位于 `~/.cc-switch/cc-switch.db`
    pub fn init() -> Result<Self, AppError> {
        let db_path = get_app_config_dir().join("cc-switch.db");
        let db_exists = db_path.exists();

        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let conn = ConnectionPool::file(db_path.clone(), configure_connection)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Self {
            conn,
            log_count_cache: Mutex::new(None),
        };

        if !db_exists {
            // For a brand-new database, configure incremental auto-vacuum
            // before creating any tables so no rebuild is needed later.
            let conn = lock_conn!(db.conn);
            conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        db.create_tables()?;

        // Pre-migration backup: only when upgrading from an existing database
        {
            let conn = lock_conn!(db.conn);
            let version = Self::get_user_version(&conn)?;
            drop(conn);
            if version > 0 && version < SCHEMA_VERSION {
                log::info!(
                    "Creating pre-migration database backup (v{version} → v{SCHEMA_VERSION})"
                );
                if let Err(e) = db.backup_database_file() {
                    log::warn!("Pre-migration backup failed, continuing migration: {e}");
                }
            }
        }

        db.apply_schema_migrations()?;
        if let Err(e) = db.ensure_incremental_auto_vacuum() {
            log::warn!("Failed to ensure incremental auto-vacuum: {e}");
        }
        db.ensure_model_pricing_seeded()?;
        if let Err(e) = crate::services::model_pricing::sync_local_model_pricing(&db) {
            log::warn!("Failed to sync local model pricing file: {e}");
        }

        // Startup cleanup: prune old logs and reclaim space
        if let Err(e) = db.cleanup_old_stream_check_logs(7) {
            log::warn!("Startup stream_check_logs cleanup failed: {e}");
        }
        if let Err(e) = db.rollup_and_prune(30) {
            log::warn!("Startup rollup_and_prune failed: {e}");
        }
        // Reclaim disk space after cleanup
        {
            let conn = lock_conn!(db.conn);
            if let Err(e) = Self::incremental_vacuum_on_conn(&conn) {
                log::warn!("Startup incremental vacuum failed: {e}");
            }
        }

        Ok(db)
    }

    /// 读取磁盘上数据库的 `user_version`；仅当它比应用支持的 [`SCHEMA_VERSION`]
    /// 更新时返回 `Some(version)`。
    ///
    /// 用于初始化失败后判断是否为「数据库版本过新（应用过旧，需升级应用）」的可恢复
    /// 场景——此时不应反复弹出无效的重试对话框，而应引导用户在应用内升级。
    pub fn stored_user_version_exceeds_supported(
        db_path: &std::path::Path,
    ) -> Result<Option<i32>, AppError> {
        if !db_path.exists() {
            return Ok(None);
        }
        let conn = Connection::open(db_path).map_err(|e| AppError::Database(e.to_string()))?;
        let version = Self::get_user_version(&conn)?;
        Ok((version > SCHEMA_VERSION).then_some(version))
    }

    /// 创建内存数据库（用于测试）
    pub fn memory() -> Result<Self, AppError> {
        let conn = ConnectionPool::memory(configure_connection)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Self {
            conn,
            log_count_cache: Mutex::new(None),
        };
        {
            let conn = lock_conn!(db.conn);
            conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        db.create_tables()?;
        db.ensure_model_pricing_seeded()?;

        Ok(db)
    }

    /// 测试用：在指定路径上打开数据库。
    ///
    /// 和 `init()` 用同一套连接配置（PRAGMA + 变更钩子），但不做迁移和启动清理，
    /// 也不假定路径是应用配置目录下的那一个。
    #[cfg(test)]
    pub(crate) fn open_for_test(path: &std::path::Path) -> Result<Self, AppError> {
        let conn = ConnectionPool::file(path.to_path_buf(), configure_connection)
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self {
            conn,
            log_count_cache: Mutex::new(None),
        })
    }

    /// 库文件路径；内存库是 `None`。
    pub(crate) fn file_path(&self) -> Option<&std::path::Path> {
        self.conn.path()
    }

    /// 打开一个已有的库并跑迁移（真实库副本的迁移验证用）。与 [`Self::init`] 的
    /// create_tables → apply_schema_migrations 两条一致，但路径由调用方给。
    #[cfg(test)]
    pub(crate) fn open_and_migrate_for_test(path: &std::path::Path) -> Result<Self, AppError> {
        let db = Self::open_for_test(path)?;
        db.create_tables()?;
        db.apply_schema_migrations()?;
        Ok(db)
    }

    pub(crate) fn get_auto_vacuum_mode(conn: &Connection) -> Result<i32, AppError> {
        conn.query_row("PRAGMA auto_vacuum;", [], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("读取 auto_vacuum 失败: {e}")))
    }

    /// 回收全部空闲页。`PRAGMA incremental_vacuum` 每释放一页产出一行结果，
    /// 必须把结果读完才会回收完；`execute_batch` 只 step 一次，只能回收 1 页。
    pub(crate) fn incremental_vacuum_on_conn(conn: &Connection) -> Result<(), AppError> {
        let mut stmt = conn
            .prepare("PRAGMA incremental_vacuum;")
            .map_err(|e| AppError::Database(format!("执行 incremental_vacuum 失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("执行 incremental_vacuum 失败: {e}")))?;
        while rows
            .next()
            .map_err(|e| AppError::Database(format!("执行 incremental_vacuum 失败: {e}")))?
            .is_some()
        {}
        Ok(())
    }

    fn has_user_tables(conn: &Connection) -> Result<bool, AppError> {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("读取表数量失败: {e}")))?;
        Ok(count > 0)
    }

    pub(crate) fn ensure_incremental_auto_vacuum_on_conn(
        conn: &Connection,
    ) -> Result<bool, AppError> {
        let mode = Self::get_auto_vacuum_mode(conn)?;
        if mode == 2 {
            return Ok(false);
        }

        let has_tables = Self::has_user_tables(conn)?;
        conn.execute("PRAGMA auto_vacuum = INCREMENTAL;", [])
            .map_err(|e| AppError::Database(format!("设置 auto_vacuum 失败: {e}")))?;

        if !has_tables {
            return Ok(false);
        }

        conn.execute("VACUUM;", [])
            .map_err(|e| AppError::Database(format!("执行 VACUUM 失败: {e}")))?;
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(format!("恢复 foreign_keys 失败: {e}")))?;
        Ok(true)
    }

    pub(crate) fn ensure_incremental_auto_vacuum(&self) -> Result<bool, AppError> {
        let mode = {
            let conn = lock_conn!(self.conn);
            Self::get_auto_vacuum_mode(&conn)?
        };
        if mode == 2 {
            return Ok(false);
        }

        let has_tables = {
            let conn = lock_conn!(self.conn);
            Self::has_user_tables(&conn)?
        };
        if has_tables {
            log::info!(
                "Detected auto_vacuum={mode}, rebuilding database to enable incremental vacuum"
            );
            self.backup_database_file()?;
        }

        let rebuilt = {
            let conn = lock_conn!(self.conn);
            Self::ensure_incremental_auto_vacuum_on_conn(&conn)?
        };

        if rebuilt {
            log::info!("Incremental auto-vacuum enabled after database rebuild");
        } else {
            log::info!("Incremental auto-vacuum configured for new database");
        }

        Ok(rebuilt)
    }

    /// 检查 MCP 服务器表是否为空
    pub fn is_mcp_table_empty(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_servers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count == 0)
    }

    /// 检查提示词表是否为空
    pub fn is_prompts_table_empty(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count == 0)
    }
}
