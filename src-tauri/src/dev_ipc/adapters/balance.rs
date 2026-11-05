// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/balance.rs（共 1 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_balance(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_base_url: String = __args.required::<String>("baseUrl")?;
    let __p_api_key: String = __args.required::<String>("apiKey")?;
    let __result = get_balance(__p_base_url, __p_api_key).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
