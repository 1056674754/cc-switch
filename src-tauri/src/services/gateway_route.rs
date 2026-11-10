//! 按模型选上游（P2 的路由部分）。
//!
//! 客户端请求里的 `model` 就是路由键：
//! - 形如 `<供应商名>/<模型 id>` → 发往那家供应商，上游模型名换成 `/` 后面那段
//! - 其它（普通模型名）→ 交给现有的默认路由，不在这一层猜
//!
//! 解析直接复用目录（[`crate::services::gateway_catalog::catalog_view`]）和
//! [`crate::mode::stack::StackTarget`]：后者已经是「单家、不转移」的请求形状，
//! 代理管线里 `RequestContext` 对它的处理正是我们要的语义。

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::mode::stack::StackTarget;
use crate::provider::Provider;
use crate::services::gateway_catalog::{resolve_reference, CatalogViewEntry};

/// 把 `<供应商名>/<模型 id>` 解析成转发目标。不是引用形状时返回 `None`。
///
/// 引用形状但查不到（供应商不存在、模型不在目录、这家 app 没有绑定该上游）返回 `Err`：
/// 用户显式点名了模型，不能静默落到别的上游。
pub fn resolve_reference_target(
    db: &Database,
    app: &AppType,
    model: &str,
) -> Result<Option<StackTarget>, AppError> {
    let Some((supplier, upstream_model)) = model.split_once('/') else {
        return Ok(None);
    };
    let supplier = supplier.trim();
    let upstream_model =
        crate::proxy::model_mapper::strip_one_m_suffix_for_upstream(upstream_model.trim())
            .trim()
            .to_string();
    if supplier.is_empty() || upstream_model.is_empty() {
        return Ok(None);
    }

    let catalog = crate::services::gateway_catalog::catalog_view_cached(db)?;
    let reference = format!("{supplier}/{upstream_model}");
    let Some(entry) = crate::services::gateway_catalog::resolve_reference(&catalog, &reference)
    else {
        // 供应商名对得上、模型对不上 → 用户点名了一个不存在的模型，报错不回落。
        // 供应商名也对不上 → 可能是客户端自己的别名（如 `anthropic/claude-x`），走默认路由。
        let known_supplier = catalog.iter().any(|entry| entry.upstream_name == supplier);
        if !known_supplier {
            return Ok(None);
        }
        return Err(AppError::Config(format!(
            "模型 {reference} 不在目录里（供应商 {supplier} 没有发布这个模型）"
        )));
    };

    // 目录来源里有这一家的名字，用它反查 provider 行。同一个上游在 claude 与
    // claude-desktop 下是两条 provider 行，各自有自己的 settings。
    let source_prefix = format!("{}:", app.as_str());
    let provider_name = entry
        .sources
        .iter()
        .find_map(|source| source.strip_prefix(&source_prefix));
    let provider = match provider_name {
        Some(provider_name) => {
            let providers = db.get_all_providers(app.as_str())?;
            let mut candidates = providers
                .into_iter()
                .filter(|(_, provider)| provider.name == provider_name);
            let Some((_, provider)) = candidates.next() else {
                return Err(AppError::Config(format!(
                    "模型 {} 的供应商 {provider_name} 在 {} 里已不存在",
                    model,
                    app.as_str()
                )));
            };
            provider
        }
        // 接入端选模型就是绑定：这家 app 没走过「给 app 加供应商」也能选目录里的模型，
        // 按上游端点和密钥临时拼一个行。不认识的上游协议还是会报原来的错。
        None if matches!(app, AppType::Codex) => {
            match synthetic_codex_provider(db, entry)? {
                Some(provider) => provider,
                None => {
                    return Err(AppError::Config(format!(
                        "模型 {} 不属于 {}：这家 app 没有绑定该上游",
                        model,
                        app.as_str()
                    )))
                }
            }
        }
        None if matches!(app, AppType::Claude | AppType::ClaudeDesktop) => {
            match synthetic_claude_provider(db, entry)? {
                Some(provider) => provider,
                None => {
                    return Err(AppError::Config(format!(
                        "模型 {} 不属于 {}：这家 app 没有绑定该上游",
                        model,
                        app.as_str()
                    )))
                }
            }
        }
        None => {
            return Err(AppError::Config(format!(
                "模型 {} 不属于 {}：这家 app 没有绑定该上游",
                model,
                app.as_str()
            )))
        }
    };
    Ok(Some(StackTarget {
        provider,
        upstream_model,
        original_model: model.to_string(),
    }))
}



/// 凭证可能是 `{file:~/.secrets/xxx}` 这种引用（opencode 那套写法）：发请求前读真实值。
fn expand_secret(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let Some(rest) = raw.strip_prefix("{file:") else {
        return Some(raw.to_string());
    };
    let path = rest.strip_suffix('}')?.trim();
    let path = match path.strip_prefix("~/") {
        Some(relative) => dirs::home_dir()?.join(relative),
        None => std::path::PathBuf::from(path),
    };
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// 上游端点按 Codex 能用的协议排优先级；其余协议（各家私有托管端点）这里不管。
fn codex_endpoint_rank(protocol: &str) -> Option<u8> {
    match protocol {
        "openai_responses" => Some(0),
        "openai_chat" => Some(1),
        "anthropic" => Some(2),
        _ => None,
    }
}


/// Claude / Claude Desktop 用环境变量表达上游：只认 Anthropic Messages 协议的端点。
/// 凭证同样优先用目录来源那家 provider 的（同一端点可能挂着旧号和新号）。
pub(crate) fn synthetic_claude_provider(
    db: &Database,
    entry: &CatalogViewEntry,
) -> Result<Option<Provider>, AppError> {
    let endpoints: Vec<(String, String)> = {
        let conn = crate::database::lock_conn!(db.conn);
        let mut stmt =
            conn.prepare("SELECT id, base_url FROM upstream_endpoints WHERE id = ?1")?;
        let mut found = Vec::new();
        for endpoint_id in &entry.endpoint_ids {
            if let Ok(row) = stmt.query_row([endpoint_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                found.push(row);
            }
        }
        found
    };
    if endpoints.is_empty() {
        return Ok(None);
    }
    let mut chosen: Option<(String, String)> = None;
    {
        let conn = crate::database::lock_conn!(db.conn);
        let mut protocol_stmt =
            conn.prepare("SELECT protocol FROM upstream_endpoints WHERE id = ?1")?;
        for (endpoint_id, base_url) in &endpoints {
            let Ok(protocol) = protocol_stmt.query_row([endpoint_id], |row| row.get::<_, String>(0))
            else {
                continue;
            };
            if protocol != "anthropic" {
                continue;
            }
            for source in &entry.sources {
                let Some((source_app, source_name)) = source.split_once(':') else {
                    continue;
                };
                let Ok(mut stmt) = conn.prepare(
                    "SELECT b.provider_id FROM provider_bindings b
                     JOIN providers p ON p.id = b.provider_id AND p.app_type = b.app_type
                     WHERE b.app_type = ?1 AND p.name = ?2 AND b.endpoint_id = ?3",
                ) else {
                    continue;
                };
                let provider_ids: Vec<String> = stmt
                    .query_map([source_app, source_name, endpoint_id.as_str()], |row| {
                        row.get::<_, String>(0)
                    })?
                    .flatten()
                    .collect();
                for provider_id in provider_ids {
                    let secret: Option<String> = conn
                        .query_row(
                            "SELECT secret FROM upstream_credentials
                             WHERE endpoint_id = ?1 AND app_type = ?2 AND provider_id = ?3",
                            [endpoint_id.as_str(), source_app, provider_id.as_str()],
                            |row| row.get::<_, String>(0),
                        )
                        .ok()
                        .filter(|secret| !secret.trim().is_empty());
                    if let Some(secret) = secret {
                        chosen = Some((base_url.clone(), secret));
                        break;
                    }
                }
                if chosen.is_some() {
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        if chosen.is_none() {
            for (endpoint_id, base_url) in &endpoints {
                let Ok(protocol) =
                    protocol_stmt.query_row([endpoint_id], |row| row.get::<_, String>(0))
                else {
                    continue;
                };
                if protocol != "anthropic" {
                    continue;
                }
                let secret: Option<String> = conn
                    .query_row(
                        "SELECT secret FROM upstream_credentials
                         WHERE endpoint_id = ?1 AND secret IS NOT NULL
                         ORDER BY (app_type = 'claude') DESC
                         LIMIT 1",
                        [endpoint_id.as_str()],
                        |row| row.get::<_, String>(0),
                    )
                    .ok()
                    .filter(|secret| !secret.trim().is_empty());
                if let Some(secret) = secret {
                    chosen = Some((base_url.clone(), secret));
                    break;
                }
            }
        }
    }
    let Some((base_url, secret)) = chosen else {
        return Ok(None);
    };
    let Some(secret) = expand_secret(&secret) else {
        return Ok(None);
    };
    let model = entry.model_id.clone();
    let settings = serde_json::json!({
        "env": {
            "ANTHROPIC_BASE_URL": base_url,
            "ANTHROPIC_AUTH_TOKEN": secret,
            "ANTHROPIC_MODEL": model,
        }
    });
    Ok(Some(Provider::with_id(
        format!("gateway-{}", entry.upstream_id),
        entry.upstream_name.clone(),
        settings,
        None,
    )))
}

/// 给「应用配置里点名、但这家 app 没绑过」的上游临时拼一个 Codex 供应商行：
/// 端点取该上游 Codex 能用的那条，密钥取该端点已存的凭证（优先这个 app 自己的）。
/// 不写库：请求路由完就丢，绑定关系由应用模型配置本身表达。
pub(crate) fn synthetic_codex_provider(
    db: &Database,
    entry: &CatalogViewEntry,
) -> Result<Option<Provider>, AppError> {
    let endpoints: Vec<(String, String, String)> = {
        let conn = crate::database::lock_conn!(db.conn);
        let mut stmt = conn
            .prepare("SELECT id, protocol, base_url FROM upstream_endpoints WHERE id = ?1")?;
        let mut found = Vec::new();
        for endpoint_id in &entry.endpoint_ids {
            if let Ok(row) = stmt.query_row([endpoint_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            }) {
                found.push(row);
            }
        }
        found
    };
    let usable: Vec<(String, String, String)> = endpoints
        .into_iter()
        .filter(|(_, protocol, _)| codex_endpoint_rank(protocol).is_some())
        .collect();
    if usable.is_empty() {
        return Ok(None);
    }
    // 优先用目录来源里那家 provider 的凭证：同一端点可能挂着几家（旧号和能用的号），
    // 目录来源就是这条模型真正在用的那家。
    let mut chosen: Option<(String, String, String, String)> = None;
    {
        let conn = crate::database::lock_conn!(db.conn);
        for source in &entry.sources {
            let Some((source_app, source_name)) = source.split_once(':') else {
                continue;
            };
            let mut stmt = conn.prepare(
                "SELECT b.endpoint_id, b.provider_id, e.protocol, e.base_url
                 FROM provider_bindings b
                 JOIN providers p ON p.id = b.provider_id AND p.app_type = b.app_type
                 JOIN upstream_endpoints e ON e.id = b.endpoint_id
                 WHERE b.app_type = ?1 AND p.name = ?2",
            )?;
            let rows = stmt.query_map([source_app, source_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows.flatten() {
                let (endpoint_id, provider_id, protocol, base_url) = row;
                if codex_endpoint_rank(&protocol).is_none()
                    || !entry.endpoint_ids.iter().any(|id| id == &endpoint_id)
                {
                    continue;
                }
                let secret: Option<String> = conn
                    .query_row(
                        "SELECT secret FROM upstream_credentials
                         WHERE endpoint_id = ?1 AND app_type = ?2 AND provider_id = ?3",
                        [&endpoint_id, source_app, &provider_id],
                        |row| row.get::<_, String>(0),
                    )
                    .ok()
                    .filter(|secret| !secret.trim().is_empty());
                if let Some(secret) = secret {
                    chosen = Some((endpoint_id, protocol, base_url, secret));
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
    }
    let (_endpoint_id, protocol, base_url, secret) = match chosen {
        Some(found) => found,
        None => {
            let (endpoint_id, protocol, base_url) = usable
                .into_iter()
                .min_by_key(|(_, protocol, _)| {
                    codex_endpoint_rank(protocol).unwrap_or(u8::MAX)
                })
                .expect("usable 非空");
            let secret: Option<String> = {
                let conn = crate::database::lock_conn!(db.conn);
                let mut stmt = conn.prepare(
                    "SELECT secret FROM upstream_credentials
                     WHERE endpoint_id = ?1 AND secret IS NOT NULL
                     ORDER BY (app_type = 'codex') DESC
                     LIMIT 1",
                )?;
                stmt.query_row([&endpoint_id], |row| row.get::<_, String>(0))
                    .ok()
                    .filter(|secret| !secret.trim().is_empty())
            };
            let Some(secret) = secret else {
                return Ok(None);
            };
            (endpoint_id, protocol, base_url, secret)
        }
    };
    let Some(secret) = expand_secret(&secret) else {
        return Ok(None);
    };
    let wire_api = match protocol.as_str() {
        "openai_chat" => "chat",
        "openai_responses" => "responses",
        // Anthropic 上游走代理的 Responses→Messages 转换，wire_api 只是占位。
        _ => "chat",
    };
    let model = entry.model_id.clone();
    let display = entry
        .display_name
        .clone()
        .unwrap_or_else(|| entry.model_id.clone());
    let config = format!(
        "model = {model:?}\nmodel_provider = \"custom\"\n\n[model_providers]\n\n[model_providers.custom]\nname = {name:?}\nbase_url = {base_url:?}\nwire_api = {wire_api:?}\nrequires_openai_auth = true\n",
        name = entry.upstream_name,
    );
    let mut settings = serde_json::json!({
        "auth": { "OPENAI_API_KEY": secret },
        "config": config,
        "modelCatalog": { "models": [{
            "model": model,
            "displayName": display,
            "contextWindow": entry.window,
        }] },
    });
    if protocol == "anthropic" {
        settings["api_format"] = serde_json::json!("anthropic");
    }
    Ok(Some(Provider::with_id(
        format!("gateway-{}", entry.upstream_id),
        entry.upstream_name.clone(),
        settings,
        None,
    )))
}

/// 累加式 app（OpenCode 等）的客户端会把 provider 前缀吃掉，只发裸模型 id。
/// 在该 app 的网关模型配置里挑引用：同名模型唯一命中就选它；多家都发布时报歧义；
/// 一个都没配命中而目录里确实有这个模型时报「不在允许列表」；否则回落到默认模型。
pub fn pick_configured_reference(
    config: &crate::services::gateway_config::AppModelConfig,
    catalog: &[CatalogViewEntry],
    model: &str,
    app: &str,
) -> Result<Option<String>, AppError> {
    let bare = crate::proxy::model_mapper::strip_one_m_suffix_for_upstream(model.trim())
        .trim()
        .to_string();
    let mut references: Vec<&str> = Vec::new();
    for reference in std::iter::once(config.default.as_deref())
        .flatten()
        .chain(config.allowed.iter().map(String::as_str))
        .chain(config.slots.values().map(String::as_str))
    {
        if !references.contains(&reference) {
            references.push(reference);
        }
    }
    let configured: Vec<&CatalogViewEntry> = references
        .iter()
        .filter_map(|reference| resolve_reference(catalog, reference))
        .collect();
    let hits: Vec<&&CatalogViewEntry> = configured
        .iter()
        .filter(|entry| entry.model_id == bare)
        .collect();
    match hits.len() {
        1 => Ok(Some(hits[0].reference.clone())),
        n if n > 1 => Err(AppError::Config(format!(
            "模型 {model} 在多条引用里都存在，请用 <供应商>/<模型> 指定上游"
        ))),
        _ => {
            let published = catalog.iter().any(|entry| {
                entry.model_id == bare
                    && entry
                        .sources
                        .iter()
                        .any(|source| source.starts_with(&format!("{app}:")))
            });
            if published {
                return Err(AppError::Config(format!(
                    "模型 {model} 没有为此应用配置（不在默认模型 / 槽位 / 允许列表里）"
                )));
            }
            Ok(configured
                .iter()
                .find(|entry| config.default.as_deref() == Some(entry.reference.as_str()))
                .map(|entry| entry.reference.clone()))
        }
    }
}

/// 把裸模型 id 解析成 `<供应商>/<模型>` 再查上游（给累加式 app 的请求热路径用）。
pub fn resolve_configured_model(
    db: &Database,
    app: &AppType,
    model: &str,
) -> Result<Option<StackTarget>, AppError> {
    if model.trim().is_empty() {
        return Ok(None);
    }
    let Some(config) = db.get_app_model_config(app.as_str())? else {
        return Ok(None);
    };
    let catalog = crate::services::gateway_catalog::catalog_view_cached(db)?;
    let Some(reference) = pick_configured_reference(&config, &catalog, model, app.as_str())? else {
        return Ok(None);
    };
    resolve_reference_target(db, app, &reference)
}

/// Claude Desktop 的角色 route id（`claude-opus-5` 等）→ 应用配置里该槽位配的模型引用。
///
/// Desktop 的模型菜单发的是角色 route id，应用配置把 1+4 槽位按角色名存；槽位没配就
/// 返回 `None`，交给原来的 provider 映射表（老行为不变）。
pub(crate) fn desktop_slot_reference(
    db: &Database,
    model: &str,
) -> Result<Option<String>, AppError> {
    let Some(role) = crate::claude_desktop_config::role_for_route_id(model) else {
        return Ok(None);
    };
    let Some(config) = db.get_app_model_config(AppType::ClaudeDesktop.as_str())? else {
        return Ok(None);
    };
    Ok(config
        .slots
        .get(role)
        .map(|reference| reference.trim())
        .filter(|reference| !reference.is_empty())
        .map(str::to_string))
}

/// Desktop 请求里是角色 route id 时，按应用配置的槽位换成模型引用再解析上游。
/// 槽位没配返回 `None`（沿用 provider 映射表）。
pub(crate) fn resolve_desktop_role_target(
    db: &Database,
    model: &str,
) -> Result<Option<StackTarget>, AppError> {
    let Some(reference) = desktop_slot_reference(db, model)? else {
        return Ok(None);
    };
    resolve_reference_target(db, &AppType::ClaudeDesktop, &reference)
}

/// 应用模型配置里点过名的引用（默认 + 允许列表，去重保序）：发布和反解都用这个顺序。
pub(crate) fn desktop_config_references(
    config: &crate::services::gateway_config::AppModelConfig,
) -> Vec<&str> {
    let mut references: Vec<&str> = Vec::new();
    for reference in std::iter::once(config.default.as_deref())
        .flatten()
        .chain(config.allowed.iter().map(String::as_str))
    {
        if !references.contains(&reference) {
            references.push(reference);
        }
    }
    references
}

/// Claude Desktop 的模型配置引用发布出去的编码 id（`claude-fable-ccs-<数字>`）→ 配置里的引用。
///
/// 引用本身是 `供应商/模型` 形状，Desktop 的选择器不收（见
/// `controller::desktop_model_config_publications` 的注释），所以发布前编码、请求回来时在
/// 这里解回引用再按引用选上游。配置接管后 Stack 已经关掉，这段会先于 Stack 解析命中。
pub(crate) fn resolve_desktop_config_target(
    db: &Database,
    model: &str,
) -> Result<Option<StackTarget>, AppError> {
    let crate::mode::stack::Decoded::Code(code) =
        crate::mode::stack::decode(&AppType::ClaudeDesktop, model)
    else {
        return Ok(None);
    };
    let Some(config) = db.get_app_model_config(AppType::ClaudeDesktop.as_str())? else {
        return Ok(None);
    };
    let wanted = crate::mode::stack::desktop_id_from_code(code);
    for reference in desktop_config_references(&config) {
        if crate::mode::stack::desktop_config_id(reference) == wanted {
            return resolve_reference_target(db, &AppType::ClaudeDesktop, reference);
        }
    }
    Err(AppError::Config(format!(
        "Claude Desktop 的模型配置里没有 {model}（可能已被移除），请重新选择模型"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;
    use serde_json::json;

    #[test]
    fn file_secret_references_are_read_before_use() {
        assert_eq!(expand_secret("sk-plain"), Some("sk-plain".to_string()));
        assert_eq!(expand_secret("  "), None);
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("key.txt");
        std::fs::write(&file, "sk-from-file\n").expect("write key");
        assert_eq!(
            expand_secret(&format!("{{file:{}}}", file.display())),
            Some("sk-from-file".to_string())
        );
        assert_eq!(
            expand_secret(&format!("{{file:{}}}", dir.path().join("missing").display())),
            None
        );
    }

    fn claude_provider(id: &str, name: &str, base_url: &str, models: &[&str]) -> Provider {
        let mut env = json!({
            "ANTHROPIC_BASE_URL": base_url,
            "ANTHROPIC_AUTH_TOKEN": "sk-test",
            "ANTHROPIC_MODEL": models[0],
        });
        for (index, slot) in ["OPUS", "SONNET", "HAIKU", "FABLE"].iter().enumerate() {
            if let Some(model) = models.get(index + 1) {
                env[format!("ANTHROPIC_DEFAULT_{slot}_MODEL")] = json!(model);
            }
        }
        Provider::with_id(
            id.to_string(),
            name.to_string(),
            json!({ "env": env }),
            None,
        )
    }

    fn catalog_entry(reference: &str, model_id: &str) -> CatalogViewEntry {
        CatalogViewEntry {
            reference: reference.to_string(),
            upstream_id: "u1".to_string(),
            upstream_name: reference.split_once('/').unwrap().0.to_string(),
            model_id: model_id.to_string(),
            display_name: Some(model_id.to_string()),
            one_m: false,
            window: None,
            endpoint_ids: Vec::new(),
            sources: vec!["opencode:p".to_string()],
        }
    }

    fn app_config(
        default: Option<&str>,
        allowed: &[&str],
        slots: &[(&str, &str)],
    ) -> crate::services::gateway_config::AppModelConfig {
        crate::services::gateway_config::AppModelConfig {
            default: default.map(str::to_string),
            slots: slots
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            allowed: allowed.iter().map(|value| value.to_string()).collect(),
            routing: None,
        }
    }

    /// 真机烟测（默认忽略）：真实库副本里，给一个从没绑过这个 app 的供应商选模型，
    /// 路由能按上游端点 + 凭证临时拼出 Codex 行（接入端选模型即绑定）。
    ///
    /// 跑法：`CC_SWITCH_REAL_DB=~/.cc-switch/cc-switch.db cargo test --lib
    /// real_db_codex_resolves_model_without_provider_row -- --ignored --nocapture`
    #[test]
    #[ignore = "需要 CC_SWITCH_REAL_DB 指向真实 cc-switch.db 的副本"]
    #[serial_test::serial]
    fn real_db_codex_resolves_model_without_provider_row() {
        let source = std::env::var("CC_SWITCH_REAL_DB").expect("CC_SWITCH_REAL_DB");
        let home = tempfile::tempdir().expect("temp home");
        let config_dir = home.path().join(".cc-switch");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        std::fs::copy(&source, config_dir.join("cc-switch.db")).expect("copy real db");
        std::env::set_var("CC_SWITCH_TEST_HOME", home.path());
        let db = Database::init().expect("migrate db copy");

        let catalog = crate::services::gateway_catalog::catalog_view_cached(&db).expect("catalog");
        let entry = catalog
            .iter()
            .find(|entry| {
                entry.upstream_name == "Kimi"
                    && !entry
                        .sources
                        .iter()
                        .any(|source| source.starts_with("codex:"))
                    && !entry.endpoint_ids.is_empty()
            })
            .expect("real db has a Kimi supplier codex never bound");
        eprintln!(
            "codex 没绑过 {}，端点 {:?}",
            entry.reference, entry.endpoint_ids
        );

        let provider = synthetic_codex_provider(&db, entry)
            .expect("synthetic provider")
            .expect("端点和凭证都在，应该拼得出来");
        assert_eq!(provider.id, format!("gateway-{}", entry.upstream_id));
        assert!(provider.settings_config["auth"]["OPENAI_API_KEY"]
            .as_str()
            .is_some_and(|secret| !secret.is_empty()));
        assert!(provider.settings_config["config"]
            .as_str()
            .is_some_and(|config| config.contains("base_url")));
        assert_eq!(
            provider.settings_config["modelCatalog"]["models"][0]["model"],
            serde_json::json!(entry.model_id)
        );

        let target = resolve_reference_target(&db, &AppType::Codex, &entry.reference)
            .expect("resolve for codex")
            .expect("hit");
        assert!(target.provider.id.starts_with("gateway-"));
        assert_eq!(target.upstream_model, entry.model_id);
    }

    /// 真机烟测（默认忽略）：Claude 也一样，没绑过的供应商选中模型后按上游端点 + 凭证拼行。
    ///
    /// 跑法：`CC_SWITCH_REAL_DB=~/.cc-switch/cc-switch.db cargo test --lib
    /// real_db_claude_resolves_model_without_provider_row -- --ignored --nocapture`
    #[test]
    #[ignore = "需要 CC_SWITCH_REAL_DB 指向真实 cc-switch.db 的副本"]
    #[serial_test::serial]
    fn real_db_claude_resolves_model_without_provider_row() {
        let source = std::env::var("CC_SWITCH_REAL_DB").expect("CC_SWITCH_REAL_DB");
        let home = tempfile::tempdir().expect("temp home");
        let config_dir = home.path().join(".cc-switch");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        std::fs::copy(&source, config_dir.join("cc-switch.db")).expect("copy real db");
        std::env::set_var("CC_SWITCH_TEST_HOME", home.path());
        let db = Database::init().expect("migrate db copy");

        let catalog = crate::services::gateway_catalog::catalog_view_cached(&db).expect("catalog");
        let mut checked = 0usize;
        for entry in &catalog {
            if entry
                .sources
                .iter()
                .any(|source| source.starts_with("claude:") || source.starts_with("claude-desktop:"))
            {
                continue;
            }
            let Some(provider) = synthetic_claude_provider(&db, entry).expect("synthetic") else {
                continue;
            };
            let target = resolve_reference_target(&db, &AppType::Claude, &entry.reference)
                .expect("resolve for claude")
                .expect("hit");
            assert!(target.provider.id.starts_with("gateway-"));
            assert_eq!(target.upstream_model, entry.model_id);
            assert!(provider.settings_config["env"]["ANTHROPIC_BASE_URL"]
                .as_str()
                .is_some());
            eprintln!("claude 没绑过 {}，拼行成功", entry.reference);
            checked += 1;
            break;
        }
        assert!(checked > 0, "真实库应该有一条 claude 没绑过、但能拼行的供应商");
    }

    /// 真机烟测（默认忽略）：把真实 cc-switch.db 的**副本**放进临时 home，
    /// 让 `Database::init()` 走一遍真实迁移，再验证目录派生和引用解析。
    ///
    /// 跑法：`CC_SWITCH_REAL_DB=~/.cc-switch/cc-switch.db cargo test --lib
    /// real_db_copy_migrates -- --ignored --nocapture`
    #[test]
    #[ignore = "需要 CC_SWITCH_REAL_DB 指向真实 cc-switch.db 的副本"]
    #[serial_test::serial]
    fn real_db_copy_migrates_to_current_schema_and_derives_catalog() {
        let source = std::env::var("CC_SWITCH_REAL_DB").expect("CC_SWITCH_REAL_DB");
        let home = tempfile::tempdir().expect("temp home");
        let config_dir = home.path().join(".cc-switch");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        let db_path = config_dir.join("cc-switch.db");
        std::fs::copy(&source, &db_path).expect("copy real db");

        let user_version = |path: &std::path::Path| -> i32 {
            let conn = rusqlite::Connection::open(path).expect("open copy");
            conn.query_row("PRAGMA user_version", [], |row| row.get(0))
                .expect("user_version")
        };
        let before = user_version(&db_path);
        std::env::set_var("CC_SWITCH_TEST_HOME", home.path());
        let db = Database::init().expect("migrate db copy");
        let after = user_version(&db_path);
        eprintln!("user_version: {before} -> {after}");
        assert_eq!(after, crate::database::SCHEMA_VERSION);
        assert!(after >= before, "迁移不能降版本");

        let conn = rusqlite::Connection::open(&db_path).expect("reopen copy");
        let table_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='gateway_app_profiles'",
                [],
                |row| row.get(0),
            )
            .expect("gateway_app_profiles");
        assert_eq!(table_count, 1, "迁移后应该有 gateway_app_profiles");
        let upstreams: i64 = conn
            .query_row("SELECT count(*) FROM upstreams", [], |row| row.get(0))
            .expect("upstreams");
        eprintln!("upstreams in real db: {upstreams}");
        assert!(upstreams > 0, "真实库应该有供应商");

        let catalog = crate::services::gateway_catalog::catalog_view_cached(&db)
            .expect("derive catalog from real db");
        eprintln!("catalog entries: {}", catalog.len());
        assert!(!catalog.is_empty(), "真实库的目录不该是空的");

        let entry = catalog
            .iter()
            .find(|entry| {
                entry
                    .sources
                    .iter()
                    .any(|source| source.starts_with("claude:"))
            })
            .expect("real db has a claude-bound supplier");
        let target = resolve_reference_target(&db, &AppType::Claude, &entry.reference)
            .expect("resolve against real catalog")
            .expect("hit");
        assert!(!target.provider.id.is_empty());
        eprintln!(
            "resolved {} -> provider {} model {}",
            entry.reference, target.provider.id, target.upstream_model
        );

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    #[test]
    fn bare_model_picks_the_unique_configured_reference() {
        let catalog = vec![
            catalog_entry("CTYun/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(Some("Kimi/gpt-5"), &["CTYun/kimi-k3"], &[]);
        let picked =
            pick_configured_reference(&config, &catalog, "kimi-k3", "opencode").expect("pick");
        assert_eq!(picked.as_deref(), Some("CTYun/kimi-k3"));

        // 没配过的模型名回落到默认引用
        let picked = pick_configured_reference(&config, &catalog, "unknown-model", "opencode")
            .expect("fallback");
        assert_eq!(picked.as_deref(), Some("Kimi/gpt-5"));
    }

    #[test]
    fn bare_model_ambiguity_and_allowlist_errors_are_reported() {
        let catalog = vec![
            catalog_entry("CTYun/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(None, &["CTYun/kimi-k3", "Kimi/kimi-k3"], &[]);
        let error =
            pick_configured_reference(&config, &catalog, "kimi-k3", "opencode").unwrap_err();
        assert!(error.to_string().contains("指定上游"), "{error}");

        // 目录里有、但应用没配 -> 报不在允许列表，不静默换模型
        let catalog = vec![
            catalog_entry("CTYun/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(Some("Kimi/gpt-5"), &[], &[]);
        let error =
            pick_configured_reference(&config, &catalog, "kimi-k3", "opencode").unwrap_err();
        assert!(error.to_string().contains("没有为此应用配置"), "{error}");
    }

    #[test]
    fn slot_models_are_configured_references_too() {
        let catalog = vec![
            catalog_entry("CTYun/opus-x", "opus-x"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(Some("Kimi/gpt-5"), &[], &[("opus", "CTYun/opus-x")]);
        let picked =
            pick_configured_reference(&config, &catalog, "opus-x", "opencode").expect("pick");
        assert_eq!(picked.as_deref(), Some("CTYun/opus-x"));
    }

    #[test]
    fn reference_resolves_to_that_supplier_and_model() {
        let db = Database::memory().expect("memory db");
        db.save_provider(
            "claude",
            &claude_provider(
                "p-ctyun",
                "CTYun",
                "https://ai.ctaigw.cn/v1",
                &["deepseek-v4.1-flash-1017", "qwen3.8-max"],
            ),
        )
        .expect("save");
        db.save_provider(
            "claude",
            &claude_provider(
                "p-kimi",
                "Kimi",
                "https://api.kimi.com/coding",
                &["kimi-k3"],
            ),
        )
        .expect("save");

        let target =
            resolve_reference_target(&db, &AppType::Claude, "CTYun/deepseek-v4.1-flash-1017")
                .expect("resolve")
                .expect("hit");
        assert_eq!(target.provider.id, "p-ctyun");
        assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");
        assert_eq!(target.original_model, "CTYun/deepseek-v4.1-flash-1017");

        // `[1M]` 标记在引用里要剥掉，仍能解析
        let target =
            resolve_reference_target(&db, &AppType::Claude, "CTYun/deepseek-v4.1-flash-1017[1M]")
                .expect("resolve")
                .expect("hit");
        assert_eq!(target.provider.id, "p-ctyun");
        assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");
    }

    #[test]
    fn plain_model_names_are_left_to_the_default_route() {
        let db = Database::memory().expect("memory db");
        assert!(
            resolve_reference_target(&db, &AppType::Claude, "claude-sonnet-4-5")
                .expect("resolve")
                .is_none()
        );
    }

    #[test]
    fn unknown_model_under_a_known_supplier_is_an_error() {
        let db = Database::memory().expect("memory db");
        db.save_provider(
            "claude",
            &claude_provider(
                "p-ctyun",
                "CTYun",
                "https://ai.ctaigw.cn/v1",
                &["deepseek-v4.1-flash-1017"],
            ),
        )
        .expect("save");
        let result = resolve_reference_target(&db, &AppType::Claude, "CTYun/not-a-model")
            .expect_err("要报错");
        assert!(result.to_string().contains("不在目录里"), "{result}");
    }

    /// 真实库副本验证：
    /// `CCS_VERIFY_DB=/path/to/copy.db cargo test --lib gateway_route -- --ignored --nocapture`
    #[test]
    #[ignore = "手动：用真实库副本验证，需 CCS_VERIFY_DB"]
    fn manual_resolve_against_real_db_copy() -> Result<(), AppError> {
        let Ok(path) = std::env::var("CCS_VERIFY_DB") else {
            return Ok(());
        };
        let db = Database::open_and_migrate_for_test(std::path::Path::new(&path))?;
        let catalog = crate::services::gateway_catalog::catalog_view(&db)?;
        for entry in catalog
            .iter()
            .filter(|entry| {
                entry
                    .sources
                    .iter()
                    .any(|source| source.starts_with("claude:"))
            })
            .take(4)
        {
            let target = resolve_reference_target(&db, &AppType::Claude, &entry.reference)?;
            println!(
                "{} -> {:?}",
                entry.reference,
                target.as_ref().map(|t| (&t.provider.id, &t.upstream_model))
            );
            assert!(target.is_some(), "{} 应能解析", entry.reference);
        }
        // 客户端自己的别名（供应商名不存在）→ 交给默认路由
        assert!(resolve_reference_target(&db, &AppType::Claude, "anthropic/claude-x")?.is_none());
        Ok(())
    }

    #[test]
    fn claude_desktop_role_ids_resolve_through_slots() {
        use crate::provider::{ClaudeDesktopModelRoute, ProviderMeta};

        let db = Database::memory().expect("memory db");
        let mut provider = claude_provider(
            "p-ctyun",
            "CTYun",
            "https://ai.ctaigw.cn/v1",
            &["deepseek-v4.1-flash-1017"],
        );
        provider.meta = Some(ProviderMeta {
            claude_desktop_model_routes: [(
                "claude-sonnet-5".to_string(),
                ClaudeDesktopModelRoute {
                    model: "deepseek-v4.1-flash-1017".to_string(),
                    label_override: Some("DeepSeek V4.1".to_string()),
                    supports_1m: None,
                },
            )]
            .into_iter()
            .collect(),
            ..ProviderMeta::default()
        });
        db.save_provider("claude-desktop", &provider).expect("save");
        db.set_app_model_config(
            "claude-desktop",
            &app_config(
                None,
                &[],
                &[
                    ("opus", "CTYun/deepseek-v4.1-flash-1017"),
                    ("haiku", "CTYun/deepseek-v4.1-flash-1017"),
                ],
            ),
        )
        .expect("save config");

        // 配了槽位的角色 → 按槽位里的引用选上游（Opus 的当前 route id 与旧 route id 都认）
        for model in ["claude-opus-5", "claude-opus-4-8"] {
            let target = resolve_desktop_role_target(&db, model)
                .expect("resolve")
                .unwrap_or_else(|| panic!("{model} 应命中 opus 槽位"));
            assert_eq!(target.provider.id, "p-ctyun");
            assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");
        }
        // Haiku 5.5 是旧库可能留下的另一个 id
        assert!(resolve_desktop_role_target(&db, "claude-haiku-5-5")
            .expect("resolve")
            .is_some());
        // 没配槽位的角色 → 交给原来的 provider 映射表
        assert!(resolve_desktop_role_target(&db, "claude-sonnet-5")
            .expect("resolve")
            .is_none());
        // 不是角色 id → 不掺和
        assert!(resolve_desktop_role_target(&db, "kimi-k3")
            .expect("resolve")
            .is_none());
    }

    #[test]
    fn reference_from_another_app_is_rejected_or_missing() {
        let db = Database::memory().expect("memory db");
        db.save_provider(
            "claude-desktop",
            &claude_provider(
                "p-ctyun",
                "CTYun",
                "https://ai.ctaigw.cn/v1",
                &["deepseek-v4.1-flash-1017"],
            ),
        )
        .expect("save");

        // 目录里没有 model 来源是 claude，故不是引用命中
        let result =
            resolve_reference_target(&db, &AppType::Claude, "CTYun/deepseek-v4.1-flash-1017")
                .expect("resolve");
        assert!(result.is_none());
    }

    fn desktop_with_config() -> Database {
        use crate::provider::{ClaudeDesktopMode, ClaudeDesktopModelRoute, ProviderMeta};

        let db = Database::memory().expect("memory db");
        let mut provider = claude_provider(
            "p-ctyun",
            "CTYun",
            "https://ai.ctaigw.cn/v1",
            &["deepseek-v4.1-flash-1017"],
        );
        // 目录里的 Desktop 行来自映射表：没有 meta 的 provider 不贡献模型。
        provider.meta = Some(ProviderMeta {
            claude_desktop_mode: Some(ClaudeDesktopMode::Proxy),
            claude_desktop_model_routes: [(
                "claude-sonnet-5".to_string(),
                ClaudeDesktopModelRoute {
                    model: "deepseek-v4.1-flash-1017".to_string(),
                    label_override: None,
                    supports_1m: None,
                },
            )]
            .into_iter()
            .collect(),
            ..ProviderMeta::default()
        });
        db.save_provider("claude-desktop", &provider).expect("save");
        db.set_app_model_config(
            "claude-desktop",
            &app_config(
                Some("CTYun/deepseek-v4.1-flash-1017"),
                &["CTYun/deepseek-v4.1-flash-1017"],
                &[],
            ),
        )
        .expect("save config");
        db
    }

    #[test]
    fn claude_desktop_config_references_resolve_through_encoded_ids() {
        let db = desktop_with_config();
        let id = crate::mode::stack::desktop_config_id("CTYun/deepseek-v4.1-flash-1017");
        assert!(
            crate::claude_desktop_config::is_claude_safe_model_id(&id),
            "编码后的 id 必须是 Desktop 认的 claude-* 形状: {id}"
        );

        let target = resolve_desktop_config_target(&db, &id)
            .expect("resolve")
            .expect("hit");
        assert_eq!(target.provider.id, "p-ctyun");
        assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");
        assert_eq!(target.original_model, "CTYun/deepseek-v4.1-flash-1017");

        // Desktop 对 supports1m 的条目会在 id 后面补 [1m]
        let target = resolve_desktop_config_target(&db, &format!("{id}[1m]"))
            .expect("resolve")
            .expect("hit");
        assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");

        // 角色 id、别的普通模型名不走这条
        for other in ["claude-haiku-5-5", "claude-fable-5", "kimi-k3"] {
            assert!(
                resolve_desktop_config_target(&db, other)
                    .expect("resolve")
                    .is_none(),
                "{other}"
            );
        }
    }

    #[test]
    fn claude_desktop_config_reference_removed_from_config_is_an_error() {
        let db = desktop_with_config();
        let id = crate::mode::stack::desktop_config_id("CTYun/removed-model");
        let error = resolve_desktop_config_target(&db, &id).expect_err("要报错");
        assert!(error.to_string().contains("模型配置里没有"), "{error}");
    }

    #[test]
    fn claude_desktop_config_ids_are_left_to_the_stack_when_no_config() {
        let db = Database::memory().expect("memory db");
        let id = crate::mode::stack::desktop_config_id("CTYun/deepseek-v4.1-flash-1017");
        assert!(
            resolve_desktop_config_target(&db, &id)
                .expect("resolve")
                .is_none(),
            "没配模型配置时编码 id 交给 Stack 解析"
        );
    }
}
