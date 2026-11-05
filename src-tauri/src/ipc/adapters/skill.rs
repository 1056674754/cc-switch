// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/skill.rs（共 29 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_installed_skills(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = get_installed_skills(__p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_skill_backups(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_skill_backups();
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_skill_backup(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_backup_id: String = __args.required::<String>("backupId")?;
    let __result = delete_skill_backup(__p_backup_id);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_install_skill_unified(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_skill: DiscoverableSkill = __args.required::<DiscoverableSkill>("skill")?;
    let __p_current_app: String = __args.required::<String>("currentApp")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result =
        install_skill_unified(__p_skill, __p_current_app, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_uninstall_skill_unified(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = uninstall_skill_unified(__p_id, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_restore_skill_backup(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_backup_id: String = __args.required::<String>("backupId")?;
    let __p_current_app: String = __args.required::<String>("currentApp")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = restore_skill_backup(__p_backup_id, __p_current_app, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_toggle_skill_app(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = toggle_skill_app(__p_id, __p_app, __p_enabled, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_scan_unmanaged_skills(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = scan_unmanaged_skills(__p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_skills_from_apps(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_imports: Vec<ImportSkillSelection> =
        __args.required::<Vec<ImportSkillSelection>>("imports")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = import_skills_from_apps(__p_imports, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_discover_available_skills(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_force_refresh: Option<bool> = __args.option::<bool>("forceRefresh")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = discover_available_skills(__p_force_refresh, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_discover_repo_skills(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_owner: String = __args.required::<String>("owner")?;
    let __p_name: String = __args.required::<String>("name")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = discover_repo_skills(__p_owner, __p_name, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_check_skill_updates(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = check_skill_updates(__p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_resync_skills_to_apps(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = resync_skills_to_apps(__p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_skill(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = update_skill(__p_id, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_migrate_skill_storage(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_target: SkillStorageLocation = __args.required::<SkillStorageLocation>("target")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = migrate_skill_storage(__p_target, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_cc_switch_skills_dir(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_cc_switch_skills_dir().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_cc_switch_skills_dir(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_handle: tauri::AppHandle = __app.clone();
    let __result = open_cc_switch_skills_dir(__p_handle).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_skill_dir(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_handle: tauri::AppHandle = __app.clone();
    let __p_directory: String = __args.required::<String>("directory")?;
    let __result = open_skill_dir(__p_handle, __p_directory).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_search_skills_sh(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_query: String = __args.required::<String>("query")?;
    let __p_limit: usize = __args.required::<usize>("limit")?;
    let __p_offset: usize = __args.required::<usize>("offset")?;
    let __result = search_skills_sh(__p_query, __p_limit, __p_offset).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_skills(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = get_skills(__p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_skills_for_app(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = get_skills_for_app(__p_app, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_install_skill(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_directory: String = __args.required::<String>("directory")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = install_skill(__p_directory, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_install_skill_for_app(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_directory: String = __args.required::<String>("directory")?;
    let __p_service = __args.state::<SkillServiceState>(__app)?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = install_skill_for_app(__p_app, __p_directory, __p_service, __p_app_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_uninstall_skill(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_directory: String = __args.required::<String>("directory")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = uninstall_skill(__p_directory, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_uninstall_skill_for_app(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_directory: String = __args.required::<String>("directory")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = uninstall_skill_for_app(__p_app, __p_directory, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_skill_repos(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = get_skill_repos(__p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_skill_repo(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_repo: SkillRepo = __args.required::<SkillRepo>("repo")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = add_skill_repo(__p_repo, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_remove_skill_repo(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_owner: String = __args.required::<String>("owner")?;
    let __p_name: String = __args.required::<String>("name")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = remove_skill_repo(__p_owner, __p_name, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_install_skills_from_zip(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_file_path: String = __args.required::<String>("filePath")?;
    let __p_current_app: String = __args.required::<String>("currentApp")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __result = install_skills_from_zip(__p_file_path, __p_current_app, __p_app_state);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
