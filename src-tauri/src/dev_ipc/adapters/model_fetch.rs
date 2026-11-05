// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/model_fetch.rs（共 3 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_fetch_models_for_config(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_base_url: String = __args.required::<String>("baseUrl")?;
    let __p_api_key: String = __args.required::<String>("apiKey")?;
    let __p_is_full_url: Option<bool> = __args.option::<bool>("isFullUrl")?;
    let __p_models_url: Option<String> = __args.option::<String>("modelsUrl")?;
    let __p_custom_user_agent: Option<String> = __args.option::<String>("customUserAgent")?;
    let __p_api_format: Option<String> = __args.option::<String>("apiFormat")?;
    let __p_request_headers: Option<BTreeMap<String, String>> = __args.option::<BTreeMap<String, String>>("requestHeaders")?;
    let __result = fetch_models_for_config(__p_base_url, __p_api_key, __p_is_full_url, __p_models_url, __p_custom_user_agent, __p_api_format, __p_request_headers).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_agy_models(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_apiKey: Option<String> = __args.option::<String>("apiKey")?;
    let __p_baseUrl: Option<String> = __args.option::<String>("baseUrl")?;
    let __result = get_agy_models(__p_apiKey, __p_baseUrl).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_opencode_models(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_opencode_models().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
