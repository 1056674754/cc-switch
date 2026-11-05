// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 命令清单来自 src-tauri/src/lib.rs 的 generate_handler!，与桌面端注册表一一对应。

use tauri::AppHandle;

/// 已桥接的命令名（顺序与 generate_handler! 一致）。
pub(crate) const COMMANDS: &[&str] = &[
    "get_providers",
    "get_gateway_catalog",
    "get_app_model_config",
    "get_model_overrides",
    "set_model_override",
    "clear_model_override",
    "adopt_opencode_to_gateway",
    "release_opencode_from_gateway",
    "get_opencode_gateway_status",
    "adopt_hermes_to_gateway",
    "get_hermes_gateway_status",
    "release_hermes_from_gateway",
    "adopt_openclaw_to_gateway",
    "get_openclaw_gateway_status",
    "release_openclaw_from_gateway",
    "adopt_pi_to_gateway",
    "get_pi_gateway_status",
    "release_pi_from_gateway",
    "adopt_omp_to_gateway",
    "get_omp_gateway_status",
    "release_omp_from_gateway",
    "adopt_dsh_to_gateway",
    "get_dsh_gateway_status",
    "release_dsh_from_gateway",
    "set_app_model_config",
    "migrate_gateway_from_stack",
    "get_suppliers_overview",
    "update_supplier_base_url",
    "adopt_supplier",
    "create_supplier",
    "update_supplier_secret",
    "update_binding_model_map",
    "set_supplier_enabled",
    "delete_supplier",
    "get_current_provider",
    "add_provider",
    "update_provider",
    "get_provider_editor_view",
    "delete_provider",
    "add_provider_to_live_config",
    "remove_provider_from_live_config",
    "switch_provider",
    "import_default_config",
    "get_claude_desktop_status",
    "get_claude_desktop_default_routes",
    "import_claude_desktop_providers_from_claude",
    "ensure_claude_desktop_official_provider",
    "ensure_codex_official_provider",
    "ensure_grokbuild_official_provider",
    "get_claude_config_status",
    "get_config_status",
    "get_claude_code_config_path",
    "get_config_dir",
    "open_config_folder",
    "pick_directory",
    "open_external",
    "get_init_error",
    "get_migration_result",
    "get_skills_migration_result",
    "get_app_config_path",
    "open_app_config_folder",
    "reveal_app_log_file",
    "get_claude_common_config_snippet",
    "set_claude_common_config_snippet",
    "get_common_config_snippet",
    "set_common_config_snippet",
    "extract_common_config_snippet",
    "read_live_provider_settings",
    "get_settings",
    "save_settings",
    "has_codex_unify_history_backup",
    "codex_forces_multi_agent_v2",
    "get_codex_session_compression",
    "set_codex_session_compression",
    "get_codex_sessions_disk_usage",
    "restore_codex_unified_history",
    "get_rectifier_config",
    "set_rectifier_config",
    "get_optimizer_config",
    "set_optimizer_config",
    "get_copilot_optimizer_config",
    "set_copilot_optimizer_config",
    "get_log_config",
    "set_log_config",
    "restart_app",
    "install_update_and_restart",
    "check_app_update_available",
    "check_for_updates",
    "open_portable_update_download",
    "is_portable_mode",
    "copy_text_to_clipboard",
    "get_claude_plugin_status",
    "read_claude_plugin_config",
    "apply_claude_plugin_config",
    "is_claude_plugin_applied",
    "apply_claude_onboarding_skip",
    "clear_claude_onboarding_skip",
    "get_claude_mcp_status",
    "read_claude_mcp_config",
    "upsert_claude_mcp_server",
    "delete_claude_mcp_server",
    "validate_mcp_command",
    "queryProviderUsage",
    "testUsageScript",
    "get_subscription_quota",
    "get_codex_oauth_quota",
    "get_codex_oauth_models",
    "get_xai_oauth_models",
    "get_xai_oauth_quota",
    "get_coding_plan_quota",
    "get_balance",
    "get_mcp_config",
    "upsert_mcp_server_in_config",
    "delete_mcp_server_in_config",
    "set_mcp_enabled",
    "get_mcp_servers",
    "start_mcp_connectivity_checks",
    "cancel_mcp_connectivity_checks",
    "upsert_mcp_server",
    "delete_mcp_server",
    "toggle_mcp_app",
    "import_mcp_from_apps",
    "resync_mcp_to_apps",
    "get_prompts",
    "upsert_prompt",
    "delete_prompt",
    "enable_prompt",
    "import_prompt_from_file",
    "get_current_prompt_file_content",
    "get_prompt_file_location",
    "get_pi_prompt_file",
    "replace_pi_prompt_file",
    "delete_pi_prompt_file",
    "list_pi_prompt_templates",
    "upsert_pi_prompt_template",
    "delete_pi_prompt_template",
    "get_pi_current_state",
    "update_pi_provider_usage_script",
    "get_pi_session_discovery",
    "get_omp_current_state",
    "list_profiles",
    "create_profile",
    "update_profile",
    "delete_profile",
    "clear_current_profile",
    "apply_profile",
    "fetch_models_for_config",
    "get_agy_models",
    "get_opencode_models",
    "test_api_endpoints",
    "get_custom_endpoints",
    "add_custom_endpoint",
    "remove_custom_endpoint",
    "update_endpoint_last_used",
    "get_app_config_dir_override",
    "set_app_config_dir_override",
    "update_providers_sort_order",
    "export_config_to_file",
    "import_config_from_file",
    "webdav_test_connection",
    "webdav_sync_upload",
    "webdav_sync_download",
    "webdav_sync_save_settings",
    "webdav_sync_clear_settings",
    "webdav_sync_fetch_remote_info",
    "s3_test_connection",
    "s3_sync_upload",
    "s3_sync_download",
    "s3_sync_save_settings",
    "s3_sync_fetch_remote_info",
    "save_file_dialog",
    "open_file_dialog",
    "open_zip_file_dialog",
    "create_db_backup",
    "list_db_backups",
    "list_backup_locations",
    "delete_backup_location",
    "reveal_backup_location",
    "restore_db_backup",
    "rename_db_backup",
    "delete_db_backup",
    "sync_current_providers_live",
    "parse_deeplink",
    "merge_deeplink_config",
    "fetch_deeplink_icon",
    "import_from_deeplink",
    "import_from_deeplink_unified",
    "peek_pending_deeplink",
    "ack_pending_deeplink",
    "update_tray_menu",
    "check_env_conflicts",
    "delete_env_vars",
    "restore_env_backup",
    "get_installed_skills",
    "get_skill_backups",
    "delete_skill_backup",
    "install_skill_unified",
    "uninstall_skill_unified",
    "restore_skill_backup",
    "toggle_skill_app",
    "scan_unmanaged_skills",
    "import_skills_from_apps",
    "discover_available_skills",
    "discover_repo_skills",
    "check_skill_updates",
    "resync_skills_to_apps",
    "update_skill",
    "migrate_skill_storage",
    "get_cc_switch_skills_dir",
    "open_cc_switch_skills_dir",
    "open_skill_dir",
    "search_skills_sh",
    "get_skills",
    "get_skills_for_app",
    "install_skill",
    "install_skill_for_app",
    "uninstall_skill",
    "uninstall_skill_for_app",
    "get_skill_repos",
    "add_skill_repo",
    "remove_skill_repo",
    "install_skills_from_zip",
    "set_auto_launch",
    "get_auto_launch_status",
    "start_proxy_server",
    "stop_proxy_server",
    "stop_proxy_with_restore",
    "get_proxy_takeover_status",
    "set_proxy_takeover_for_app",
    "adopt_app_to_gateway",
    "release_app_from_gateway",
    "get_app_mode",
    "set_proxy_route",
    "take_startup_attach_failures",
    "take_tray_navigation",
    "tray_panel_state",
    "tray_panel_placement",
    "tray_panel_layout",
    "tray_panel_hide",
    "tray_panel_action",
    "tray_app_page_seen",
    "exit_proxy_apps_in_mode",
    "get_direct_provider",
    "get_proxy_status",
    "get_proxy_config",
    "update_proxy_config",
    "get_global_proxy_config",
    "update_global_proxy_config",
    "get_proxy_client_host",
    "set_proxy_client_host",
    "get_proxy_config_for_app",
    "update_proxy_config_for_app",
    "get_pricing_model_source",
    "set_pricing_model_source",
    "is_proxy_running",
    "is_live_takeover_active",
    "switch_proxy_provider",
    "get_proxy_stack",
    "set_proxy_stack_member",
    "adopt_codex_stack_catalog",
    "restart_codex_app_server_daemon",
    "acknowledge_codex_stale_clients",
    "get_provider_health",
    "reset_circuit_breaker",
    "get_circuit_breaker_config",
    "update_circuit_breaker_config",
    "get_circuit_breaker_stats",
    "get_failover_queue",
    "get_available_providers_for_failover",
    "add_to_failover_queue",
    "remove_from_failover_queue",
    "get_auto_failover_enabled",
    "set_auto_failover_enabled",
    "get_usage_summary",
    "get_session_usage_summary",
    "get_usage_summary_by_app",
    "get_usage_trends",
    "get_usage_first_date",
    "get_provider_stats",
    "get_model_stats",
    "get_request_logs",
    "get_request_detail",
    "get_model_pricing",
    "update_model_pricing",
    "update_model_pricing_batch",
    "delete_model_pricing",
    "get_models_dev_sync_config",
    "save_models_dev_sync_config",
    "record_models_dev_sync_result",
    "fetch_models_dev",
    "check_provider_limits",
    "sync_session_usage",
    "rebuild_codex_usage",
    "get_session_usage_last_sync",
    "get_usage_data_sources",
    "fetch_exchange_rates",
    "stream_check_provider",
    "list_sessions",
    "get_session_messages",
    "stream_session_messages",
    "get_session_block_content",
    "get_session_image",
    "reveal_session_path",
    "export_session_markdown",
    "delete_session",
    "delete_sessions",
    "launch_session_terminal",
    "get_tool_versions",
    "run_tool_lifecycle_action",
    "prepare_tool_uninstall",
    "probe_tool_installations",
    "list_tool_installations",
    "open_provider_terminal",
    "get_universal_providers",
    "get_universal_provider",
    "upsert_universal_provider",
    "delete_universal_provider",
    "sync_universal_provider",
    "get_custom_icons",
    "add_custom_icon",
    "rename_custom_icon",
    "delete_custom_icon",
    "import_opencode_providers_from_live",
    "get_opencode_live_provider_ids",
    "get_opencode_default_model",
    "set_opencode_default_model",
    "import_openclaw_providers_from_live",
    "get_openclaw_live_provider_ids",
    "get_openclaw_live_provider",
    "scan_openclaw_config_health",
    "get_openclaw_default_model",
    "set_openclaw_default_model",
    "get_openclaw_model_catalog",
    "set_openclaw_model_catalog",
    "get_openclaw_agents_defaults",
    "set_openclaw_agents_defaults",
    "get_openclaw_env",
    "set_openclaw_env",
    "get_openclaw_tools",
    "set_openclaw_tools",
    "import_hermes_providers_from_live",
    "get_hermes_live_provider_ids",
    "get_hermes_live_provider",
    "get_hermes_model_config",
    "open_hermes_web_ui",
    "launch_hermes_dashboard",
    "get_hermes_memory",
    "set_hermes_memory",
    "get_hermes_memory_limits",
    "set_hermes_memory_enabled",
    "get_global_proxy_url",
    "set_global_proxy_url",
    "test_proxy_url",
    "get_upstream_proxy_status",
    "scan_local_proxies",
    "set_window_theme",
    "auth_start_login",
    "auth_poll_for_account",
    "auth_cancel_login",
    "auth_list_accounts",
    "auth_get_status",
    "auth_remove_account",
    "auth_set_default_account",
    "auth_logout",
    "agy_account_is_running",
    "agy_account_quota",
    "claude_account_submit_paste",
    "get_claude_account_quota",
    "get_claude_default_plan",
    "get_claude_login_switch",
    "kimi_oauth_status",
    "dsh_account_status",
    "dsh_account_sign_out",
    "dsh_account_login_start",
    "dsh_account_login_poll",
    "dsh_account_login_cancel",
    "kimi_oauth_start",
    "kimi_oauth_poll",
    "kimi_oauth_cancel",
    "kimi_oauth_remove",
    "kimi_oauth_quota",
    "kimi_oauth_use_account",
    "kimi_oauth_models",
    "copilot_start_device_flow",
    "copilot_poll_for_auth",
    "copilot_poll_for_account",
    "copilot_list_accounts",
    "copilot_remove_account",
    "copilot_set_default_account",
    "copilot_get_auth_status",
    "copilot_logout",
    "copilot_is_authenticated",
    "copilot_get_token",
    "copilot_get_token_for_account",
    "copilot_get_models",
    "copilot_get_models_for_account",
    "copilot_get_usage",
    "copilot_get_usage_for_account",
    "read_omo_local_file",
    "get_current_omo_provider_id",
    "disable_current_omo",
    "read_omo_slim_local_file",
    "get_current_omo_slim_provider_id",
    "disable_current_omo_slim",
    "read_workspace_file",
    "write_workspace_file",
    "list_daily_memory_files",
    "read_daily_memory_file",
    "write_daily_memory_file",
    "delete_daily_memory_file",
    "search_daily_memory_files",
    "open_workspace_directory",
    "enter_lightweight_mode",
    "exit_lightweight_mode",
    "is_lightweight_mode",
];

/// 已桥接的命令数量。
pub(crate) const COMMAND_COUNT: usize = 415;

/// 命令名 → 所属源文件（模块）名。控制服务用它标注改动归属的界面域。
pub(crate) fn module_of(command: &str) -> Option<&'static str> {
    Some(match command {
        "agy_account_is_running" | "agy_account_quota" => "agy_account",
        "auth_start_login"
        | "auth_poll_for_account"
        | "auth_cancel_login"
        | "auth_list_accounts"
        | "auth_get_status"
        | "auth_remove_account"
        | "auth_set_default_account"
        | "auth_logout" => "auth",
        "get_balance" => "balance",
        "claude_account_submit_paste"
        | "get_claude_account_quota"
        | "get_claude_default_plan"
        | "get_claude_login_switch" => "claude_account",
        "get_codex_oauth_quota" | "get_codex_oauth_models" => "codex_oauth",
        "get_coding_plan_quota" => "coding_plan",
        "get_claude_config_status"
        | "get_config_status"
        | "get_claude_code_config_path"
        | "get_config_dir"
        | "open_config_folder"
        | "pick_directory"
        | "get_app_config_path"
        | "open_app_config_folder"
        | "reveal_app_log_file"
        | "get_claude_common_config_snippet"
        | "set_claude_common_config_snippet"
        | "get_common_config_snippet"
        | "set_common_config_snippet"
        | "extract_common_config_snippet" => "config",
        "copilot_start_device_flow"
        | "copilot_poll_for_auth"
        | "copilot_poll_for_account"
        | "copilot_list_accounts"
        | "copilot_remove_account"
        | "copilot_set_default_account"
        | "copilot_get_auth_status"
        | "copilot_logout"
        | "copilot_is_authenticated"
        | "copilot_get_token"
        | "copilot_get_token_for_account"
        | "copilot_get_models"
        | "copilot_get_models_for_account"
        | "copilot_get_usage"
        | "copilot_get_usage_for_account" => "copilot",
        "get_custom_icons" | "add_custom_icon" | "rename_custom_icon" | "delete_custom_icon" => {
            "custom_icon"
        }
        "parse_deeplink"
        | "merge_deeplink_config"
        | "fetch_deeplink_icon"
        | "import_from_deeplink"
        | "import_from_deeplink_unified"
        | "peek_pending_deeplink"
        | "ack_pending_deeplink" => "deeplink",
        "dsh_account_status"
        | "dsh_account_sign_out"
        | "dsh_account_login_start"
        | "dsh_account_login_poll"
        | "dsh_account_login_cancel" => "dsh",
        "check_env_conflicts" | "delete_env_vars" | "restore_env_backup" => "env",
        "get_failover_queue"
        | "get_available_providers_for_failover"
        | "add_to_failover_queue"
        | "remove_from_failover_queue"
        | "get_auto_failover_enabled"
        | "set_auto_failover_enabled" => "failover",
        "get_gateway_catalog"
        | "get_app_model_config"
        | "get_model_overrides"
        | "set_model_override"
        | "clear_model_override"
        | "adopt_opencode_to_gateway"
        | "release_opencode_from_gateway"
        | "get_opencode_gateway_status"
        | "adopt_hermes_to_gateway"
        | "get_hermes_gateway_status"
        | "release_hermes_from_gateway"
        | "adopt_openclaw_to_gateway"
        | "get_openclaw_gateway_status"
        | "release_openclaw_from_gateway"
        | "adopt_pi_to_gateway"
        | "get_pi_gateway_status"
        | "release_pi_from_gateway"
        | "adopt_omp_to_gateway"
        | "get_omp_gateway_status"
        | "release_omp_from_gateway"
        | "adopt_dsh_to_gateway"
        | "get_dsh_gateway_status"
        | "release_dsh_from_gateway"
        | "set_app_model_config"
        | "migrate_gateway_from_stack" => "gateway",
        "get_global_proxy_url"
        | "set_global_proxy_url"
        | "test_proxy_url"
        | "get_upstream_proxy_status"
        | "scan_local_proxies" => "global_proxy",
        "import_hermes_providers_from_live"
        | "get_hermes_live_provider_ids"
        | "get_hermes_live_provider"
        | "get_hermes_model_config"
        | "open_hermes_web_ui"
        | "launch_hermes_dashboard"
        | "get_hermes_memory"
        | "set_hermes_memory"
        | "get_hermes_memory_limits"
        | "set_hermes_memory_enabled" => "hermes",
        "export_config_to_file"
        | "import_config_from_file"
        | "save_file_dialog"
        | "open_file_dialog"
        | "open_zip_file_dialog"
        | "create_db_backup"
        | "list_db_backups"
        | "list_backup_locations"
        | "delete_backup_location"
        | "reveal_backup_location"
        | "restore_db_backup"
        | "rename_db_backup"
        | "delete_db_backup"
        | "sync_current_providers_live" => "import_export",
        "kimi_oauth_status"
        | "kimi_oauth_start"
        | "kimi_oauth_poll"
        | "kimi_oauth_cancel"
        | "kimi_oauth_remove"
        | "kimi_oauth_quota"
        | "kimi_oauth_use_account"
        | "kimi_oauth_models" => "kimi_oauth",
        "update_tray_menu" => "lib",
        "enter_lightweight_mode" | "exit_lightweight_mode" | "is_lightweight_mode" => "lightweight",
        "get_claude_mcp_status"
        | "read_claude_mcp_config"
        | "upsert_claude_mcp_server"
        | "delete_claude_mcp_server"
        | "validate_mcp_command"
        | "get_mcp_config"
        | "upsert_mcp_server_in_config"
        | "delete_mcp_server_in_config"
        | "set_mcp_enabled"
        | "get_mcp_servers"
        | "start_mcp_connectivity_checks"
        | "cancel_mcp_connectivity_checks"
        | "upsert_mcp_server"
        | "delete_mcp_server"
        | "toggle_mcp_app"
        | "import_mcp_from_apps"
        | "resync_mcp_to_apps" => "mcp",
        "open_external"
        | "get_init_error"
        | "get_migration_result"
        | "get_skills_migration_result"
        | "check_for_updates"
        | "open_portable_update_download"
        | "is_portable_mode"
        | "copy_text_to_clipboard"
        | "get_tool_versions"
        | "run_tool_lifecycle_action"
        | "prepare_tool_uninstall"
        | "probe_tool_installations"
        | "list_tool_installations"
        | "open_provider_terminal"
        | "set_window_theme" => "misc",
        "fetch_models_for_config" | "get_agy_models" | "get_opencode_models" => "model_fetch",
        "read_omo_local_file"
        | "get_current_omo_provider_id"
        | "disable_current_omo"
        | "read_omo_slim_local_file"
        | "get_current_omo_slim_provider_id"
        | "disable_current_omo_slim" => "omo",
        "get_omp_current_state" => "omp",
        "import_openclaw_providers_from_live"
        | "get_openclaw_live_provider_ids"
        | "get_openclaw_live_provider"
        | "scan_openclaw_config_health"
        | "get_openclaw_default_model"
        | "set_openclaw_default_model"
        | "get_openclaw_model_catalog"
        | "set_openclaw_model_catalog"
        | "get_openclaw_agents_defaults"
        | "set_openclaw_agents_defaults"
        | "get_openclaw_env"
        | "set_openclaw_env"
        | "get_openclaw_tools"
        | "set_openclaw_tools" => "openclaw",
        "get_pi_current_state" | "update_pi_provider_usage_script" | "get_pi_session_discovery" => {
            "pi"
        }
        "get_claude_plugin_status"
        | "read_claude_plugin_config"
        | "apply_claude_plugin_config"
        | "is_claude_plugin_applied"
        | "apply_claude_onboarding_skip"
        | "clear_claude_onboarding_skip" => "plugin",
        "list_profiles"
        | "create_profile"
        | "update_profile"
        | "delete_profile"
        | "clear_current_profile"
        | "apply_profile" => "profile",
        "get_prompts"
        | "upsert_prompt"
        | "delete_prompt"
        | "enable_prompt"
        | "import_prompt_from_file"
        | "get_current_prompt_file_content"
        | "get_prompt_file_location"
        | "get_pi_prompt_file"
        | "replace_pi_prompt_file"
        | "delete_pi_prompt_file"
        | "list_pi_prompt_templates"
        | "upsert_pi_prompt_template"
        | "delete_pi_prompt_template" => "prompt",
        "get_providers"
        | "get_current_provider"
        | "add_provider"
        | "update_provider"
        | "get_provider_editor_view"
        | "delete_provider"
        | "add_provider_to_live_config"
        | "remove_provider_from_live_config"
        | "switch_provider"
        | "import_default_config"
        | "get_claude_desktop_status"
        | "get_claude_desktop_default_routes"
        | "import_claude_desktop_providers_from_claude"
        | "ensure_claude_desktop_official_provider"
        | "ensure_codex_official_provider"
        | "ensure_grokbuild_official_provider"
        | "read_live_provider_settings"
        | "queryProviderUsage"
        | "testUsageScript"
        | "test_api_endpoints"
        | "get_custom_endpoints"
        | "add_custom_endpoint"
        | "remove_custom_endpoint"
        | "update_endpoint_last_used"
        | "update_providers_sort_order"
        | "get_universal_providers"
        | "get_universal_provider"
        | "upsert_universal_provider"
        | "delete_universal_provider"
        | "sync_universal_provider"
        | "import_opencode_providers_from_live"
        | "get_opencode_live_provider_ids"
        | "get_opencode_default_model"
        | "set_opencode_default_model" => "provider",
        "start_proxy_server"
        | "stop_proxy_server"
        | "stop_proxy_with_restore"
        | "get_proxy_takeover_status"
        | "set_proxy_takeover_for_app"
        | "adopt_app_to_gateway"
        | "release_app_from_gateway"
        | "get_app_mode"
        | "set_proxy_route"
        | "take_startup_attach_failures"
        | "exit_proxy_apps_in_mode"
        | "get_direct_provider"
        | "get_proxy_status"
        | "get_proxy_config"
        | "update_proxy_config"
        | "get_global_proxy_config"
        | "update_global_proxy_config"
        | "get_proxy_client_host"
        | "set_proxy_client_host"
        | "get_proxy_config_for_app"
        | "update_proxy_config_for_app"
        | "get_pricing_model_source"
        | "set_pricing_model_source"
        | "is_proxy_running"
        | "is_live_takeover_active"
        | "switch_proxy_provider"
        | "get_proxy_stack"
        | "set_proxy_stack_member"
        | "adopt_codex_stack_catalog"
        | "restart_codex_app_server_daemon"
        | "acknowledge_codex_stale_clients"
        | "get_provider_health"
        | "reset_circuit_breaker"
        | "get_circuit_breaker_config"
        | "update_circuit_breaker_config"
        | "get_circuit_breaker_stats" => "proxy",
        "s3_test_connection"
        | "s3_sync_upload"
        | "s3_sync_download"
        | "s3_sync_save_settings"
        | "s3_sync_fetch_remote_info" => "s3_sync",
        "list_sessions"
        | "get_session_messages"
        | "stream_session_messages"
        | "get_session_block_content"
        | "get_session_image"
        | "reveal_session_path"
        | "export_session_markdown"
        | "delete_session"
        | "delete_sessions"
        | "launch_session_terminal" => "session_manager",
        "get_settings"
        | "save_settings"
        | "has_codex_unify_history_backup"
        | "codex_forces_multi_agent_v2"
        | "get_codex_session_compression"
        | "set_codex_session_compression"
        | "get_codex_sessions_disk_usage"
        | "restore_codex_unified_history"
        | "get_rectifier_config"
        | "set_rectifier_config"
        | "get_optimizer_config"
        | "set_optimizer_config"
        | "get_copilot_optimizer_config"
        | "set_copilot_optimizer_config"
        | "get_log_config"
        | "set_log_config"
        | "restart_app"
        | "install_update_and_restart"
        | "check_app_update_available"
        | "get_app_config_dir_override"
        | "set_app_config_dir_override"
        | "set_auto_launch"
        | "get_auto_launch_status" => "settings",
        "get_installed_skills"
        | "get_skill_backups"
        | "delete_skill_backup"
        | "install_skill_unified"
        | "uninstall_skill_unified"
        | "restore_skill_backup"
        | "toggle_skill_app"
        | "scan_unmanaged_skills"
        | "import_skills_from_apps"
        | "discover_available_skills"
        | "discover_repo_skills"
        | "check_skill_updates"
        | "resync_skills_to_apps"
        | "update_skill"
        | "migrate_skill_storage"
        | "get_cc_switch_skills_dir"
        | "open_cc_switch_skills_dir"
        | "open_skill_dir"
        | "search_skills_sh"
        | "get_skills"
        | "get_skills_for_app"
        | "install_skill"
        | "install_skill_for_app"
        | "uninstall_skill"
        | "uninstall_skill_for_app"
        | "get_skill_repos"
        | "add_skill_repo"
        | "remove_skill_repo"
        | "install_skills_from_zip" => "skill",
        "stream_check_provider" => "stream_check",
        "get_subscription_quota" => "subscription",
        "get_suppliers_overview"
        | "update_supplier_base_url"
        | "adopt_supplier"
        | "create_supplier"
        | "update_supplier_secret"
        | "update_binding_model_map"
        | "set_supplier_enabled"
        | "delete_supplier" => "suppliers",
        "take_tray_navigation" | "tray_app_page_seen" => "tray",
        "tray_panel_state"
        | "tray_panel_placement"
        | "tray_panel_layout"
        | "tray_panel_hide"
        | "tray_panel_action" => "tray_panel",
        "get_usage_summary"
        | "get_session_usage_summary"
        | "get_usage_summary_by_app"
        | "get_usage_trends"
        | "get_usage_first_date"
        | "get_provider_stats"
        | "get_model_stats"
        | "get_request_logs"
        | "get_request_detail"
        | "get_model_pricing"
        | "update_model_pricing"
        | "update_model_pricing_batch"
        | "delete_model_pricing"
        | "get_models_dev_sync_config"
        | "save_models_dev_sync_config"
        | "record_models_dev_sync_result"
        | "fetch_models_dev"
        | "check_provider_limits"
        | "sync_session_usage"
        | "rebuild_codex_usage"
        | "get_session_usage_last_sync"
        | "get_usage_data_sources"
        | "fetch_exchange_rates" => "usage",
        "webdav_test_connection"
        | "webdav_sync_upload"
        | "webdav_sync_download"
        | "webdav_sync_save_settings"
        | "webdav_sync_clear_settings"
        | "webdav_sync_fetch_remote_info" => "webdav_sync",
        "read_workspace_file"
        | "write_workspace_file"
        | "list_daily_memory_files"
        | "read_daily_memory_file"
        | "write_daily_memory_file"
        | "delete_daily_memory_file"
        | "search_daily_memory_files"
        | "open_workspace_directory" => "workspace",
        "get_xai_oauth_models" | "get_xai_oauth_quota" => "xai_oauth",
        _ => return None,
    })
}

/// 命令名 → 适配函数。参数走和 Tauri IPC 相同的 camelCase key。
pub(crate) async fn dispatch(
    command: &str,
    __app: &AppHandle,
    __args: serde_json::Value,
) -> crate::ipc::Outcome {
    let mut __args = crate::ipc::Args::new(__args)?;
    match command {
        "get_providers" => crate::commands::provider::ipc_adapters::__ipc_get_providers(__app, &mut __args).await,
        "get_gateway_catalog" => crate::commands::gateway::ipc_adapters::__ipc_get_gateway_catalog(__app, &mut __args).await,
        "get_app_model_config" => crate::commands::gateway::ipc_adapters::__ipc_get_app_model_config(__app, &mut __args).await,
        "get_model_overrides" => crate::commands::gateway::ipc_adapters::__ipc_get_model_overrides(__app, &mut __args).await,
        "set_model_override" => crate::commands::gateway::ipc_adapters::__ipc_set_model_override(__app, &mut __args).await,
        "clear_model_override" => crate::commands::gateway::ipc_adapters::__ipc_clear_model_override(__app, &mut __args).await,
        "adopt_opencode_to_gateway" => crate::commands::gateway::ipc_adapters::__ipc_adopt_opencode_to_gateway(__app, &mut __args).await,
        "release_opencode_from_gateway" => crate::commands::gateway::ipc_adapters::__ipc_release_opencode_from_gateway(__app, &mut __args).await,
        "get_opencode_gateway_status" => crate::commands::gateway::ipc_adapters::__ipc_get_opencode_gateway_status(__app, &mut __args).await,
        "adopt_hermes_to_gateway" => crate::commands::gateway::ipc_adapters::__ipc_adopt_hermes_to_gateway(__app, &mut __args).await,
        "get_hermes_gateway_status" => crate::commands::gateway::ipc_adapters::__ipc_get_hermes_gateway_status(__app, &mut __args).await,
        "release_hermes_from_gateway" => crate::commands::gateway::ipc_adapters::__ipc_release_hermes_from_gateway(__app, &mut __args).await,
        "adopt_openclaw_to_gateway" => crate::commands::gateway::ipc_adapters::__ipc_adopt_openclaw_to_gateway(__app, &mut __args).await,
        "get_openclaw_gateway_status" => crate::commands::gateway::ipc_adapters::__ipc_get_openclaw_gateway_status(__app, &mut __args).await,
        "release_openclaw_from_gateway" => crate::commands::gateway::ipc_adapters::__ipc_release_openclaw_from_gateway(__app, &mut __args).await,
        "adopt_pi_to_gateway" => crate::commands::gateway::ipc_adapters::__ipc_adopt_pi_to_gateway(__app, &mut __args).await,
        "get_pi_gateway_status" => crate::commands::gateway::ipc_adapters::__ipc_get_pi_gateway_status(__app, &mut __args).await,
        "release_pi_from_gateway" => crate::commands::gateway::ipc_adapters::__ipc_release_pi_from_gateway(__app, &mut __args).await,
        "adopt_omp_to_gateway" => crate::commands::gateway::ipc_adapters::__ipc_adopt_omp_to_gateway(__app, &mut __args).await,
        "get_omp_gateway_status" => crate::commands::gateway::ipc_adapters::__ipc_get_omp_gateway_status(__app, &mut __args).await,
        "release_omp_from_gateway" => crate::commands::gateway::ipc_adapters::__ipc_release_omp_from_gateway(__app, &mut __args).await,
        "adopt_dsh_to_gateway" => crate::commands::gateway::ipc_adapters::__ipc_adopt_dsh_to_gateway(__app, &mut __args).await,
        "get_dsh_gateway_status" => crate::commands::gateway::ipc_adapters::__ipc_get_dsh_gateway_status(__app, &mut __args).await,
        "release_dsh_from_gateway" => crate::commands::gateway::ipc_adapters::__ipc_release_dsh_from_gateway(__app, &mut __args).await,
        "set_app_model_config" => crate::commands::gateway::ipc_adapters::__ipc_set_app_model_config(__app, &mut __args).await,
        "migrate_gateway_from_stack" => crate::commands::gateway::ipc_adapters::__ipc_migrate_gateway_from_stack(__app, &mut __args).await,
        "get_suppliers_overview" => crate::commands::suppliers::ipc_adapters::__ipc_get_suppliers_overview(__app, &mut __args).await,
        "update_supplier_base_url" => crate::commands::suppliers::ipc_adapters::__ipc_update_supplier_base_url(__app, &mut __args).await,
        "adopt_supplier" => crate::commands::suppliers::ipc_adapters::__ipc_adopt_supplier(__app, &mut __args).await,
        "create_supplier" => crate::commands::suppliers::ipc_adapters::__ipc_create_supplier(__app, &mut __args).await,
        "update_supplier_secret" => crate::commands::suppliers::ipc_adapters::__ipc_update_supplier_secret(__app, &mut __args).await,
        "update_binding_model_map" => crate::commands::suppliers::ipc_adapters::__ipc_update_binding_model_map(__app, &mut __args).await,
        "set_supplier_enabled" => crate::commands::suppliers::ipc_adapters::__ipc_set_supplier_enabled(__app, &mut __args).await,
        "delete_supplier" => crate::commands::suppliers::ipc_adapters::__ipc_delete_supplier(__app, &mut __args).await,
        "get_current_provider" => crate::commands::provider::ipc_adapters::__ipc_get_current_provider(__app, &mut __args).await,
        "add_provider" => crate::commands::provider::ipc_adapters::__ipc_add_provider(__app, &mut __args).await,
        "update_provider" => crate::commands::provider::ipc_adapters::__ipc_update_provider(__app, &mut __args).await,
        "get_provider_editor_view" => crate::commands::provider::ipc_adapters::__ipc_get_provider_editor_view(__app, &mut __args).await,
        "delete_provider" => crate::commands::provider::ipc_adapters::__ipc_delete_provider(__app, &mut __args).await,
        "add_provider_to_live_config" => crate::commands::provider::ipc_adapters::__ipc_add_provider_to_live_config(__app, &mut __args).await,
        "remove_provider_from_live_config" => crate::commands::provider::ipc_adapters::__ipc_remove_provider_from_live_config(__app, &mut __args).await,
        "switch_provider" => crate::commands::provider::ipc_adapters::__ipc_switch_provider(__app, &mut __args).await,
        "import_default_config" => crate::commands::provider::ipc_adapters::__ipc_import_default_config(__app, &mut __args).await,
        "get_claude_desktop_status" => crate::commands::provider::ipc_adapters::__ipc_get_claude_desktop_status(__app, &mut __args).await,
        "get_claude_desktop_default_routes" => crate::commands::provider::ipc_adapters::__ipc_get_claude_desktop_default_routes(__app, &mut __args).await,
        "import_claude_desktop_providers_from_claude" => crate::commands::provider::ipc_adapters::__ipc_import_claude_desktop_providers_from_claude(__app, &mut __args).await,
        "ensure_claude_desktop_official_provider" => crate::commands::provider::ipc_adapters::__ipc_ensure_claude_desktop_official_provider(__app, &mut __args).await,
        "ensure_codex_official_provider" => crate::commands::provider::ipc_adapters::__ipc_ensure_codex_official_provider(__app, &mut __args).await,
        "ensure_grokbuild_official_provider" => crate::commands::provider::ipc_adapters::__ipc_ensure_grokbuild_official_provider(__app, &mut __args).await,
        "get_claude_config_status" => crate::commands::config::ipc_adapters::__ipc_get_claude_config_status(__app, &mut __args).await,
        "get_config_status" => crate::commands::config::ipc_adapters::__ipc_get_config_status(__app, &mut __args).await,
        "get_claude_code_config_path" => crate::commands::config::ipc_adapters::__ipc_get_claude_code_config_path(__app, &mut __args).await,
        "get_config_dir" => crate::commands::config::ipc_adapters::__ipc_get_config_dir(__app, &mut __args).await,
        "open_config_folder" => crate::commands::config::ipc_adapters::__ipc_open_config_folder(__app, &mut __args).await,
        "pick_directory" => crate::commands::config::ipc_adapters::__ipc_pick_directory(__app, &mut __args).await,
        "open_external" => crate::commands::misc::ipc_adapters::__ipc_open_external(__app, &mut __args).await,
        "get_init_error" => crate::commands::misc::ipc_adapters::__ipc_get_init_error(__app, &mut __args).await,
        "get_migration_result" => crate::commands::misc::ipc_adapters::__ipc_get_migration_result(__app, &mut __args).await,
        "get_skills_migration_result" => crate::commands::misc::ipc_adapters::__ipc_get_skills_migration_result(__app, &mut __args).await,
        "get_app_config_path" => crate::commands::config::ipc_adapters::__ipc_get_app_config_path(__app, &mut __args).await,
        "open_app_config_folder" => crate::commands::config::ipc_adapters::__ipc_open_app_config_folder(__app, &mut __args).await,
        "reveal_app_log_file" => crate::commands::config::ipc_adapters::__ipc_reveal_app_log_file(__app, &mut __args).await,
        "get_claude_common_config_snippet" => crate::commands::config::ipc_adapters::__ipc_get_claude_common_config_snippet(__app, &mut __args).await,
        "set_claude_common_config_snippet" => crate::commands::config::ipc_adapters::__ipc_set_claude_common_config_snippet(__app, &mut __args).await,
        "get_common_config_snippet" => crate::commands::config::ipc_adapters::__ipc_get_common_config_snippet(__app, &mut __args).await,
        "set_common_config_snippet" => crate::commands::config::ipc_adapters::__ipc_set_common_config_snippet(__app, &mut __args).await,
        "extract_common_config_snippet" => crate::commands::config::ipc_adapters::__ipc_extract_common_config_snippet(__app, &mut __args).await,
        "read_live_provider_settings" => crate::commands::provider::ipc_adapters::__ipc_read_live_provider_settings(__app, &mut __args).await,
        "get_settings" => crate::commands::settings::ipc_adapters::__ipc_get_settings(__app, &mut __args).await,
        "save_settings" => crate::commands::settings::ipc_adapters::__ipc_save_settings(__app, &mut __args).await,
        "has_codex_unify_history_backup" => crate::commands::settings::ipc_adapters::__ipc_has_codex_unify_history_backup(__app, &mut __args).await,
        "codex_forces_multi_agent_v2" => crate::commands::settings::ipc_adapters::__ipc_codex_forces_multi_agent_v2(__app, &mut __args).await,
        "get_codex_session_compression" => crate::commands::settings::ipc_adapters::__ipc_get_codex_session_compression(__app, &mut __args).await,
        "set_codex_session_compression" => crate::commands::settings::ipc_adapters::__ipc_set_codex_session_compression(__app, &mut __args).await,
        "get_codex_sessions_disk_usage" => crate::commands::settings::ipc_adapters::__ipc_get_codex_sessions_disk_usage(__app, &mut __args).await,
        "restore_codex_unified_history" => crate::commands::settings::ipc_adapters::__ipc_restore_codex_unified_history(__app, &mut __args).await,
        "get_rectifier_config" => crate::commands::settings::ipc_adapters::__ipc_get_rectifier_config(__app, &mut __args).await,
        "set_rectifier_config" => crate::commands::settings::ipc_adapters::__ipc_set_rectifier_config(__app, &mut __args).await,
        "get_optimizer_config" => crate::commands::settings::ipc_adapters::__ipc_get_optimizer_config(__app, &mut __args).await,
        "set_optimizer_config" => crate::commands::settings::ipc_adapters::__ipc_set_optimizer_config(__app, &mut __args).await,
        "get_copilot_optimizer_config" => crate::commands::settings::ipc_adapters::__ipc_get_copilot_optimizer_config(__app, &mut __args).await,
        "set_copilot_optimizer_config" => crate::commands::settings::ipc_adapters::__ipc_set_copilot_optimizer_config(__app, &mut __args).await,
        "get_log_config" => crate::commands::settings::ipc_adapters::__ipc_get_log_config(__app, &mut __args).await,
        "set_log_config" => crate::commands::settings::ipc_adapters::__ipc_set_log_config(__app, &mut __args).await,
        "restart_app" => crate::commands::settings::ipc_adapters::__ipc_restart_app(__app, &mut __args).await,
        "install_update_and_restart" => crate::commands::settings::ipc_adapters::__ipc_install_update_and_restart(__app, &mut __args).await,
        "check_app_update_available" => crate::commands::settings::ipc_adapters::__ipc_check_app_update_available(__app, &mut __args).await,
        "check_for_updates" => crate::commands::misc::ipc_adapters::__ipc_check_for_updates(__app, &mut __args).await,
        "open_portable_update_download" => crate::commands::misc::ipc_adapters::__ipc_open_portable_update_download(__app, &mut __args).await,
        "is_portable_mode" => crate::commands::misc::ipc_adapters::__ipc_is_portable_mode(__app, &mut __args).await,
        "copy_text_to_clipboard" => crate::commands::misc::ipc_adapters::__ipc_copy_text_to_clipboard(__app, &mut __args).await,
        "get_claude_plugin_status" => crate::commands::plugin::ipc_adapters::__ipc_get_claude_plugin_status(__app, &mut __args).await,
        "read_claude_plugin_config" => crate::commands::plugin::ipc_adapters::__ipc_read_claude_plugin_config(__app, &mut __args).await,
        "apply_claude_plugin_config" => crate::commands::plugin::ipc_adapters::__ipc_apply_claude_plugin_config(__app, &mut __args).await,
        "is_claude_plugin_applied" => crate::commands::plugin::ipc_adapters::__ipc_is_claude_plugin_applied(__app, &mut __args).await,
        "apply_claude_onboarding_skip" => crate::commands::plugin::ipc_adapters::__ipc_apply_claude_onboarding_skip(__app, &mut __args).await,
        "clear_claude_onboarding_skip" => crate::commands::plugin::ipc_adapters::__ipc_clear_claude_onboarding_skip(__app, &mut __args).await,
        "get_claude_mcp_status" => crate::commands::mcp::ipc_adapters::__ipc_get_claude_mcp_status(__app, &mut __args).await,
        "read_claude_mcp_config" => crate::commands::mcp::ipc_adapters::__ipc_read_claude_mcp_config(__app, &mut __args).await,
        "upsert_claude_mcp_server" => crate::commands::mcp::ipc_adapters::__ipc_upsert_claude_mcp_server(__app, &mut __args).await,
        "delete_claude_mcp_server" => crate::commands::mcp::ipc_adapters::__ipc_delete_claude_mcp_server(__app, &mut __args).await,
        "validate_mcp_command" => crate::commands::mcp::ipc_adapters::__ipc_validate_mcp_command(__app, &mut __args).await,
        "queryProviderUsage" => crate::commands::provider::ipc_adapters::__ipc_queryProviderUsage(__app, &mut __args).await,
        "testUsageScript" => crate::commands::provider::ipc_adapters::__ipc_testUsageScript(__app, &mut __args).await,
        "get_subscription_quota" => crate::commands::subscription::ipc_adapters::__ipc_get_subscription_quota(__app, &mut __args).await,
        "get_codex_oauth_quota" => crate::commands::codex_oauth::ipc_adapters::__ipc_get_codex_oauth_quota(__app, &mut __args).await,
        "get_codex_oauth_models" => crate::commands::codex_oauth::ipc_adapters::__ipc_get_codex_oauth_models(__app, &mut __args).await,
        "get_xai_oauth_models" => crate::commands::xai_oauth::ipc_adapters::__ipc_get_xai_oauth_models(__app, &mut __args).await,
        "get_xai_oauth_quota" => crate::commands::xai_oauth::ipc_adapters::__ipc_get_xai_oauth_quota(__app, &mut __args).await,
        "get_coding_plan_quota" => crate::commands::coding_plan::ipc_adapters::__ipc_get_coding_plan_quota(__app, &mut __args).await,
        "get_balance" => crate::commands::balance::ipc_adapters::__ipc_get_balance(__app, &mut __args).await,
        "get_mcp_config" => crate::commands::mcp::ipc_adapters::__ipc_get_mcp_config(__app, &mut __args).await,
        "upsert_mcp_server_in_config" => crate::commands::mcp::ipc_adapters::__ipc_upsert_mcp_server_in_config(__app, &mut __args).await,
        "delete_mcp_server_in_config" => crate::commands::mcp::ipc_adapters::__ipc_delete_mcp_server_in_config(__app, &mut __args).await,
        "set_mcp_enabled" => crate::commands::mcp::ipc_adapters::__ipc_set_mcp_enabled(__app, &mut __args).await,
        "get_mcp_servers" => crate::commands::mcp::ipc_adapters::__ipc_get_mcp_servers(__app, &mut __args).await,
        "start_mcp_connectivity_checks" => crate::commands::mcp::ipc_adapters::__ipc_start_mcp_connectivity_checks(__app, &mut __args).await,
        "cancel_mcp_connectivity_checks" => crate::commands::mcp::ipc_adapters::__ipc_cancel_mcp_connectivity_checks(__app, &mut __args).await,
        "upsert_mcp_server" => crate::commands::mcp::ipc_adapters::__ipc_upsert_mcp_server(__app, &mut __args).await,
        "delete_mcp_server" => crate::commands::mcp::ipc_adapters::__ipc_delete_mcp_server(__app, &mut __args).await,
        "toggle_mcp_app" => crate::commands::mcp::ipc_adapters::__ipc_toggle_mcp_app(__app, &mut __args).await,
        "import_mcp_from_apps" => crate::commands::mcp::ipc_adapters::__ipc_import_mcp_from_apps(__app, &mut __args).await,
        "resync_mcp_to_apps" => crate::commands::mcp::ipc_adapters::__ipc_resync_mcp_to_apps(__app, &mut __args).await,
        "get_prompts" => crate::commands::prompt::ipc_adapters::__ipc_get_prompts(__app, &mut __args).await,
        "upsert_prompt" => crate::commands::prompt::ipc_adapters::__ipc_upsert_prompt(__app, &mut __args).await,
        "delete_prompt" => crate::commands::prompt::ipc_adapters::__ipc_delete_prompt(__app, &mut __args).await,
        "enable_prompt" => crate::commands::prompt::ipc_adapters::__ipc_enable_prompt(__app, &mut __args).await,
        "import_prompt_from_file" => crate::commands::prompt::ipc_adapters::__ipc_import_prompt_from_file(__app, &mut __args).await,
        "get_current_prompt_file_content" => crate::commands::prompt::ipc_adapters::__ipc_get_current_prompt_file_content(__app, &mut __args).await,
        "get_prompt_file_location" => crate::commands::prompt::ipc_adapters::__ipc_get_prompt_file_location(__app, &mut __args).await,
        "get_pi_prompt_file" => crate::commands::prompt::ipc_adapters::__ipc_get_pi_prompt_file(__app, &mut __args).await,
        "replace_pi_prompt_file" => crate::commands::prompt::ipc_adapters::__ipc_replace_pi_prompt_file(__app, &mut __args).await,
        "delete_pi_prompt_file" => crate::commands::prompt::ipc_adapters::__ipc_delete_pi_prompt_file(__app, &mut __args).await,
        "list_pi_prompt_templates" => crate::commands::prompt::ipc_adapters::__ipc_list_pi_prompt_templates(__app, &mut __args).await,
        "upsert_pi_prompt_template" => crate::commands::prompt::ipc_adapters::__ipc_upsert_pi_prompt_template(__app, &mut __args).await,
        "delete_pi_prompt_template" => crate::commands::prompt::ipc_adapters::__ipc_delete_pi_prompt_template(__app, &mut __args).await,
        "get_pi_current_state" => crate::commands::pi::ipc_adapters::__ipc_get_pi_current_state(__app, &mut __args).await,
        "update_pi_provider_usage_script" => crate::commands::pi::ipc_adapters::__ipc_update_pi_provider_usage_script(__app, &mut __args).await,
        "get_pi_session_discovery" => crate::commands::pi::ipc_adapters::__ipc_get_pi_session_discovery(__app, &mut __args).await,
        "get_omp_current_state" => crate::commands::omp::ipc_adapters::__ipc_get_omp_current_state(__app, &mut __args).await,
        "list_profiles" => crate::commands::profile::ipc_adapters::__ipc_list_profiles(__app, &mut __args).await,
        "create_profile" => crate::commands::profile::ipc_adapters::__ipc_create_profile(__app, &mut __args).await,
        "update_profile" => crate::commands::profile::ipc_adapters::__ipc_update_profile(__app, &mut __args).await,
        "delete_profile" => crate::commands::profile::ipc_adapters::__ipc_delete_profile(__app, &mut __args).await,
        "clear_current_profile" => crate::commands::profile::ipc_adapters::__ipc_clear_current_profile(__app, &mut __args).await,
        "apply_profile" => crate::commands::profile::ipc_adapters::__ipc_apply_profile(__app, &mut __args).await,
        "fetch_models_for_config" => crate::commands::model_fetch::ipc_adapters::__ipc_fetch_models_for_config(__app, &mut __args).await,
        "get_agy_models" => crate::commands::model_fetch::ipc_adapters::__ipc_get_agy_models(__app, &mut __args).await,
        "get_opencode_models" => crate::commands::model_fetch::ipc_adapters::__ipc_get_opencode_models(__app, &mut __args).await,
        "test_api_endpoints" => crate::commands::provider::ipc_adapters::__ipc_test_api_endpoints(__app, &mut __args).await,
        "get_custom_endpoints" => crate::commands::provider::ipc_adapters::__ipc_get_custom_endpoints(__app, &mut __args).await,
        "add_custom_endpoint" => crate::commands::provider::ipc_adapters::__ipc_add_custom_endpoint(__app, &mut __args).await,
        "remove_custom_endpoint" => crate::commands::provider::ipc_adapters::__ipc_remove_custom_endpoint(__app, &mut __args).await,
        "update_endpoint_last_used" => crate::commands::provider::ipc_adapters::__ipc_update_endpoint_last_used(__app, &mut __args).await,
        "get_app_config_dir_override" => crate::commands::settings::ipc_adapters::__ipc_get_app_config_dir_override(__app, &mut __args).await,
        "set_app_config_dir_override" => crate::commands::settings::ipc_adapters::__ipc_set_app_config_dir_override(__app, &mut __args).await,
        "update_providers_sort_order" => crate::commands::provider::ipc_adapters::__ipc_update_providers_sort_order(__app, &mut __args).await,
        "export_config_to_file" => crate::commands::import_export::ipc_adapters::__ipc_export_config_to_file(__app, &mut __args).await,
        "import_config_from_file" => crate::commands::import_export::ipc_adapters::__ipc_import_config_from_file(__app, &mut __args).await,
        "webdav_test_connection" => crate::commands::webdav_sync::ipc_adapters::__ipc_webdav_test_connection(__app, &mut __args).await,
        "webdav_sync_upload" => crate::commands::webdav_sync::ipc_adapters::__ipc_webdav_sync_upload(__app, &mut __args).await,
        "webdav_sync_download" => crate::commands::webdav_sync::ipc_adapters::__ipc_webdav_sync_download(__app, &mut __args).await,
        "webdav_sync_save_settings" => crate::commands::webdav_sync::ipc_adapters::__ipc_webdav_sync_save_settings(__app, &mut __args).await,
        "webdav_sync_clear_settings" => crate::commands::webdav_sync::ipc_adapters::__ipc_webdav_sync_clear_settings(__app, &mut __args).await,
        "webdav_sync_fetch_remote_info" => crate::commands::webdav_sync::ipc_adapters::__ipc_webdav_sync_fetch_remote_info(__app, &mut __args).await,
        "s3_test_connection" => crate::commands::s3_sync::ipc_adapters::__ipc_s3_test_connection(__app, &mut __args).await,
        "s3_sync_upload" => crate::commands::s3_sync::ipc_adapters::__ipc_s3_sync_upload(__app, &mut __args).await,
        "s3_sync_download" => crate::commands::s3_sync::ipc_adapters::__ipc_s3_sync_download(__app, &mut __args).await,
        "s3_sync_save_settings" => crate::commands::s3_sync::ipc_adapters::__ipc_s3_sync_save_settings(__app, &mut __args).await,
        "s3_sync_fetch_remote_info" => crate::commands::s3_sync::ipc_adapters::__ipc_s3_sync_fetch_remote_info(__app, &mut __args).await,
        "save_file_dialog" => crate::commands::import_export::ipc_adapters::__ipc_save_file_dialog(__app, &mut __args).await,
        "open_file_dialog" => crate::commands::import_export::ipc_adapters::__ipc_open_file_dialog(__app, &mut __args).await,
        "open_zip_file_dialog" => crate::commands::import_export::ipc_adapters::__ipc_open_zip_file_dialog(__app, &mut __args).await,
        "create_db_backup" => crate::commands::import_export::ipc_adapters::__ipc_create_db_backup(__app, &mut __args).await,
        "list_db_backups" => crate::commands::import_export::ipc_adapters::__ipc_list_db_backups(__app, &mut __args).await,
        "list_backup_locations" => crate::commands::import_export::ipc_adapters::__ipc_list_backup_locations(__app, &mut __args).await,
        "delete_backup_location" => crate::commands::import_export::ipc_adapters::__ipc_delete_backup_location(__app, &mut __args).await,
        "reveal_backup_location" => crate::commands::import_export::ipc_adapters::__ipc_reveal_backup_location(__app, &mut __args).await,
        "restore_db_backup" => crate::commands::import_export::ipc_adapters::__ipc_restore_db_backup(__app, &mut __args).await,
        "rename_db_backup" => crate::commands::import_export::ipc_adapters::__ipc_rename_db_backup(__app, &mut __args).await,
        "delete_db_backup" => crate::commands::import_export::ipc_adapters::__ipc_delete_db_backup(__app, &mut __args).await,
        "sync_current_providers_live" => crate::commands::import_export::ipc_adapters::__ipc_sync_current_providers_live(__app, &mut __args).await,
        "parse_deeplink" => crate::commands::deeplink::ipc_adapters::__ipc_parse_deeplink(__app, &mut __args).await,
        "merge_deeplink_config" => crate::commands::deeplink::ipc_adapters::__ipc_merge_deeplink_config(__app, &mut __args).await,
        "fetch_deeplink_icon" => crate::commands::deeplink::ipc_adapters::__ipc_fetch_deeplink_icon(__app, &mut __args).await,
        "import_from_deeplink" => crate::commands::deeplink::ipc_adapters::__ipc_import_from_deeplink(__app, &mut __args).await,
        "import_from_deeplink_unified" => crate::commands::deeplink::ipc_adapters::__ipc_import_from_deeplink_unified(__app, &mut __args).await,
        "peek_pending_deeplink" => crate::commands::deeplink::ipc_adapters::__ipc_peek_pending_deeplink(__app, &mut __args).await,
        "ack_pending_deeplink" => crate::commands::deeplink::ipc_adapters::__ipc_ack_pending_deeplink(__app, &mut __args).await,
        "update_tray_menu" => crate::ipc_adapters::__ipc_update_tray_menu(__app, &mut __args).await,
        "check_env_conflicts" => crate::commands::env::ipc_adapters::__ipc_check_env_conflicts(__app, &mut __args).await,
        "delete_env_vars" => crate::commands::env::ipc_adapters::__ipc_delete_env_vars(__app, &mut __args).await,
        "restore_env_backup" => crate::commands::env::ipc_adapters::__ipc_restore_env_backup(__app, &mut __args).await,
        "get_installed_skills" => crate::commands::skill::ipc_adapters::__ipc_get_installed_skills(__app, &mut __args).await,
        "get_skill_backups" => crate::commands::skill::ipc_adapters::__ipc_get_skill_backups(__app, &mut __args).await,
        "delete_skill_backup" => crate::commands::skill::ipc_adapters::__ipc_delete_skill_backup(__app, &mut __args).await,
        "install_skill_unified" => crate::commands::skill::ipc_adapters::__ipc_install_skill_unified(__app, &mut __args).await,
        "uninstall_skill_unified" => crate::commands::skill::ipc_adapters::__ipc_uninstall_skill_unified(__app, &mut __args).await,
        "restore_skill_backup" => crate::commands::skill::ipc_adapters::__ipc_restore_skill_backup(__app, &mut __args).await,
        "toggle_skill_app" => crate::commands::skill::ipc_adapters::__ipc_toggle_skill_app(__app, &mut __args).await,
        "scan_unmanaged_skills" => crate::commands::skill::ipc_adapters::__ipc_scan_unmanaged_skills(__app, &mut __args).await,
        "import_skills_from_apps" => crate::commands::skill::ipc_adapters::__ipc_import_skills_from_apps(__app, &mut __args).await,
        "discover_available_skills" => crate::commands::skill::ipc_adapters::__ipc_discover_available_skills(__app, &mut __args).await,
        "discover_repo_skills" => crate::commands::skill::ipc_adapters::__ipc_discover_repo_skills(__app, &mut __args).await,
        "check_skill_updates" => crate::commands::skill::ipc_adapters::__ipc_check_skill_updates(__app, &mut __args).await,
        "resync_skills_to_apps" => crate::commands::skill::ipc_adapters::__ipc_resync_skills_to_apps(__app, &mut __args).await,
        "update_skill" => crate::commands::skill::ipc_adapters::__ipc_update_skill(__app, &mut __args).await,
        "migrate_skill_storage" => crate::commands::skill::ipc_adapters::__ipc_migrate_skill_storage(__app, &mut __args).await,
        "get_cc_switch_skills_dir" => crate::commands::skill::ipc_adapters::__ipc_get_cc_switch_skills_dir(__app, &mut __args).await,
        "open_cc_switch_skills_dir" => crate::commands::skill::ipc_adapters::__ipc_open_cc_switch_skills_dir(__app, &mut __args).await,
        "open_skill_dir" => crate::commands::skill::ipc_adapters::__ipc_open_skill_dir(__app, &mut __args).await,
        "search_skills_sh" => crate::commands::skill::ipc_adapters::__ipc_search_skills_sh(__app, &mut __args).await,
        "get_skills" => crate::commands::skill::ipc_adapters::__ipc_get_skills(__app, &mut __args).await,
        "get_skills_for_app" => crate::commands::skill::ipc_adapters::__ipc_get_skills_for_app(__app, &mut __args).await,
        "install_skill" => crate::commands::skill::ipc_adapters::__ipc_install_skill(__app, &mut __args).await,
        "install_skill_for_app" => crate::commands::skill::ipc_adapters::__ipc_install_skill_for_app(__app, &mut __args).await,
        "uninstall_skill" => crate::commands::skill::ipc_adapters::__ipc_uninstall_skill(__app, &mut __args).await,
        "uninstall_skill_for_app" => crate::commands::skill::ipc_adapters::__ipc_uninstall_skill_for_app(__app, &mut __args).await,
        "get_skill_repos" => crate::commands::skill::ipc_adapters::__ipc_get_skill_repos(__app, &mut __args).await,
        "add_skill_repo" => crate::commands::skill::ipc_adapters::__ipc_add_skill_repo(__app, &mut __args).await,
        "remove_skill_repo" => crate::commands::skill::ipc_adapters::__ipc_remove_skill_repo(__app, &mut __args).await,
        "install_skills_from_zip" => crate::commands::skill::ipc_adapters::__ipc_install_skills_from_zip(__app, &mut __args).await,
        "set_auto_launch" => crate::commands::settings::ipc_adapters::__ipc_set_auto_launch(__app, &mut __args).await,
        "get_auto_launch_status" => crate::commands::settings::ipc_adapters::__ipc_get_auto_launch_status(__app, &mut __args).await,
        "start_proxy_server" => crate::commands::proxy::ipc_adapters::__ipc_start_proxy_server(__app, &mut __args).await,
        "stop_proxy_server" => crate::commands::proxy::ipc_adapters::__ipc_stop_proxy_server(__app, &mut __args).await,
        "stop_proxy_with_restore" => crate::commands::proxy::ipc_adapters::__ipc_stop_proxy_with_restore(__app, &mut __args).await,
        "get_proxy_takeover_status" => crate::commands::proxy::ipc_adapters::__ipc_get_proxy_takeover_status(__app, &mut __args).await,
        "set_proxy_takeover_for_app" => crate::commands::proxy::ipc_adapters::__ipc_set_proxy_takeover_for_app(__app, &mut __args).await,
        "adopt_app_to_gateway" => crate::commands::proxy::ipc_adapters::__ipc_adopt_app_to_gateway(__app, &mut __args).await,
        "release_app_from_gateway" => crate::commands::proxy::ipc_adapters::__ipc_release_app_from_gateway(__app, &mut __args).await,
        "get_app_mode" => crate::commands::proxy::ipc_adapters::__ipc_get_app_mode(__app, &mut __args).await,
        "set_proxy_route" => crate::commands::proxy::ipc_adapters::__ipc_set_proxy_route(__app, &mut __args).await,
        "take_startup_attach_failures" => crate::commands::proxy::ipc_adapters::__ipc_take_startup_attach_failures(__app, &mut __args).await,
        "take_tray_navigation" => crate::tray::ipc_adapters::__ipc_take_tray_navigation(__app, &mut __args).await,
        "tray_panel_state" => crate::tray_panel::ipc_adapters::__ipc_tray_panel_state(__app, &mut __args).await,
        "tray_panel_placement" => crate::tray_panel::ipc_adapters::__ipc_tray_panel_placement(__app, &mut __args).await,
        "tray_panel_layout" => crate::tray_panel::ipc_adapters::__ipc_tray_panel_layout(__app, &mut __args).await,
        "tray_panel_hide" => crate::tray_panel::ipc_adapters::__ipc_tray_panel_hide(__app, &mut __args).await,
        "tray_panel_action" => crate::tray_panel::ipc_adapters::__ipc_tray_panel_action(__app, &mut __args).await,
        "tray_app_page_seen" => crate::tray::ipc_adapters::__ipc_tray_app_page_seen(__app, &mut __args).await,
        "exit_proxy_apps_in_mode" => crate::commands::proxy::ipc_adapters::__ipc_exit_proxy_apps_in_mode(__app, &mut __args).await,
        "get_direct_provider" => crate::commands::proxy::ipc_adapters::__ipc_get_direct_provider(__app, &mut __args).await,
        "get_proxy_status" => crate::commands::proxy::ipc_adapters::__ipc_get_proxy_status(__app, &mut __args).await,
        "get_proxy_config" => crate::commands::proxy::ipc_adapters::__ipc_get_proxy_config(__app, &mut __args).await,
        "update_proxy_config" => crate::commands::proxy::ipc_adapters::__ipc_update_proxy_config(__app, &mut __args).await,
        "get_global_proxy_config" => crate::commands::proxy::ipc_adapters::__ipc_get_global_proxy_config(__app, &mut __args).await,
        "update_global_proxy_config" => crate::commands::proxy::ipc_adapters::__ipc_update_global_proxy_config(__app, &mut __args).await,
        "get_proxy_client_host" => crate::commands::proxy::ipc_adapters::__ipc_get_proxy_client_host(__app, &mut __args).await,
        "set_proxy_client_host" => crate::commands::proxy::ipc_adapters::__ipc_set_proxy_client_host(__app, &mut __args).await,
        "get_proxy_config_for_app" => crate::commands::proxy::ipc_adapters::__ipc_get_proxy_config_for_app(__app, &mut __args).await,
        "update_proxy_config_for_app" => crate::commands::proxy::ipc_adapters::__ipc_update_proxy_config_for_app(__app, &mut __args).await,
        "get_pricing_model_source" => crate::commands::proxy::ipc_adapters::__ipc_get_pricing_model_source(__app, &mut __args).await,
        "set_pricing_model_source" => crate::commands::proxy::ipc_adapters::__ipc_set_pricing_model_source(__app, &mut __args).await,
        "is_proxy_running" => crate::commands::proxy::ipc_adapters::__ipc_is_proxy_running(__app, &mut __args).await,
        "is_live_takeover_active" => crate::commands::proxy::ipc_adapters::__ipc_is_live_takeover_active(__app, &mut __args).await,
        "switch_proxy_provider" => crate::commands::proxy::ipc_adapters::__ipc_switch_proxy_provider(__app, &mut __args).await,
        "get_proxy_stack" => crate::commands::proxy::ipc_adapters::__ipc_get_proxy_stack(__app, &mut __args).await,
        "set_proxy_stack_member" => crate::commands::proxy::ipc_adapters::__ipc_set_proxy_stack_member(__app, &mut __args).await,
        "adopt_codex_stack_catalog" => crate::commands::proxy::ipc_adapters::__ipc_adopt_codex_stack_catalog(__app, &mut __args).await,
        "restart_codex_app_server_daemon" => crate::commands::proxy::ipc_adapters::__ipc_restart_codex_app_server_daemon(__app, &mut __args).await,
        "acknowledge_codex_stale_clients" => crate::commands::proxy::ipc_adapters::__ipc_acknowledge_codex_stale_clients(__app, &mut __args).await,
        "get_provider_health" => crate::commands::proxy::ipc_adapters::__ipc_get_provider_health(__app, &mut __args).await,
        "reset_circuit_breaker" => crate::commands::proxy::ipc_adapters::__ipc_reset_circuit_breaker(__app, &mut __args).await,
        "get_circuit_breaker_config" => crate::commands::proxy::ipc_adapters::__ipc_get_circuit_breaker_config(__app, &mut __args).await,
        "update_circuit_breaker_config" => crate::commands::proxy::ipc_adapters::__ipc_update_circuit_breaker_config(__app, &mut __args).await,
        "get_circuit_breaker_stats" => crate::commands::proxy::ipc_adapters::__ipc_get_circuit_breaker_stats(__app, &mut __args).await,
        "get_failover_queue" => crate::commands::failover::ipc_adapters::__ipc_get_failover_queue(__app, &mut __args).await,
        "get_available_providers_for_failover" => crate::commands::failover::ipc_adapters::__ipc_get_available_providers_for_failover(__app, &mut __args).await,
        "add_to_failover_queue" => crate::commands::failover::ipc_adapters::__ipc_add_to_failover_queue(__app, &mut __args).await,
        "remove_from_failover_queue" => crate::commands::failover::ipc_adapters::__ipc_remove_from_failover_queue(__app, &mut __args).await,
        "get_auto_failover_enabled" => crate::commands::failover::ipc_adapters::__ipc_get_auto_failover_enabled(__app, &mut __args).await,
        "set_auto_failover_enabled" => crate::commands::failover::ipc_adapters::__ipc_set_auto_failover_enabled(__app, &mut __args).await,
        "get_usage_summary" => crate::commands::usage::ipc_adapters::__ipc_get_usage_summary(__app, &mut __args).await,
        "get_session_usage_summary" => crate::commands::usage::ipc_adapters::__ipc_get_session_usage_summary(__app, &mut __args).await,
        "get_usage_summary_by_app" => crate::commands::usage::ipc_adapters::__ipc_get_usage_summary_by_app(__app, &mut __args).await,
        "get_usage_trends" => crate::commands::usage::ipc_adapters::__ipc_get_usage_trends(__app, &mut __args).await,
        "get_usage_first_date" => crate::commands::usage::ipc_adapters::__ipc_get_usage_first_date(__app, &mut __args).await,
        "get_provider_stats" => crate::commands::usage::ipc_adapters::__ipc_get_provider_stats(__app, &mut __args).await,
        "get_model_stats" => crate::commands::usage::ipc_adapters::__ipc_get_model_stats(__app, &mut __args).await,
        "get_request_logs" => crate::commands::usage::ipc_adapters::__ipc_get_request_logs(__app, &mut __args).await,
        "get_request_detail" => crate::commands::usage::ipc_adapters::__ipc_get_request_detail(__app, &mut __args).await,
        "get_model_pricing" => crate::commands::usage::ipc_adapters::__ipc_get_model_pricing(__app, &mut __args).await,
        "update_model_pricing" => crate::commands::usage::ipc_adapters::__ipc_update_model_pricing(__app, &mut __args).await,
        "update_model_pricing_batch" => crate::commands::usage::ipc_adapters::__ipc_update_model_pricing_batch(__app, &mut __args).await,
        "delete_model_pricing" => crate::commands::usage::ipc_adapters::__ipc_delete_model_pricing(__app, &mut __args).await,
        "get_models_dev_sync_config" => crate::commands::usage::ipc_adapters::__ipc_get_models_dev_sync_config(__app, &mut __args).await,
        "save_models_dev_sync_config" => crate::commands::usage::ipc_adapters::__ipc_save_models_dev_sync_config(__app, &mut __args).await,
        "record_models_dev_sync_result" => crate::commands::usage::ipc_adapters::__ipc_record_models_dev_sync_result(__app, &mut __args).await,
        "fetch_models_dev" => crate::commands::usage::ipc_adapters::__ipc_fetch_models_dev(__app, &mut __args).await,
        "check_provider_limits" => crate::commands::usage::ipc_adapters::__ipc_check_provider_limits(__app, &mut __args).await,
        "sync_session_usage" => crate::commands::usage::ipc_adapters::__ipc_sync_session_usage(__app, &mut __args).await,
        "rebuild_codex_usage" => crate::commands::usage::ipc_adapters::__ipc_rebuild_codex_usage(__app, &mut __args).await,
        "get_session_usage_last_sync" => crate::commands::usage::ipc_adapters::__ipc_get_session_usage_last_sync(__app, &mut __args).await,
        "get_usage_data_sources" => crate::commands::usage::ipc_adapters::__ipc_get_usage_data_sources(__app, &mut __args).await,
        "fetch_exchange_rates" => crate::commands::usage::ipc_adapters::__ipc_fetch_exchange_rates(__app, &mut __args).await,
        "stream_check_provider" => crate::commands::stream_check::ipc_adapters::__ipc_stream_check_provider(__app, &mut __args).await,
        "list_sessions" => crate::commands::session_manager::ipc_adapters::__ipc_list_sessions(__app, &mut __args).await,
        "get_session_messages" => crate::commands::session_manager::ipc_adapters::__ipc_get_session_messages(__app, &mut __args).await,
        "stream_session_messages" => crate::commands::session_manager::ipc_adapters::__ipc_stream_session_messages(__app, &mut __args).await,
        "get_session_block_content" => crate::commands::session_manager::ipc_adapters::__ipc_get_session_block_content(__app, &mut __args).await,
        "get_session_image" => crate::commands::session_manager::ipc_adapters::__ipc_get_session_image(__app, &mut __args).await,
        "reveal_session_path" => crate::commands::session_manager::ipc_adapters::__ipc_reveal_session_path(__app, &mut __args).await,
        "export_session_markdown" => crate::commands::session_manager::ipc_adapters::__ipc_export_session_markdown(__app, &mut __args).await,
        "delete_session" => crate::commands::session_manager::ipc_adapters::__ipc_delete_session(__app, &mut __args).await,
        "delete_sessions" => crate::commands::session_manager::ipc_adapters::__ipc_delete_sessions(__app, &mut __args).await,
        "launch_session_terminal" => crate::commands::session_manager::ipc_adapters::__ipc_launch_session_terminal(__app, &mut __args).await,
        "get_tool_versions" => crate::commands::misc::ipc_adapters::__ipc_get_tool_versions(__app, &mut __args).await,
        "run_tool_lifecycle_action" => crate::commands::misc::ipc_adapters::__ipc_run_tool_lifecycle_action(__app, &mut __args).await,
        "prepare_tool_uninstall" => crate::commands::misc::ipc_adapters::__ipc_prepare_tool_uninstall(__app, &mut __args).await,
        "probe_tool_installations" => crate::commands::misc::ipc_adapters::__ipc_probe_tool_installations(__app, &mut __args).await,
        "list_tool_installations" => crate::commands::misc::ipc_adapters::__ipc_list_tool_installations(__app, &mut __args).await,
        "open_provider_terminal" => crate::commands::misc::ipc_adapters::__ipc_open_provider_terminal(__app, &mut __args).await,
        "get_universal_providers" => crate::commands::provider::ipc_adapters::__ipc_get_universal_providers(__app, &mut __args).await,
        "get_universal_provider" => crate::commands::provider::ipc_adapters::__ipc_get_universal_provider(__app, &mut __args).await,
        "upsert_universal_provider" => crate::commands::provider::ipc_adapters::__ipc_upsert_universal_provider(__app, &mut __args).await,
        "delete_universal_provider" => crate::commands::provider::ipc_adapters::__ipc_delete_universal_provider(__app, &mut __args).await,
        "sync_universal_provider" => crate::commands::provider::ipc_adapters::__ipc_sync_universal_provider(__app, &mut __args).await,
        "get_custom_icons" => crate::commands::custom_icon::ipc_adapters::__ipc_get_custom_icons(__app, &mut __args).await,
        "add_custom_icon" => crate::commands::custom_icon::ipc_adapters::__ipc_add_custom_icon(__app, &mut __args).await,
        "rename_custom_icon" => crate::commands::custom_icon::ipc_adapters::__ipc_rename_custom_icon(__app, &mut __args).await,
        "delete_custom_icon" => crate::commands::custom_icon::ipc_adapters::__ipc_delete_custom_icon(__app, &mut __args).await,
        "import_opencode_providers_from_live" => crate::commands::provider::ipc_adapters::__ipc_import_opencode_providers_from_live(__app, &mut __args).await,
        "get_opencode_live_provider_ids" => crate::commands::provider::ipc_adapters::__ipc_get_opencode_live_provider_ids(__app, &mut __args).await,
        "get_opencode_default_model" => crate::commands::provider::ipc_adapters::__ipc_get_opencode_default_model(__app, &mut __args).await,
        "set_opencode_default_model" => crate::commands::provider::ipc_adapters::__ipc_set_opencode_default_model(__app, &mut __args).await,
        "import_openclaw_providers_from_live" => crate::commands::openclaw::ipc_adapters::__ipc_import_openclaw_providers_from_live(__app, &mut __args).await,
        "get_openclaw_live_provider_ids" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_live_provider_ids(__app, &mut __args).await,
        "get_openclaw_live_provider" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_live_provider(__app, &mut __args).await,
        "scan_openclaw_config_health" => crate::commands::openclaw::ipc_adapters::__ipc_scan_openclaw_config_health(__app, &mut __args).await,
        "get_openclaw_default_model" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_default_model(__app, &mut __args).await,
        "set_openclaw_default_model" => crate::commands::openclaw::ipc_adapters::__ipc_set_openclaw_default_model(__app, &mut __args).await,
        "get_openclaw_model_catalog" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_model_catalog(__app, &mut __args).await,
        "set_openclaw_model_catalog" => crate::commands::openclaw::ipc_adapters::__ipc_set_openclaw_model_catalog(__app, &mut __args).await,
        "get_openclaw_agents_defaults" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_agents_defaults(__app, &mut __args).await,
        "set_openclaw_agents_defaults" => crate::commands::openclaw::ipc_adapters::__ipc_set_openclaw_agents_defaults(__app, &mut __args).await,
        "get_openclaw_env" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_env(__app, &mut __args).await,
        "set_openclaw_env" => crate::commands::openclaw::ipc_adapters::__ipc_set_openclaw_env(__app, &mut __args).await,
        "get_openclaw_tools" => crate::commands::openclaw::ipc_adapters::__ipc_get_openclaw_tools(__app, &mut __args).await,
        "set_openclaw_tools" => crate::commands::openclaw::ipc_adapters::__ipc_set_openclaw_tools(__app, &mut __args).await,
        "import_hermes_providers_from_live" => crate::commands::hermes::ipc_adapters::__ipc_import_hermes_providers_from_live(__app, &mut __args).await,
        "get_hermes_live_provider_ids" => crate::commands::hermes::ipc_adapters::__ipc_get_hermes_live_provider_ids(__app, &mut __args).await,
        "get_hermes_live_provider" => crate::commands::hermes::ipc_adapters::__ipc_get_hermes_live_provider(__app, &mut __args).await,
        "get_hermes_model_config" => crate::commands::hermes::ipc_adapters::__ipc_get_hermes_model_config(__app, &mut __args).await,
        "open_hermes_web_ui" => crate::commands::hermes::ipc_adapters::__ipc_open_hermes_web_ui(__app, &mut __args).await,
        "launch_hermes_dashboard" => crate::commands::hermes::ipc_adapters::__ipc_launch_hermes_dashboard(__app, &mut __args).await,
        "get_hermes_memory" => crate::commands::hermes::ipc_adapters::__ipc_get_hermes_memory(__app, &mut __args).await,
        "set_hermes_memory" => crate::commands::hermes::ipc_adapters::__ipc_set_hermes_memory(__app, &mut __args).await,
        "get_hermes_memory_limits" => crate::commands::hermes::ipc_adapters::__ipc_get_hermes_memory_limits(__app, &mut __args).await,
        "set_hermes_memory_enabled" => crate::commands::hermes::ipc_adapters::__ipc_set_hermes_memory_enabled(__app, &mut __args).await,
        "get_global_proxy_url" => crate::commands::global_proxy::ipc_adapters::__ipc_get_global_proxy_url(__app, &mut __args).await,
        "set_global_proxy_url" => crate::commands::global_proxy::ipc_adapters::__ipc_set_global_proxy_url(__app, &mut __args).await,
        "test_proxy_url" => crate::commands::global_proxy::ipc_adapters::__ipc_test_proxy_url(__app, &mut __args).await,
        "get_upstream_proxy_status" => crate::commands::global_proxy::ipc_adapters::__ipc_get_upstream_proxy_status(__app, &mut __args).await,
        "scan_local_proxies" => crate::commands::global_proxy::ipc_adapters::__ipc_scan_local_proxies(__app, &mut __args).await,
        "set_window_theme" => crate::commands::misc::ipc_adapters::__ipc_set_window_theme(__app, &mut __args).await,
        "auth_start_login" => crate::commands::auth::ipc_adapters::__ipc_auth_start_login(__app, &mut __args).await,
        "auth_poll_for_account" => crate::commands::auth::ipc_adapters::__ipc_auth_poll_for_account(__app, &mut __args).await,
        "auth_cancel_login" => crate::commands::auth::ipc_adapters::__ipc_auth_cancel_login(__app, &mut __args).await,
        "auth_list_accounts" => crate::commands::auth::ipc_adapters::__ipc_auth_list_accounts(__app, &mut __args).await,
        "auth_get_status" => crate::commands::auth::ipc_adapters::__ipc_auth_get_status(__app, &mut __args).await,
        "auth_remove_account" => crate::commands::auth::ipc_adapters::__ipc_auth_remove_account(__app, &mut __args).await,
        "auth_set_default_account" => crate::commands::auth::ipc_adapters::__ipc_auth_set_default_account(__app, &mut __args).await,
        "auth_logout" => crate::commands::auth::ipc_adapters::__ipc_auth_logout(__app, &mut __args).await,
        "agy_account_is_running" => crate::commands::agy_account::ipc_adapters::__ipc_agy_account_is_running(__app, &mut __args).await,
        "agy_account_quota" => crate::commands::agy_account::ipc_adapters::__ipc_agy_account_quota(__app, &mut __args).await,
        "claude_account_submit_paste" => crate::commands::claude_account::ipc_adapters::__ipc_claude_account_submit_paste(__app, &mut __args).await,
        "get_claude_account_quota" => crate::commands::claude_account::ipc_adapters::__ipc_get_claude_account_quota(__app, &mut __args).await,
        "get_claude_default_plan" => crate::commands::claude_account::ipc_adapters::__ipc_get_claude_default_plan(__app, &mut __args).await,
        "get_claude_login_switch" => crate::commands::claude_account::ipc_adapters::__ipc_get_claude_login_switch(__app, &mut __args).await,
        "kimi_oauth_status" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_status(__app, &mut __args).await,
        "dsh_account_status" => crate::commands::dsh::ipc_adapters::__ipc_dsh_account_status(__app, &mut __args).await,
        "dsh_account_sign_out" => crate::commands::dsh::ipc_adapters::__ipc_dsh_account_sign_out(__app, &mut __args).await,
        "dsh_account_login_start" => crate::commands::dsh::ipc_adapters::__ipc_dsh_account_login_start(__app, &mut __args).await,
        "dsh_account_login_poll" => crate::commands::dsh::ipc_adapters::__ipc_dsh_account_login_poll(__app, &mut __args).await,
        "dsh_account_login_cancel" => crate::commands::dsh::ipc_adapters::__ipc_dsh_account_login_cancel(__app, &mut __args).await,
        "kimi_oauth_start" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_start(__app, &mut __args).await,
        "kimi_oauth_poll" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_poll(__app, &mut __args).await,
        "kimi_oauth_cancel" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_cancel(__app, &mut __args).await,
        "kimi_oauth_remove" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_remove(__app, &mut __args).await,
        "kimi_oauth_quota" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_quota(__app, &mut __args).await,
        "kimi_oauth_use_account" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_use_account(__app, &mut __args).await,
        "kimi_oauth_models" => crate::commands::kimi_oauth::ipc_adapters::__ipc_kimi_oauth_models(__app, &mut __args).await,
        "copilot_start_device_flow" => crate::commands::copilot::ipc_adapters::__ipc_copilot_start_device_flow(__app, &mut __args).await,
        "copilot_poll_for_auth" => crate::commands::copilot::ipc_adapters::__ipc_copilot_poll_for_auth(__app, &mut __args).await,
        "copilot_poll_for_account" => crate::commands::copilot::ipc_adapters::__ipc_copilot_poll_for_account(__app, &mut __args).await,
        "copilot_list_accounts" => crate::commands::copilot::ipc_adapters::__ipc_copilot_list_accounts(__app, &mut __args).await,
        "copilot_remove_account" => crate::commands::copilot::ipc_adapters::__ipc_copilot_remove_account(__app, &mut __args).await,
        "copilot_set_default_account" => crate::commands::copilot::ipc_adapters::__ipc_copilot_set_default_account(__app, &mut __args).await,
        "copilot_get_auth_status" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_auth_status(__app, &mut __args).await,
        "copilot_logout" => crate::commands::copilot::ipc_adapters::__ipc_copilot_logout(__app, &mut __args).await,
        "copilot_is_authenticated" => crate::commands::copilot::ipc_adapters::__ipc_copilot_is_authenticated(__app, &mut __args).await,
        "copilot_get_token" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_token(__app, &mut __args).await,
        "copilot_get_token_for_account" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_token_for_account(__app, &mut __args).await,
        "copilot_get_models" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_models(__app, &mut __args).await,
        "copilot_get_models_for_account" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_models_for_account(__app, &mut __args).await,
        "copilot_get_usage" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_usage(__app, &mut __args).await,
        "copilot_get_usage_for_account" => crate::commands::copilot::ipc_adapters::__ipc_copilot_get_usage_for_account(__app, &mut __args).await,
        "read_omo_local_file" => crate::commands::omo::ipc_adapters::__ipc_read_omo_local_file(__app, &mut __args).await,
        "get_current_omo_provider_id" => crate::commands::omo::ipc_adapters::__ipc_get_current_omo_provider_id(__app, &mut __args).await,
        "disable_current_omo" => crate::commands::omo::ipc_adapters::__ipc_disable_current_omo(__app, &mut __args).await,
        "read_omo_slim_local_file" => crate::commands::omo::ipc_adapters::__ipc_read_omo_slim_local_file(__app, &mut __args).await,
        "get_current_omo_slim_provider_id" => crate::commands::omo::ipc_adapters::__ipc_get_current_omo_slim_provider_id(__app, &mut __args).await,
        "disable_current_omo_slim" => crate::commands::omo::ipc_adapters::__ipc_disable_current_omo_slim(__app, &mut __args).await,
        "read_workspace_file" => crate::commands::workspace::ipc_adapters::__ipc_read_workspace_file(__app, &mut __args).await,
        "write_workspace_file" => crate::commands::workspace::ipc_adapters::__ipc_write_workspace_file(__app, &mut __args).await,
        "list_daily_memory_files" => crate::commands::workspace::ipc_adapters::__ipc_list_daily_memory_files(__app, &mut __args).await,
        "read_daily_memory_file" => crate::commands::workspace::ipc_adapters::__ipc_read_daily_memory_file(__app, &mut __args).await,
        "write_daily_memory_file" => crate::commands::workspace::ipc_adapters::__ipc_write_daily_memory_file(__app, &mut __args).await,
        "delete_daily_memory_file" => crate::commands::workspace::ipc_adapters::__ipc_delete_daily_memory_file(__app, &mut __args).await,
        "search_daily_memory_files" => crate::commands::workspace::ipc_adapters::__ipc_search_daily_memory_files(__app, &mut __args).await,
        "open_workspace_directory" => crate::commands::workspace::ipc_adapters::__ipc_open_workspace_directory(__app, &mut __args).await,
        "enter_lightweight_mode" => crate::commands::lightweight::ipc_adapters::__ipc_enter_lightweight_mode(__app, &mut __args).await,
        "exit_lightweight_mode" => crate::commands::lightweight::ipc_adapters::__ipc_exit_lightweight_mode(__app, &mut __args).await,
        "is_lightweight_mode" => crate::commands::lightweight::ipc_adapters::__ipc_is_lightweight_mode(__app, &mut __args).await,
        other => Err(format!(
            "ipc: 未知命令 `{other}`（已桥接 {COMMAND_COUNT} 条）"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 命令清单不重复() {
        let mut names = COMMANDS.to_vec();
        names.sort_unstable();
        let original = names.len();
        names.dedup();
        assert_eq!(original, names.len(), "COMMANDS 里有重复命令");
        assert_eq!(COMMANDS.len(), COMMAND_COUNT);
    }

    /// 每条命令都必须有模块归属，否则控制服务的事件标注不出界面域。
    #[test]
    fn 每条命令都有模块归属() {
        for command in COMMANDS {
            assert!(
                module_of(command).is_some(),
                "命令 `{command}` 没有模块归属，请重跑生成脚本"
            );
        }
        assert_eq!(module_of("get_settings"), Some("settings"));
        assert_eq!(module_of("switch_provider"), Some("provider"));
        assert_eq!(module_of("并不是命令"), None);
    }

    /// 漂移守卫：generate_handler! 里增删命令后忘了重新生成时，这里会挂。
    #[test]
    fn 与_generate_handler_注册表一致() {
        let lib_rs = include_str!("../lib.rs");
        let start = lib_rs
            .find("generate_handler![")
            .expect("lib.rs 里应该有 generate_handler!");
        let end = lib_rs[start..]
            .find("]);")
            .map(|offset| offset + start)
            .expect("generate_handler! 应该有结尾");
        let registered: Vec<&str> = lib_rs[start..end]
            .lines()
            .skip(1)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| line.trim_end_matches(','))
            .map(|entry| entry.rsplit("::").next().unwrap_or(entry))
            .collect();
        assert_eq!(
            registered, COMMANDS,
            "命令清单和 generate_handler! 不一致：请重跑 scripts/gen_command_adapters.py"
        );
        assert_eq!(registered.len(), COMMAND_COUNT);
    }
}
