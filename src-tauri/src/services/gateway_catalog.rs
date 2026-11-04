//! 网关模型目录的派生（P1 切片的纯逻辑部分）。
//!
//! 原则：**不重写模型推导**。claude / claude-desktop / codex 直接调用
//! [`crate::mode::stack`] 里已有的三个函数（`[1M]` 剥标记、显示名、窗口规则都在里面）；
//! 其余 app 的推导在后续切片补，调用方用 [`supports`] 判断覆盖范围。
//!
//! 目录的键是（端点, 规范模型名）：同一个上游被多个 app 引用时只出现一次，
//! `sources` 记下是哪些 app 的行贡献的。

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Serialize;
use serde_json::Value;

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::model_mapper::strip_one_m_suffix_for_upstream;

/// 目录里的一条。
///
/// 键是（上游, 模型）而不是（端点, 模型）：真实库副本跑出来 CTYun 有两条端点
///（`anthropic` 与 `openai_responses`），同一个 `deepseek-v4.1-flash-1017` 会重复出现。
/// 用户选的是模型，协议由接入方的绑定决定，所以目录按上游合并、端点在解析时再选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub upstream_id: String,
    /// 这个上游下能承载该模型的端点（按协议区分）。
    pub endpoint_ids: Vec<String>,
    /// 规范模型名：去掉 `[1M]` 标记后的原值。
    pub model_id: String,
    pub display_name: Option<String>,
    pub one_m: bool,
    /// 已知的上下文窗口；未知是 `None`。
    pub window: Option<u64>,
    /// 贡献这个模型的来源，形如 `claude:CTYun`。
    pub sources: Vec<String>,
}

/// 一个 provider 行贡献的模型（还没按端点去重）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawModel {
    model: String,
    display: Option<String>,
    one_m: bool,
    window: Option<u64>,
}

/// 一个 provider 行贡献的模型清单。推导全部来自现有函数，这里只做 `[1M]` 归一。
fn raw_models(app: &AppType, provider: &Provider) -> Vec<RawModel> {
    match app {
        AppType::Claude => crate::mode::stack::claude_models("catalog", provider)
            .into_iter()
            .map(|model| RawModel {
                model: strip_one_m_suffix_for_upstream(&model.upstream)
                    .trim()
                    .to_string(),
                display: Some(model.name.clone()),
                one_m: model.one_m,
                window: Some(model.window),
            })
            .collect(),
        AppType::ClaudeDesktop => crate::mode::stack::desktop_models("catalog", provider)
            .into_iter()
            .map(|model| RawModel {
                model: strip_one_m_suffix_for_upstream(&model.upstream)
                    .trim()
                    .to_string(),
                display: Some(model.label.clone()),
                one_m: model.supports_1m,
                window: None,
            })
            .collect(),
        AppType::Codex => {
            let config = provider
                .settings_config
                .get("config")
                .and_then(Value::as_str)
                .unwrap_or("");
            crate::codex_config::codex_published_models(&provider.settings_config, config)
                .into_iter()
                .map(|raw| RawModel {
                    one_m: raw.to_ascii_lowercase().contains("[1m]"),
                    model: strip_one_m_suffix_for_upstream(&raw).trim().to_string(),
                    display: None,
                    window: None,
                })
                .collect()
        }
        _ => config_models(app, provider),
    }
}

/// 累加式 app 的模型：各家配置里的 `models` 容器形态不同，统一按下面三种读。
///
/// - 对象：`id → 元信息`（opencode/opencode V2/dsh/kimi/mcode）；kimi 的键是
///   `<provider>/<model>`，上游模型名取内层的 `model` 字段
/// - 数组：元素带 `id`（omp/openclaw），或只有一个键的对象（hermes）
/// - 字符串数组：模型名本身就是 id
fn config_models(app: &AppType, provider: &Provider) -> Vec<RawModel> {
    let settings = match app {
        // opencode 的 settings_config 可能是整份文件（旧数据），按读取路径抽那一条声明
        AppType::OpenCode => {
            crate::opencode_config::provider_fragment(&provider.id, &provider.settings_config, None)
                .map(|(fragment, _)| fragment.clone())
                .unwrap_or_else(|_| provider.settings_config.clone())
        }
        _ => provider.settings_config.clone(),
    };
    let Some(models) = settings.get("models") else {
        return Vec::new();
    };
    let mut found: Vec<RawModel> = Vec::new();
    let push = |model: &str, meta: Option<&Value>, found: &mut Vec<RawModel>| {
        let model = model.trim();
        if model.is_empty() {
            return;
        }
        let one_m = model.to_ascii_lowercase().contains("[1m]");
        let model = strip_one_m_suffix_for_upstream(model).trim().to_string();
        let display = meta
            .and_then(|meta| {
                ["name", "display_name", "displayName", "label"]
                    .iter()
                    .find_map(|key| meta.get(*key).and_then(Value::as_str))
            })
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        let window = meta.and_then(model_window);
        match found.iter_mut().find(|entry| entry.model == model) {
            Some(entry) => {
                entry.one_m |= one_m;
                if entry.display.is_none() {
                    entry.display = display;
                }
                if entry.window.is_none() {
                    entry.window = window;
                }
            }
            None => found.push(RawModel {
                model,
                display,
                one_m,
                window,
            }),
        }
    };
    match models {
        Value::Object(map) => {
            for (key, meta) in map {
                let model = meta
                    .get("model")
                    .and_then(Value::as_str)
                    .filter(|model| !model.trim().is_empty())
                    .unwrap_or(key);
                push(model, Some(meta), &mut found);
            }
        }
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(model) => push(model, None, &mut found),
                    Value::Object(map) => {
                        if let Some(id) = map.get("id").and_then(Value::as_str) {
                            push(id, Some(item), &mut found);
                        } else if map.len() == 1 {
                            let (key, meta) = map.iter().next().expect("one entry");
                            push(key, Some(meta), &mut found);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    found
}

/// 模型元信息里的窗口字段：各家叫法不同。
fn model_window(meta: &Value) -> Option<u64> {
    for key in ["contextWindow", "max_context_size", "context_length"] {
        if let Some(window) = meta.get(key).and_then(Value::as_u64) {
            return Some(window);
        }
    }
    for pointer in ["/limit/context", "/limits/context"] {
        if let Some(window) = meta.pointer(pointer).and_then(Value::as_u64) {
            return Some(window);
        }
    }
    None
}

/// 把一条原始模型并进目录：同（端点, 模型）合并显示信息，`one_m` 取或。
fn merge(
    entries: &mut BTreeMap<(String, String), CatalogEntry>,
    upstream_id: &str,
    endpoint_id: &str,
    source: &str,
    raw: RawModel,
) {
    if raw.model.is_empty() {
        return;
    }
    let key = (upstream_id.to_string(), raw.model.clone());
    match entries.get_mut(&key) {
        Some(entry) => {
            entry.one_m |= raw.one_m;
            if entry.display_name.is_none() {
                entry.display_name = raw.display;
            }
            if entry.window.is_none() {
                entry.window = raw.window;
            }
            if !entry.endpoint_ids.iter().any(|known| known == endpoint_id) {
                entry.endpoint_ids.push(endpoint_id.to_string());
            }
            if !entry.sources.iter().any(|known| known == source) {
                entry.sources.push(source.to_string());
            }
        }
        None => {
            entries.insert(
                key,
                CatalogEntry {
                    upstream_id: upstream_id.to_string(),
                    endpoint_ids: vec![endpoint_id.to_string()],
                    model_id: raw.model,
                    display_name: raw.display,
                    one_m: raw.one_m,
                    window: raw.window,
                    sources: vec![source.to_string()],
                },
            );
        }
    }
}

/// 目录里给 UI 用的一条：模型引用 = `<供应商名>/<模型 id>`，供应商重名时加序号。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogViewEntry {
    /// 模型引用，`<供应商名>/<模型 id>`。
    pub reference: String,
    pub upstream_id: String,
    pub upstream_name: String,
    pub model_id: String,
    pub display_name: Option<String>,
    pub one_m: bool,
    pub window: Option<u64>,
    pub endpoint_ids: Vec<String>,
    pub sources: Vec<String>,
}

/// 给每个上游分配一个不重复的显示名：重名加 `（2）`、`（3）`。
fn unique_upstream_names(
    upstreams: &[(String, String)],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut names = HashMap::new();
    for (id, name) in upstreams {
        let seen = counts.entry(name.as_str()).or_insert(0);
        *seen += 1;
        let unique = if *seen == 1 {
            name.clone()
        } else {
            format!("{name}（{seen}）")
        };
        names.insert(id.clone(), unique);
    }
    names
}

/// 从库里派生整份目录（带模型引用）。读之前先对账绑定（从未走过代理的 app 在库里没有绑定）。
pub fn derive_entries(db: &Database) -> Result<Vec<CatalogEntry>, AppError> {
    // 1. 先对账：绑定由路由器的读路径派生，没走过代理的 app 需要补。
    let (apps, bindings, endpoint_upstream): (
        Vec<String>,
        BTreeMap<(String, String), String>,
        BTreeMap<String, String>,
    ) = {
        let mut conn = crate::database::lock_conn!(db.conn);
        let apps: Vec<String> = {
            let mut stmt = conn.prepare("SELECT DISTINCT app_type FROM providers")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for app in &apps {
            if let Err(error) = Database::reconcile_provider_bindings(&mut conn, app) {
                log::warn!("reconcile bindings for {app} failed: {error}");
            }
        }
        let mut stmt =
            conn.prepare("SELECT app_type, provider_id, endpoint_id FROM provider_bindings")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
            ))
        })?;
        let bindings = rows.collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut stmt = conn.prepare("SELECT id, upstream_id FROM upstream_endpoints")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let endpoints = rows.collect::<Result<BTreeMap<_, _>, _>>()?;
        (apps, bindings, endpoints)
    };

    // 2. 逐 app 读 provider 行，用现有推导函数取模型，按（端点, 模型）合并。
    let mut entries: BTreeMap<(String, String), CatalogEntry> = BTreeMap::new();
    for app in &apps {
        let Ok(app) = AppType::from_str(app) else {
            continue;
        };
        let providers = match db.get_all_providers(app.as_str()) {
            Ok(providers) => providers,
            Err(error) => {
                log::warn!("read {} providers failed: {error}", app.as_str());
                continue;
            }
        };
        for (provider_id, provider) in providers {
            let key = (app.as_str().to_string(), provider_id);
            let Some(endpoint_id) = bindings.get(&key) else {
                continue;
            };
            let Some(upstream_id) = endpoint_upstream.get(endpoint_id) else {
                continue;
            };
            let source = format!("{}:{}", app.as_str(), provider.name);
            for raw in raw_models(&app, &provider) {
                merge(&mut entries, upstream_id, endpoint_id, &source, raw);
            }
        }
    }

    Ok(entries.into_values().collect())
}

/// 目录 + 每条的上游显示名与模型引用。上游名来自 `upstreams.name`，重名按出现顺序加序号。
pub fn catalog_view(db: &Database) -> Result<Vec<CatalogViewEntry>, AppError> {
    let entries = derive_entries(db)?;
    let mut upstream_names: Vec<(String, String)> = {
        let connected = crate::database::lock_conn!(db.conn);
        let mut stmt =
            connected.prepare("SELECT id, COALESCE(NULLIF(name, ''), host) FROM upstreams")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    upstream_names.sort_by(|a, b| a.0.cmp(&b.0));
    let names = unique_upstream_names(&upstream_names);
    let overrides: std::collections::HashMap<(String, String), _> = db
        .list_model_overrides()?
        .into_iter()
        .map(|over| ((over.upstream_id.clone(), over.model_id.clone()), over))
        .collect();
    Ok(entries
        .into_iter()
        .filter(|entry| {
            !overrides
                .get(&(entry.upstream_id.clone(), entry.model_id.clone()))
                .map(|over| over.hidden)
                .unwrap_or(false)
        })
        .map(|entry| {
            let upstream_name = names
                .get(&entry.upstream_id)
                .cloned()
                .unwrap_or_else(|| entry.upstream_id.clone());
            let display_name = overrides
                .get(&(entry.upstream_id.clone(), entry.model_id.clone()))
                .and_then(|over| over.display_name.clone())
                .or(entry.display_name);
            CatalogViewEntry {
                reference: format!("{upstream_name}/{}", entry.model_id),
                upstream_id: entry.upstream_id,
                upstream_name,
                model_id: entry.model_id,
                display_name,
                one_m: entry.one_m,
                window: entry.window,
                endpoint_ids: entry.endpoint_ids,
                sources: entry.sources,
            }
        })
        .collect())
}

/// 按模型引用查一条；查不到返回 `None`（调用方必须报错，不回落）。
pub fn resolve_reference<'a>(
    entries: &'a [CatalogViewEntry],
    reference: &str,
) -> Option<&'a CatalogViewEntry> {
    entries.iter().find(|entry| entry.reference == reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ClaudeStackModel, ProviderMeta};
    use serde_json::json;

    fn provider(name: &str, settings: Value, meta: Option<ProviderMeta>) -> Provider {
        let mut provider =
            Provider::with_id(format!("id-{name}"), name.to_string(), settings, None);
        provider.meta = meta;
        provider
    }

    #[test]
    fn claude_stack_models_are_the_source() {
        let meta = ProviderMeta {
            stack_models: Some(vec![
                ClaudeStackModel {
                    model: "kimi-k3".into(),
                    display_name: Some("Kimi K3".into()),
                    one_m: false,
                },
                ClaudeStackModel {
                    model: "qwen3.8-max[1M]".into(),
                    display_name: None,
                    one_m: true,
                },
            ]),
            ..Default::default()
        };
        let p = provider("CTYun", json!({"env": {}}), Some(meta));
        let models = raw_models(&AppType::Claude, &p);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model, "kimi-k3");
        assert_eq!(models[0].display.as_deref(), Some("Kimi K3"));
        assert_eq!(models[1].model, "qwen3.8-max");
        assert!(models[1].one_m);
    }

    #[test]
    fn claude_falls_back_to_the_four_slots_and_dedupes() {
        let p = provider(
            "CTYun",
            json!({"env": {
                "ANTHROPIC_MODEL": "deepseek-v4.1-flash-1017",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "deepseek-v4.1-flash-1017[1M]",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "qwen3.8-max",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "qwen3.8-omni-flash",
                "ANTHROPIC_DEFAULT_FABLE_MODEL": "qwen3.8-max"
            }}),
            None,
        );
        let models = raw_models(&AppType::Claude, &p);
        // 四个不同名字 + 1M 归一：deepseek 一条、qwen3.8-max 一条（两档撞了）
        let names: Vec<&str> = models.iter().map(|m| m.model.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "deepseek-v4.1-flash-1017",
                "qwen3.8-max",
                "qwen3.8-omni-flash"
            ]
        );
        assert!(models[0].one_m, "有一档标了 1M，归一后按 1M");
    }

    #[test]
    fn desktop_routes_are_the_source() {
        let meta: ProviderMeta = serde_json::from_value(json!({
            "claudeDesktopMode": "proxy",
            "claudeDesktopModelRoutes": {
                "claude-sonnet-5": {"model": "gpt-6.1-terra", "labelOverride": "gpt-6.1-terra", "supports1m": true},
                "claude-haiku-4-5": {"model": "gpt-6-luna", "labelOverride": "gpt-6-luna"}
            }
        }))
        .expect("meta");
        let p = provider("Codex", json!({}), Some(meta));
        let models = raw_models(&AppType::ClaudeDesktop, &p);
        let names: Vec<&str> = models.iter().map(|m| m.model.as_str()).collect();
        assert_eq!(names, vec!["gpt-6.1-terra", "gpt-6-luna"]);
        assert!(models[0].one_m);
    }

    #[test]
    fn codex_reads_the_top_level_model_from_its_config() {
        let p = provider(
            "自定义网关",
            json!({
                "config": "model = \"deepseek-v4.1-flash-1017\"\n",
                "auth": {}
            }),
            None,
        );
        let models = raw_models(&AppType::Codex, &p);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model, "deepseek-v4.1-flash-1017");
    }

    #[test]
    fn codex_prefers_the_model_catalog_in_settings() {
        let p = provider(
            "自定义网关",
            json!({
                "config": "model = \"top-level\"\n",
                "modelCatalog": {"models": [{"model": "kimi-k3"}, {"model": "gpt-6.1-sol[1M]"}]}
            }),
            None,
        );
        let models = raw_models(&AppType::Codex, &p);
        let names: Vec<&str> = models.iter().map(|m| m.model.as_str()).collect();
        assert_eq!(names, vec!["kimi-k3", "gpt-6.1-sol"]);
        assert!(models[1].one_m);
    }

    #[test]
    fn merge_keeps_one_row_per_endpoint_and_model() {
        let mut entries = BTreeMap::new();
        merge(
            &mut entries,
            "up-1",
            "ep-1",
            "claude:CTYun",
            RawModel {
                model: "kimi-k3".into(),
                display: None,
                one_m: false,
                window: Some(200_000),
            },
        );
        merge(
            &mut entries,
            "up-1",
            "ep-2",
            "claude-desktop:CTYun",
            RawModel {
                model: "kimi-k3".into(),
                display: Some("Kimi K3".into()),
                one_m: true,
                window: None,
            },
        );
        let values: Vec<&CatalogEntry> = entries.values().collect();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].display_name.as_deref(), Some("Kimi K3"));
        assert!(values[0].one_m);
        assert_eq!(values[0].window, Some(200_000));
        assert_eq!(values[0].sources.len(), 2);
        assert_eq!(values[0].endpoint_ids, vec!["ep-1", "ep-2"]);
    }

    #[test]
    fn duplicate_upstream_names_get_a_suffix() {
        let names = unique_upstream_names(&[
            ("up-1".into(), "CTYun".into()),
            ("up-2".into(), "Kimi".into()),
            ("up-3".into(), "CTYun".into()),
        ]);
        assert_eq!(names["up-1"], "CTYun");
        assert_eq!(names["up-2"], "Kimi");
        assert_eq!(names["up-3"], "CTYun（2）");
    }

    #[test]
    fn references_round_trip() {
        let entries = vec![CatalogViewEntry {
            reference: "CTYun/deepseek-v4.1-flash-1017".into(),
            upstream_id: "up-1".into(),
            upstream_name: "CTYun".into(),
            model_id: "deepseek-v4.1-flash-1017".into(),
            display_name: None,
            one_m: true,
            window: Some(200_000),
            endpoint_ids: vec!["ep-1".into()],
            sources: vec!["claude:CTYun".into()],
        }];
        assert_eq!(
            resolve_reference(&entries, "CTYun/deepseek-v4.1-flash-1017")
                .map(|entry| entry.upstream_id.as_str()),
            Some("up-1")
        );
        assert!(resolve_reference(&entries, "CTYun/not-there").is_none());
    }

    /// 真实库副本验证：
    /// `CCS_VERIFY_DB=/path/to/copy.db cargo test --lib gateway_catalog -- --ignored --nocapture`
    #[test]
    #[ignore = "手动：用真实库副本验证，需 CCS_VERIFY_DB"]
    fn manual_derive_against_real_db_copy() -> Result<(), crate::error::AppError> {
        let Ok(path) = std::env::var("CCS_VERIFY_DB") else {
            return Ok(());
        };
        let db = Database::open_and_migrate_for_test(std::path::Path::new(&path))?;
        let version: i32 = {
            let conn = crate::database::lock_conn!(db.conn);
            conn.query_row("PRAGMA user_version", [], |row| row.get(0))?
        };
        assert_eq!(version, crate::database::SCHEMA_VERSION);
        let entries = derive_entries(&db).expect("derive");
        let upstreams: std::collections::BTreeSet<&str> =
            entries.iter().map(|e| e.upstream_id.as_str()).collect();
        let endpoints: std::collections::BTreeSet<&str> = entries
            .iter()
            .flat_map(|e| e.endpoint_ids.iter().map(String::as_str))
            .collect();
        println!(
            "catalog: {} models across {} upstreams / {} endpoints",
            entries.len(),
            upstreams.len(),
            endpoints.len()
        );
        for entry in &entries {
            println!(
                "  {} {} endpoints={:?} one_m={} window={:?} sources={:?}",
                entry.upstream_id,
                entry.model_id,
                entry.endpoint_ids,
                entry.one_m,
                entry.window,
                entry.sources
            );
        }
        let view = catalog_view(&db).expect("view");
        let mut references = std::collections::HashSet::new();
        for entry in &view {
            assert!(
                references.insert(entry.reference.clone()),
                "duplicate reference: {}",
                entry.reference
            );
        }
        println!("references: {} unique", references.len());
        assert_eq!(view.len(), entries.len());
        assert!(!entries.is_empty());
        Ok(())
    }
}
