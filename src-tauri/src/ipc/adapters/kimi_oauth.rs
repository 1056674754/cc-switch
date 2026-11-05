// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/kimi_oauth.rs（共 8 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_status(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_start(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_region: KimiRegion = __args.required::<KimiRegion>("region")?;
    let __p_account_id: Option<String> = __args.option::<String>("accountId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_start(__p_region, __p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_poll(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_flow_id: String = __args.required::<String>("flowId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_poll(__p_flow_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_cancel(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_flow_id: String = __args.required::<String>("flowId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_cancel(__p_flow_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_remove(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = kimi_oauth_remove(__p_account_id, __p_state, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_quota(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_quota(__p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_use_account(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_use_account(__p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_kimi_oauth_models(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<KimiOAuthManager>(__app)?;
    let __result = kimi_oauth_models(__p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
