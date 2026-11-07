//! 网关模型目录的派生（P1 切片的纯逻辑部分）。
//!
//! 原则：**不重写模型推导**。claude / claude-desktop / codex 直接调用
//! [`crate::mode::stack`] 里已有的三个函数（`[1M]` 剥标记、显示名、窗口规则都在里面）；
//! 其余 app 的推导在后续切片补，调用方用 [`supports`] 判断覆盖范围。
//!
//! 目录的键是（端点, 规范模型名）：同一个上游被多个 app 引用时只出现一次，
//! `sources` 记下是哪些 app 的行贡献的。

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
pub(crate) struct RawModel {
    pub(crate) model: String,
    pub(crate) display: Option<String>,
    pub(crate) one_m: bool,
    pub(crate) window: Option<u64>,
}

/// 一个 provider 行贡献的模型清单。推导全部来自现有函数，这里只做 `[1M]` 归一。
pub(crate) fn raw_models(app: &AppType, provider: &Provider) -> Vec<RawModel> {
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

/// 目录缓存的版本号：写 provider / 上游绑定 / 每 app 配置 / 人工覆盖时由
/// [`invalidate_catalog_cache`] 加一。**不能**用 `database::db_write_epoch` 失效——
/// 请求日志和用量统计也在写库，会每秒失效一次。
static CATALOG_CACHE_EPOCH: AtomicU64 = AtomicU64::new(0);

/// 最近一次目录（按库文件路径键）。内存库（测试）不走缓存。
static CATALOG_CACHE: Mutex<Option<(String, u64, Vec<CatalogViewEntry>)>> = Mutex::new(None);

/// 目录依赖的数据变了：让缓存失效。写入点见 `save_provider` / `delete_provider` /
/// `reconcile_provider_bindings` / `gateway_config` 的写方法。
pub fn invalidate_catalog_cache() {
    CATALOG_CACHE_EPOCH.fetch_add(1, Ordering::Relaxed);
}

/// 带缓存的目录：文件库走缓存，内存库（测试）每次重算。
///
/// 计算完再读版本号存进缓存：派生过程会顺带对账绑定，对账期间的写入也算进这一次，
/// 不会一存就过期。
pub fn catalog_view_cached(db: &Database) -> Result<Vec<CatalogViewEntry>, AppError> {
    let Some(path) = db.file_path() else {
        return catalog_view(db);
    };
    let key = path.to_string_lossy().to_string();
    let cached = {
        let guard = CATALOG_CACHE
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        guard
            .as_ref()
            .filter(|(cached_key, _, _)| cached_key == &key)
            .map(|(_, epoch, entries)| (*epoch, entries.clone()))
    };
    if let Some((epoch, entries)) = cached {
        if epoch == CATALOG_CACHE_EPOCH.load(Ordering::Relaxed) {
            return Ok(entries);
        }
    }
    let entries = catalog_view(db)?;
    let epoch = CATALOG_CACHE_EPOCH.load(Ordering::Relaxed);
    *CATALOG_CACHE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = Some((key, epoch, entries.clone()));
    Ok(entries)
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

    // 一家上游可能有多条端点（协议不同）；发现结果没有 provider 行可依，按整家上游的
    // 全部端点记，路由时再按协议挑。
    let mut endpoints_by_upstream: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (endpoint_id, upstream_id) in &endpoint_upstream {
        endpoints_by_upstream
            .entry(upstream_id.clone())
            .or_default()
            .push(endpoint_id.clone());
    }

    // 2. 逐 app 读 provider 行，用现有推导函数取模型，按（端点, 模型）合并。
    let mut entries: BTreeMap<(String, String), CatalogEntry> = BTreeMap::new();
    // 上游 → 接进来的 app（`app:providerName`）。上游发现的模型没有 provider 行，
    // 但路由 / 发布都是按这个来源反查 provider 的，所以要记下这家上游已绑定的 app。
    let mut sources_by_upstream: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
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
            sources_by_upstream
                .entry(upstream_id.clone())
                .or_default()
                .insert(source.clone());
            for raw in raw_models(&app, &provider) {
                merge(&mut entries, upstream_id, endpoint_id, &source, raw);
            }
        }
    }

    // 3. 合并 `/v1/models` 的发现结果：provider 行里没有的模型也进目录（默认被 hidden
    //    覆盖挡住，用户点「加入」才可见）。provider 行已经贡献过的模型以派生值为准。
    for (upstream_id, model_id, display_name) in discovered_models(db)? {
        let key = (upstream_id.clone(), model_id.clone());
        if entries.contains_key(&key) {
            continue;
        }
        let Some(endpoint_ids) = endpoints_by_upstream.get(&upstream_id) else {
            // 没有端点就没法路由；只保留一张孤立的发现表没有意义。
            continue;
        };
        entries.insert(
            key,
            CatalogEntry {
                upstream_id: upstream_id.clone(),
                endpoint_ids: endpoint_ids.clone(),
                model_id,
                display_name,
                one_m: false,
                window: None,
                sources: sources_by_upstream
                    .get(&upstream_id)
                    .map(|sources| sources.iter().cloned().collect())
                    .unwrap_or_default(),
            },
        );
    }

    Ok(entries.into_values().collect())
}

/// `/v1/models` 的发现结果：`(上游, 模型, 显示名)`。
///
/// 表不存在时（老库还没跑到 v29）当作没有发现结果：目录派生是热路径，不能因为一张
/// 辅助表把整份目录读挂。
fn discovered_models(db: &Database) -> Result<Vec<(String, String, Option<String>)>, AppError> {
    let conn = crate::database::lock_conn!(db.conn);
    let mut stmt = match conn
        .prepare("SELECT upstream_id, model_id, display_name FROM gateway_upstream_models")
    {
        Ok(stmt) => stmt,
        Err(error) => {
            log::warn!("读取上游模型发现表失败（按空处理）: {error}");
            return Ok(Vec::new());
        }
    };
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get(2)?,
        ))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// 目录 + 每条的上游显示名与模型引用。上游名来自 `upstreams.name`，重名按出现顺序加序号。
///
/// 被人工覆盖隐藏的条目不返回（`catalog_view_including_hidden` 才会）。
pub fn catalog_view(db: &Database) -> Result<Vec<CatalogViewEntry>, AppError> {
    catalog_view_with(db, false)
}

/// 带隐藏条目的目录：供应商编辑弹窗的左栏要看「上游有、但还没启用」的模型。
pub fn catalog_view_including_hidden(db: &Database) -> Result<Vec<CatalogViewEntry>, AppError> {
    catalog_view_with(db, true)
}

fn catalog_view_with(
    db: &Database,
    include_hidden: bool,
) -> Result<Vec<CatalogViewEntry>, AppError> {
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
            include_hidden
                || !overrides
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

    #[test]
    fn file_backed_catalog_cache_serves_until_a_write_invalidates_it() {
        let path = std::env::temp_dir().join(format!(
            "ccs-catalog-cache-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        {
            let db = Database::open_and_migrate_for_test(&path).expect("open");
            let provider = provider(
                "CTYun",
                json!({"env": {"ANTHROPIC_BASE_URL": "https://cache-test.example/v1", "ANTHROPIC_MODEL": "m1"}}),
                None,
            );
            db.save_provider("claude", &provider).expect("save");

            let first = catalog_view_cached(&db).expect("first");
            assert_eq!(first.len(), 1, "第一条 provider 应该只有一个模型");
            // 再读一次走缓存：结果一致
            assert_eq!(catalog_view_cached(&db).expect("second"), first);

            // 写一家新的：缓存必须失效
            let mut provider2 = provider.clone();
            provider2.id = "p-2".into();
            provider2.name = "Kimi".into();
            provider2.settings_config = json!({"env": {"ANTHROPIC_BASE_URL": "https://kimi-cache.example", "ANTHROPIC_MODEL": "kimi-k3"}});
            db.save_provider("claude", &provider2).expect("save2");
            let after = catalog_view_cached(&db).expect("after");
            assert_eq!(after.len(), 2, "写 provider 后缓存要失效");
        }
        let _ = std::fs::remove_file(&path);
    }

    fn temp_catalog_db(tag: &str) -> (std::path::PathBuf, Database) {
        let path = std::env::temp_dir().join(format!(
            "ccs-catalog-{tag}-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let db = Database::open_and_migrate_for_test(&path).expect("open");
        (path, db)
    }

    /// 上游 `/v1/models` 拉到的候选要进目录（带这家上游已绑定 app 的来源），
    /// 且默认被隐藏挡住；「加入」后可见。
    #[test]
    fn discovered_models_join_the_catalog_behind_a_hidden_override() {
        let (path, db) = temp_catalog_db("discovered");
        let p = provider(
            "CTYun",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://ctyun.example/v1",
                "ANTHROPIC_MODEL": "m1"
            }}),
            None,
        );
        db.save_provider("claude", &p).expect("save");
        let upstream_id = derive_entries(&db).expect("derive")[0].upstream_id.clone();

        let (fetched, added) = crate::services::gateway_model_sync::store_discovered_models(
            &db,
            &upstream_id,
            &["m1".to_string(), "m2".to_string()],
        )
        .expect("store");
        assert_eq!(fetched, 2);
        assert_eq!(
            added,
            vec!["m2".to_string()],
            "m1 已经在 provider 行里，只有 m2 算新候选"
        );

        let visible = catalog_view(&db).expect("view");
        assert_eq!(
            visible
                .iter()
                .map(|entry| entry.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["m1"],
            "候选默认隐藏，不进可见目录"
        );

        let all = catalog_view_including_hidden(&db).expect("view all");
        let candidate = all
            .iter()
            .find(|entry| entry.model_id == "m2")
            .expect("m2 应该在含隐藏条目的目录里");
        assert_eq!(candidate.sources, vec!["claude:CTYun".to_string()]);
        assert!(!candidate.endpoint_ids.is_empty(), "候选要能路由");

        // 加入 = 写 hidden=false 的覆盖 → 可见；再刷一次不能把状态打回未启用
        db.set_model_override(&upstream_id, "m2", None, false)
            .expect("enable");
        assert!(catalog_view(&db)
            .expect("view")
            .iter()
            .any(|entry| entry.model_id == "m2"));
        let (_, added) = crate::services::gateway_model_sync::store_discovered_models(
            &db,
            &upstream_id,
            &["m1".to_string(), "m2".to_string()],
        )
        .expect("store again");
        assert!(added.is_empty(), "已有候选不重复算新增");
        assert!(catalog_view(&db)
            .expect("view")
            .iter()
            .any(|entry| entry.model_id == "m2"));

        drop(db);
        let _ = std::fs::remove_file(&path);
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
        let started = std::time::Instant::now();
        let entries = derive_entries(&db).expect("derive");
        let derive_elapsed = started.elapsed();
        let started = std::time::Instant::now();
        let _ = catalog_view(&db).expect("view warm");
        let view_elapsed = started.elapsed();
        let started = std::time::Instant::now();
        let _ = catalog_view_cached(&db).expect("cached warm");
        let cached_cold = started.elapsed();
        let started = std::time::Instant::now();
        let _ = catalog_view_cached(&db).expect("cached hot");
        let cached_hot = started.elapsed();
        println!(
            "timing: derive={derive_elapsed:?} view(second)={view_elapsed:?} cached(cold)={cached_cold:?} cached(hot)={cached_hot:?}"
        );
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
