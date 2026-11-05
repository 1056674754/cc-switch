// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/prompt.rs（共 13 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_prompts(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_prompts(__p_app, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_upsert_prompt(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_prompt: Prompt = __args.required::<Prompt>("prompt")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = upsert_prompt(__p_app, __p_id, __p_prompt, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_prompt(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = delete_prompt(__p_app, __p_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_enable_prompt(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = enable_prompt(__p_app, __p_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_prompt_from_file(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = import_prompt_from_file(__p_app, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_current_prompt_file_content(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __result = get_current_prompt_file_content(__p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_prompt_file_location(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: String = __args.required::<String>("app")?;
    let __result = get_prompt_file_location(__p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_pi_prompt_file(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_kind: PiPromptFileKind = __args.required::<PiPromptFileKind>("kind")?;
    let __result = get_pi_prompt_file(__p_kind).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_replace_pi_prompt_file(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_kind: PiPromptFileKind = __args.required::<PiPromptFileKind>("kind")?;
    let __p_expectedRevision: String = __args.required::<String>("expectedRevision")?;
    let __p_content: String = __args.required::<String>("content")?;
    let __result = replace_pi_prompt_file(__p_kind, __p_expectedRevision, __p_content).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_pi_prompt_file(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_kind: PiPromptFileKind = __args.required::<PiPromptFileKind>("kind")?;
    let __p_expectedRevision: String = __args.required::<String>("expectedRevision")?;
    let __result = delete_pi_prompt_file(__p_kind, __p_expectedRevision).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_pi_prompt_templates(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = list_pi_prompt_templates().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_upsert_pi_prompt_template(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_slug: String = __args.required::<String>("slug")?;
    let __p_originalSlug: Option<String> = __args.option::<String>("originalSlug")?;
    let __p_expectedRevision: String = __args.required::<String>("expectedRevision")?;
    let __p_content: String = __args.required::<String>("content")?;
    let __result = upsert_pi_prompt_template(
        __p_slug,
        __p_originalSlug,
        __p_expectedRevision,
        __p_content,
    )
    .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_pi_prompt_template(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_slug: String = __args.required::<String>("slug")?;
    let __p_expectedRevision: String = __args.required::<String>("expectedRevision")?;
    let __result = delete_pi_prompt_template(__p_slug, __p_expectedRevision).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
