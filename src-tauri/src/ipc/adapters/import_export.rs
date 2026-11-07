// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/import_export.rs（共 14 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_export_config_to_file(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_filePath: String = __args.required::<String>("filePath")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = export_config_to_file(__p_filePath, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_config_from_file(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_filePath: String = __args.required::<String>("filePath")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = import_config_from_file(__p_filePath, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_save_file_dialog(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_defaultName: String = __args.required::<String>("defaultName")?;
    let __result = save_file_dialog(__p_app, __p_defaultName).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_file_dialog(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = open_file_dialog(__p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_zip_file_dialog(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = open_zip_file_dialog(__p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_create_db_backup(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = create_db_backup(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_db_backups(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = list_db_backups().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_backup_locations(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = list_backup_locations().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_backup_location(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_backup_location(__p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_reveal_backup_location(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __result = reveal_backup_location(__p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_restore_db_backup(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_filename: String = __args.required::<String>("filename")?;
    let __result = restore_db_backup(__p_state, __p_filename).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_rename_db_backup(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_oldFilename: String = __args.required::<String>("oldFilename")?;
    let __p_newName: String = __args.required::<String>("newName")?;
    let __result = rename_db_backup(__p_oldFilename, __p_newName).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_db_backup(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_filename: String = __args.required::<String>("filename")?;
    let __result = delete_db_backup(__p_filename).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_sync_current_providers_live(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = sync_current_providers_live(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
