// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/usage.rs（共 23 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_usage_summary(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_start_date: Option<i64> = __args.option::<i64>("startDate")?;
    let __p_end_date: Option<i64> = __args.option::<i64>("endDate")?;
    let __p_app_type: Option<String> = __args.option::<String>("appType")?;
    let __p_provider_name: Option<String> = __args.option::<String>("providerName")?;
    let __p_model: Option<String> = __args.option::<String>("model")?;
    let __result = get_usage_summary(__p_state, __p_start_date, __p_end_date, __p_app_type, __p_provider_name, __p_model).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_session_usage_summary(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_session_id: String = __args.required::<String>("sessionId")?;
    let __result = get_session_usage_summary(__p_state, __p_app_type, __p_session_id).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_usage_summary_by_app(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_start_date: Option<i64> = __args.option::<i64>("startDate")?;
    let __p_end_date: Option<i64> = __args.option::<i64>("endDate")?;
    let __p_provider_name: Option<String> = __args.option::<String>("providerName")?;
    let __p_model: Option<String> = __args.option::<String>("model")?;
    let __result = get_usage_summary_by_app(__p_state, __p_start_date, __p_end_date, __p_provider_name, __p_model).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_usage_trends(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_start_date: Option<i64> = __args.option::<i64>("startDate")?;
    let __p_end_date: Option<i64> = __args.option::<i64>("endDate")?;
    let __p_app_type: Option<String> = __args.option::<String>("appType")?;
    let __p_provider_name: Option<String> = __args.option::<String>("providerName")?;
    let __p_model: Option<String> = __args.option::<String>("model")?;
    let __result = get_usage_trends(__p_state, __p_start_date, __p_end_date, __p_app_type, __p_provider_name, __p_model).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_usage_first_date(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: Option<String> = __args.option::<String>("appType")?;
    let __p_provider_name: Option<String> = __args.option::<String>("providerName")?;
    let __p_model: Option<String> = __args.option::<String>("model")?;
    let __result = get_usage_first_date(__p_state, __p_app_type, __p_provider_name, __p_model).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_provider_stats(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_start_date: Option<i64> = __args.option::<i64>("startDate")?;
    let __p_end_date: Option<i64> = __args.option::<i64>("endDate")?;
    let __p_app_type: Option<String> = __args.option::<String>("appType")?;
    let __p_provider_name: Option<String> = __args.option::<String>("providerName")?;
    let __p_model: Option<String> = __args.option::<String>("model")?;
    let __result = get_provider_stats(__p_state, __p_start_date, __p_end_date, __p_app_type, __p_provider_name, __p_model).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_model_stats(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_start_date: Option<i64> = __args.option::<i64>("startDate")?;
    let __p_end_date: Option<i64> = __args.option::<i64>("endDate")?;
    let __p_app_type: Option<String> = __args.option::<String>("appType")?;
    let __p_provider_name: Option<String> = __args.option::<String>("providerName")?;
    let __p_model: Option<String> = __args.option::<String>("model")?;
    let __result = get_model_stats(__p_state, __p_start_date, __p_end_date, __p_app_type, __p_provider_name, __p_model).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_request_logs(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_filters: LogFilters = __args.required::<LogFilters>("filters")?;
    let __p_page: u32 = __args.required::<u32>("page")?;
    let __p_page_size: u32 = __args.required::<u32>("pageSize")?;
    let __result = get_request_logs(__p_state, __p_filters, __p_page, __p_page_size).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_request_detail(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_request_id: String = __args.required::<String>("requestId")?;
    let __result = get_request_detail(__p_state, __p_request_id).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_model_pricing(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_model_pricing(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_model_pricing(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_model_id: String = __args.required::<String>("modelId")?;
    let __p_display_name: String = __args.required::<String>("displayName")?;
    let __p_input_cost: String = __args.required::<String>("inputCost")?;
    let __p_output_cost: String = __args.required::<String>("outputCost")?;
    let __p_cache_read_cost: String = __args.required::<String>("cacheReadCost")?;
    let __p_cache_creation_cost: String = __args.required::<String>("cacheCreationCost")?;
    let __result = update_model_pricing(__p_state, __p_model_id, __p_display_name, __p_input_cost, __p_output_cost, __p_cache_read_cost, __p_cache_creation_cost).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_model_pricing_batch(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_entries: Vec<ModelPricingInfo> = __args.required::<Vec<ModelPricingInfo>>("entries")?;
    let __result = update_model_pricing_batch(__p_state, __p_entries).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_model_pricing(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_model_id: String = __args.required::<String>("modelId")?;
    let __result = delete_model_pricing(__p_state, __p_model_id).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_models_dev_sync_config(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_models_dev_sync_config(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_save_models_dev_sync_config(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_config: ModelsDevSyncConfig = __args.required::<ModelsDevSyncConfig>("config")?;
    let __result = save_models_dev_sync_config(__p_state, __p_config).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_record_models_dev_sync_result(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_synced_at: Option<i64> = __args.option::<i64>("syncedAt")?;
    let __p_error: Option<String> = __args.option::<String>("error")?;
    let __result = record_models_dev_sync_result(__p_state, __p_synced_at, __p_error).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_fetch_models_dev(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = fetch_models_dev().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_check_provider_limits(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_provider_id: String = __args.required::<String>("providerId")?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __result = check_provider_limits(__p_state, __p_provider_id, __p_app_type).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_sync_session_usage(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = sync_session_usage(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_rebuild_codex_usage(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = rebuild_codex_usage(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_session_usage_last_sync(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_session_usage_last_sync();
    crate::dev_ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_usage_data_sources(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_usage_data_sources(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_fetch_exchange_rates(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = fetch_exchange_rates().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}
