// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/omp.rs（共 1 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_omp_current_state(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_omp_current_state(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
