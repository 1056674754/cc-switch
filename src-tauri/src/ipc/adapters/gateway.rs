// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/gateway.rs（共 26 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_gateway_catalog(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_refresh: Option<bool> = __args.option::<bool>("refresh")?;
    let __p_include_hidden: Option<bool> = __args.option::<bool>("includeHidden")?;
    let __result = get_gateway_catalog(__p_state, __p_refresh, __p_include_hidden);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_fetch_supplier_models(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_oauth_state = __args.state::<crate::commands::CodexOAuthState>(__app)?;
    let __p_upstream_ids: Vec<String> = __args.required::<Vec<String>>("upstreamIds")?;
    let __result = fetch_supplier_models(__p_state, __p_oauth_state, __p_upstream_ids).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_app_model_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __result = get_app_model_config(__p_state, __p_app_type);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_model_overrides(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_model_overrides(__p_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_model_override(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_upstream_id: String = __args.required::<String>("upstreamId")?;
    let __p_model_id: String = __args.required::<String>("modelId")?;
    let __p_display_name: Option<String> = __args.option::<String>("displayName")?;
    let __p_hidden: bool = __args.required::<bool>("hidden")?;
    let __result = set_model_override(
        __p_state,
        __p_upstream_id,
        __p_model_id,
        __p_display_name,
        __p_hidden,
    );
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_clear_model_override(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_upstream_id: String = __args.required::<String>("upstreamId")?;
    let __p_model_id: String = __args.required::<String>("modelId")?;
    let __result = clear_model_override(__p_state, __p_upstream_id, __p_model_id);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_opencode_to_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = adopt_opencode_to_gateway(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_release_opencode_from_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = release_opencode_from_gateway();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_opencode_gateway_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_opencode_gateway_status();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_hermes_to_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = adopt_hermes_to_gateway(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_hermes_gateway_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_hermes_gateway_status();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_release_hermes_from_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = release_hermes_from_gateway();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_openclaw_to_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = adopt_openclaw_to_gateway(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_openclaw_gateway_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_openclaw_gateway_status();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_release_openclaw_from_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = release_openclaw_from_gateway();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_pi_to_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = adopt_pi_to_gateway(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_pi_gateway_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_pi_gateway_status();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_release_pi_from_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = release_pi_from_gateway();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_omp_to_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = adopt_omp_to_gateway(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_omp_gateway_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_omp_gateway_status();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_release_omp_from_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = release_omp_from_gateway();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_dsh_to_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = adopt_dsh_to_gateway(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_dsh_gateway_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_dsh_gateway_status();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_release_dsh_from_gateway(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = release_dsh_from_gateway();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_app_model_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_config: AppModelConfig = __args.required::<AppModelConfig>("config")?;
    let __result = set_app_model_config(__p_state, __p_app_type, __p_config).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_migrate_gateway_from_stack(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = migrate_gateway_from_stack(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
