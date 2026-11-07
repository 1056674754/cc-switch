// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/suppliers.rs（共 19 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_suppliers_overview(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_suppliers_overview(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_supplier_base_url(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_base_url: String = __args.required::<String>("baseUrl")?;
    let __result = update_supplier_base_url(__p_state, __p_supplier_id, __p_base_url).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_supplier_name(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_name: String = __args.required::<String>("name")?;
    let __result = update_supplier_name(__p_state, __p_supplier_id, __p_name).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_adopt_supplier(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __result = adopt_supplier(__p_state, __p_app_type, __p_supplier_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_create_supplier(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_name: String = __args.required::<String>("name")?;
    let __p_base_url: String = __args.required::<String>("baseUrl")?;
    let __p_api_key: String = __args.required::<String>("apiKey")?;
    let __p_apps: Vec<String> = __args.required::<Vec<String>>("apps")?;
    let __p_protocol: Option<String> = __args.option::<String>("protocol")?;
    let __result = create_supplier(
        __p_state,
        __p_name,
        __p_base_url,
        __p_api_key,
        __p_apps,
        __p_protocol,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_ensure_managed_supplier(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __result = ensure_managed_supplier(__p_state, __p_auth_provider).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_supplier_secret(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_secret: String = __args.required::<String>("secret")?;
    let __result = update_supplier_secret(__p_state, __p_supplier_id, __p_secret).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_supplier_credential(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_key_hash: String = __args.required::<String>("keyHash")?;
    let __p_secret: String = __args.required::<String>("secret")?;
    let __result =
        update_supplier_credential(__p_state, __p_supplier_id, __p_key_hash, __p_secret).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_supplier_credential(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_secret: String = __args.required::<String>("secret")?;
    let __result =
        add_supplier_credential(__p_state, __p_supplier_id, __p_app_type, __p_secret).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_supplier_credential(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_key_hash: String = __args.required::<String>("keyHash")?;
    let __result = delete_supplier_credential(__p_state, __p_supplier_id, __p_key_hash).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_supplier_credential_enabled(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_key_hash: String = __args.required::<String>("keyHash")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result =
        set_supplier_credential_enabled(__p_state, __p_supplier_id, __p_key_hash, __p_enabled)
            .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_supplier_account(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_ids: Vec<String> = __args.required::<Vec<String>>("supplierIds")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_name: Option<String> = __args.option::<String>("name")?;
    let __result =
        add_supplier_account(__p_state, __p_supplier_ids, __p_account_id, __p_name).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_supplier_account(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_ids: Vec<String> = __args.required::<Vec<String>>("supplierIds")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __result = delete_supplier_account(__p_state, __p_supplier_ids, __p_account_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_supplier_account_enabled(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_ids: Vec<String> = __args.required::<Vec<String>>("supplierIds")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result =
        set_supplier_account_enabled(__p_state, __p_supplier_ids, __p_account_id, __p_enabled)
            .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_supplier_current_account(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_ids: Vec<String> = __args.required::<Vec<String>>("supplierIds")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __result = set_supplier_current_account(__p_state, __p_supplier_ids, __p_account_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_binding_model_map(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_provider_id: String = __args.required::<String>("providerId")?;
    let __p_model_map: serde_json::Value = __args.required::<serde_json::Value>("modelMap")?;
    let __result =
        update_binding_model_map(__p_state, __p_app_type, __p_provider_id, __p_model_map).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_supplier_enabled(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result = set_supplier_enabled(__p_state, __p_supplier_id, __p_enabled).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_supplier_managed_account(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_ids: Vec<String> = __args.required::<Vec<String>>("supplierIds")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __result = set_supplier_managed_account(__p_state, __p_supplier_ids, __p_account_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_supplier(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_supplier_id: String = __args.required::<String>("supplierId")?;
    let __result = delete_supplier(__p_state, __p_supplier_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
