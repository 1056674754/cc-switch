// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/provider.rs（共 34 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_providers(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __result = get_providers(__p_state, __p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_current_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __result = get_current_provider(__p_state, __p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_handle: tauri::AppHandle = __app.clone();
    let __p_app: String = __args.required::<String>("app")?;
    let __p_provider: Provider = __args.required::<Provider>("provider")?;
    let __p_addToLive: Option<bool> = __args.option::<bool>("addToLive")?;
    let __p_editorSave: Option<EditorSave> = __args.option::<EditorSave>("editorSave")?;
    let __result = add_provider(
        __p_app_handle,
        __p_app,
        __p_provider,
        __p_addToLive,
        __p_editorSave,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_handle: tauri::AppHandle = __app.clone();
    let __p_app: String = __args.required::<String>("app")?;
    let __p_provider: Provider = __args.required::<Provider>("provider")?;
    let __p_originalId: Option<String> = __args.option::<String>("originalId")?;
    let __p_editorSave: Option<EditorSave> = __args.option::<EditorSave>("editorSave")?;
    let __result = update_provider(
        __p_app_handle,
        __p_app,
        __p_provider,
        __p_originalId,
        __p_editorSave,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_provider_editor_view(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_handle: tauri::AppHandle = __app.clone();
    let __p_app: String = __args.required::<String>("app")?;
    let __p_settingsConfig: serde_json::Value =
        __args.required::<serde_json::Value>("settingsConfig")?;
    let __p_category: Option<String> = __args.option::<String>("category")?;
    let __p_providerId: Option<String> = __args.option::<String>("providerId")?;
    let __p_meta: Option<ProviderMeta> = __args.option::<ProviderMeta>("meta")?;
    let __result = get_provider_editor_view(
        __p_app_handle,
        __p_app,
        __p_settingsConfig,
        __p_category,
        __p_providerId,
        __p_meta,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_provider(__p_state, __p_app, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_provider_to_live_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = add_provider_to_live_config(__p_state, __p_app, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_remove_provider_from_live_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = remove_provider_from_live_config(__p_state, __p_app, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_switch_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_handle: tauri::AppHandle = __app.clone();
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = switch_provider(__p_app_handle, __p_app, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_default_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __result = import_default_config(__p_state, __p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_claude_desktop_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_claude_desktop_status(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_claude_desktop_default_routes(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_claude_desktop_default_routes();
    crate::ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_claude_desktop_providers_from_claude(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = import_claude_desktop_providers_from_claude(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_ensure_claude_desktop_official_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = ensure_claude_desktop_official_provider(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_ensure_codex_official_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = ensure_codex_official_provider(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_ensure_grokbuild_official_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = ensure_grokbuild_official_provider(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_read_live_provider_settings(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __result = read_live_provider_settings(__p_app);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_queryProviderUsage(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_handle: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_app: String = __args.required::<String>("app")?;
    let __result = queryProviderUsage(
        __p_app_handle,
        __p_state,
        __p_copilot_state,
        __p_xai_state,
        __p_providerId,
        __p_app,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_testUsageScript(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_scriptCode: String = __args.required::<String>("scriptCode")?;
    let __p_timeout: Option<u64> = __args.option::<u64>("timeout")?;
    let __p_apiKey: Option<String> = __args.option::<String>("apiKey")?;
    let __p_baseUrl: Option<String> = __args.option::<String>("baseUrl")?;
    let __p_accessToken: Option<String> = __args.option::<String>("accessToken")?;
    let __p_userId: Option<String> = __args.option::<String>("userId")?;
    let __p_templateType: Option<String> = __args.option::<String>("templateType")?;
    let __result = testUsageScript(
        __p_state,
        __p_providerId,
        __p_app,
        __p_scriptCode,
        __p_timeout,
        __p_apiKey,
        __p_baseUrl,
        __p_accessToken,
        __p_userId,
        __p_templateType,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_test_api_endpoints(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_urls: Vec<String> = __args.required::<Vec<String>>("urls")?;
    let __p_timeoutSecs: Option<u64> = __args.option::<u64>("timeoutSecs")?;
    let __result = test_api_endpoints(__p_urls, __p_timeoutSecs).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_custom_endpoints(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __result = get_custom_endpoints(__p_state, __p_app, __p_providerId).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_custom_endpoint(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_url: String = __args.required::<String>("url")?;
    let __result = add_custom_endpoint(__p_state, __p_app, __p_providerId, __p_url).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_remove_custom_endpoint(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_url: String = __args.required::<String>("url")?;
    let __result = remove_custom_endpoint(__p_state, __p_app, __p_providerId, __p_url).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_endpoint_last_used(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_url: String = __args.required::<String>("url")?;
    let __result = update_endpoint_last_used(__p_state, __p_app, __p_providerId, __p_url).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_providers_sort_order(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_handle: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_updates: Vec<ProviderSortUpdate> =
        __args.required::<Vec<ProviderSortUpdate>>("updates")?;
    let __result =
        update_providers_sort_order(__p_app_handle, __p_state, __p_app, __p_updates).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_universal_providers(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_universal_providers(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_universal_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = get_universal_provider(__p_state, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_upsert_universal_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_provider: UniversalProvider = __args.required::<UniversalProvider>("provider")?;
    let __result = upsert_universal_provider(__p_app, __p_state, __p_provider).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_universal_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_universal_provider(__p_app, __p_state, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_sync_universal_provider(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = sync_universal_provider(__p_app, __p_state, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_opencode_providers_from_live(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = import_opencode_providers_from_live(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_opencode_live_provider_ids(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_opencode_live_provider_ids();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_opencode_default_model(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_opencode_default_model();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_opencode_default_model(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_model: String = __args.required::<String>("model")?;
    let __result = set_opencode_default_model(__p_model);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
