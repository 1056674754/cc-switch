//! 网关模型配置（P1）：每 app 的模型配置 + 目录的人工覆盖。
//!
//! 两张表在 v27 迁移里建（`database/schema.rs::create_gateway_tables`）：
//! `gateway_app_profiles`（每 app 一份，对应 CCR `ProfileConfig` 里的模型部分）与
//! `gateway_model_overrides`（人工改名/隐藏，目录本身仍由 provider 行派生）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::Database;
use crate::error::AppError;

/// 每 app 的模型配置。字段对齐 CCR 的 `ProfileConfig`（P0 §3.4 给出每个 app 的落点）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppModelConfig {
    /// 默认模型：`<供应商名>/<模型 id>`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// 槽位 → 模型引用。哪几个槽位可用由 app 决定（claude 四个、codex 两个…），空的就不写。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slots: BTreeMap<String, String>,
    /// 允许模型列表；空 = 不限制。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed: Vec<String>,
    /// 路由策略（增强路由 / 配置级路由），原样存。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<Value>,
}

/// 目录里的一条人工覆盖：改名或隐藏。目录本身仍由 provider 行派生，这里只存差异。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOverride {
    pub upstream_id: String,
    pub model_id: String,
    pub display_name: Option<String>,
    pub hidden: bool,
}

impl Database {
    /// 读某 app 的模型配置；没配过返回 `None`。
    pub fn get_app_model_config(&self, app_type: &str) -> Result<Option<AppModelConfig>, AppError> {
        let conn = crate::database::lock_conn!(self.conn);
        let payload: Option<String> = conn
            .query_row(
                "SELECT payload FROM gateway_app_profiles WHERE app_type = ?1",
                [app_type],
                |row| row.get(0),
            )
            .ok();
        let Some(payload) = payload else {
            return Ok(None);
        };
        let config = serde_json::from_str(&payload)
            .map_err(|e| AppError::Config(format!("解析 {app_type} 模型配置失败: {e}")))?;
        Ok(Some(config))
    }

    /// 写某 app 的模型配置（覆盖）。
    pub fn set_app_model_config(
        &self,
        app_type: &str,
        config: &AppModelConfig,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string(config)
            .map_err(|e| AppError::Config(format!("序列化 {app_type} 模型配置失败: {e}")))?;
        let now = chrono::Utc::now().timestamp();
        let conn = crate::database::lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO gateway_app_profiles (app_type, payload, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(app_type) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
            rusqlite::params![app_type, payload, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 目录的全部人工覆盖。
    pub fn list_model_overrides(&self) -> Result<Vec<ModelOverride>, AppError> {
        let conn = crate::database::lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT upstream_id, model_id, display_name, hidden FROM gateway_model_overrides",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ModelOverride {
                upstream_id: row.get(0)?,
                model_id: row.get(1)?,
                display_name: row.get(2)?,
                hidden: row.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 写一条人工覆盖（改名/隐藏）。
    pub fn set_model_override(
        &self,
        upstream_id: &str,
        model_id: &str,
        display_name: Option<&str>,
        hidden: bool,
    ) -> Result<(), AppError> {
        let id = Database::p1_stable_hash(&format!("{upstream_id}|{model_id}"));
        let now = chrono::Utc::now().timestamp();
        let conn = crate::database::lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO gateway_model_overrides
                 (id, upstream_id, model_id, display_name, hidden, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(upstream_id, model_id) DO UPDATE SET
                 display_name = excluded.display_name,
                 hidden = excluded.hidden",
            rusqlite::params![id, upstream_id, model_id, display_name, hidden as i64, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 删掉一条人工覆盖，回到派生值。
    pub fn clear_model_override(&self, upstream_id: &str, model_id: &str) -> Result<(), AppError> {
        let conn = crate::database::lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM gateway_model_overrides WHERE upstream_id = ?1 AND model_id = ?2",
            rusqlite::params![upstream_id, model_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_model_config_round_trips() {
        let db = Database::memory().expect("memory db");
        assert!(db.get_app_model_config("claude").expect("get").is_none());

        let config = AppModelConfig {
            default: Some("CTYun/deepseek-v4.1-flash-1017".into()),
            slots: BTreeMap::from([("haiku".into(), "CTYun/qwen3.8-omni-flash".into())]),
            allowed: vec!["CTYun/deepseek-v4.1-flash-1017".into()],
            routing: None,
        };
        db.set_app_model_config("claude", &config).expect("set");

        let read = db
            .get_app_model_config("claude")
            .expect("get")
            .expect("some");
        assert_eq!(read, config);
        // 其它 app 不受影响
        assert!(db.get_app_model_config("codex").expect("get").is_none());
    }

    #[test]
    fn model_overrides_round_trip_and_clear() {
        let db = Database::memory().expect("memory db");
        assert!(db.list_model_overrides().expect("list").is_empty());

        db.set_model_override("up-1", "kimi-k3", Some("Kimi K3"), false)
            .expect("set");
        let overrides = db.list_model_overrides().expect("list");
        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].display_name.as_deref(), Some("Kimi K3"));

        // 同一个（上游, 模型）覆盖写，不新增行
        db.set_model_override("up-1", "kimi-k3", None, true)
            .expect("set");
        let overrides = db.list_model_overrides().expect("list");
        assert_eq!(overrides.len(), 1);
        assert!(overrides[0].hidden);

        db.clear_model_override("up-1", "kimi-k3").expect("clear");
        assert!(db.list_model_overrides().expect("list").is_empty());
    }
}
