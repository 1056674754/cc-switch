//! `ccswitch apps`：列出支持的应用标识。

use super::super::{Context, Parsed, Payload};

pub fn run(_ctx: &Context, _parsed: &Parsed) -> Result<Payload, String> {
    Ok(Payload::Lines(
        crate::app_config::AppType::all()
            .map(|app| app.as_str().to_string())
            .collect(),
    ))
}
