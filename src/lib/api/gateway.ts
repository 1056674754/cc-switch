import { invoke } from "@tauri-apps/api/core";
import type { AppMigration, AppModelConfig, GatewayCatalogEntry } from "@/types/gateway";

/** 网关模型目录与每 app 模型配置（P1）。 */
export const gatewayApi = {
  /** 整份目录：每条 = 模型引用 + 展示信息 + 端点 + 来源。 */
  async getCatalog(): Promise<GatewayCatalogEntry[]> {
    return invoke("get_gateway_catalog");
  },

  /** 读某 app 的模型配置；没配过返回 null。 */
  async getAppModelConfig(appType: string): Promise<AppModelConfig | null> {
    return invoke("get_app_model_config", { appType });
  },

  /** 写某 app 的模型配置（后端会先按目录校验引用）。 */
  async setAppModelConfig(appType: string, config: AppModelConfig): Promise<void> {
    return invoke("set_app_model_config", { appType, config });
  },

  /** 把旧的 Stack（聚合）配置一次性翻译成每 app 模型配置，幂等。 */
  async migrateFromStack(): Promise<AppMigration[]> {
    return invoke("migrate_gateway_from_stack");
  },

  /** 接管：把该 app 的客户端指到网关（替代「进入路由模式」）。 */
  async adopt(appType: string): Promise<void> {
    return invoke("adopt_app_to_gateway", { appType });
  },

  /** 释放：把客户端文件还原成接管前的样子（替代「退出路由模式」）。 */
  async release(appType: string): Promise<void> {
    return invoke("release_app_from_gateway", { appType });
  },
};
