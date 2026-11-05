// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/env.rs（共 3 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_check_env_conflicts(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __result = check_env_conflicts(__p_app);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_env_vars(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_conflicts: Vec<EnvConflict> = __args.required::<Vec<EnvConflict>>("conflicts")?;
    let __result = delete_env_vars(__p_conflicts);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_restore_env_backup(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_backup_path: String = __args.required::<String>("backupPath")?;
    let __result = restore_env_backup(__p_backup_path);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
