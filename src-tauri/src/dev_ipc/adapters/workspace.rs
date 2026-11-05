// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/workspace.rs（共 8 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_read_workspace_file(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_filename: String = __args.required::<String>("filename")?;
    let __result = read_workspace_file(__p_filename).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_write_workspace_file(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_filename: String = __args.required::<String>("filename")?;
    let __p_content: String = __args.required::<String>("content")?;
    let __result = write_workspace_file(__p_filename, __p_content).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_daily_memory_files(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = list_daily_memory_files().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_read_daily_memory_file(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_filename: String = __args.required::<String>("filename")?;
    let __result = read_daily_memory_file(__p_filename).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_write_daily_memory_file(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_filename: String = __args.required::<String>("filename")?;
    let __p_content: String = __args.required::<String>("content")?;
    let __result = write_daily_memory_file(__p_filename, __p_content).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_daily_memory_file(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_filename: String = __args.required::<String>("filename")?;
    let __result = delete_daily_memory_file(__p_filename).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_search_daily_memory_files(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_query: String = __args.required::<String>("query")?;
    let __result = search_daily_memory_files(__p_query).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_workspace_directory(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_handle: tauri::AppHandle = __app.clone();
    let __p_subdir: String = __args.required::<String>("subdir")?;
    let __result = open_workspace_directory(__p_handle, __p_subdir).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
