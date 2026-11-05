use crate::services::env_checker::{check_env_conflicts as check_conflicts, EnvConflict};
use crate::services::env_manager::{
    delete_env_vars as delete_vars, restore_from_backup, BackupInfo,
};

/// Check environment variable conflicts for a specific app
#[tauri::command]
pub fn check_env_conflicts(app: String) -> Result<Vec<EnvConflict>, String> {
    check_conflicts(&app)
}

/// Delete environment variables with backup
#[tauri::command]
pub fn delete_env_vars(conflicts: Vec<EnvConflict>) -> Result<BackupInfo, String> {
    delete_vars(conflicts)
}

/// Restore environment variables from backup file
#[tauri::command]
pub fn restore_env_backup(backup_path: String) -> Result<(), String> {
    restore_from_backup(backup_path)
}

// >>> dev-ipc adapters（生成物：scripts/gen_dev_ipc_adapters.py，勿手改）
#[cfg(feature = "dev-ipc")]
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod dev_ipc_adapters {
    include!("../dev_ipc/adapters/env.rs");
}
// <<< dev-ipc adapters
