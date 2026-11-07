import { invoke } from "@tauri-apps/api/core";
import type {
  AppMigration,
  AppModelConfig,
  GatewayCatalogEntry,
  ModelOverride,
  SupplierModelFetch,
} from "@/types/gateway";

/** 网关模型目录与每 app 模型配置（P1）。 */
export const gatewayApi = {
  /**
   * 整份目录：每条 = 模型引用 + 展示信息 + 端点 + 来源。
   *
   * `refresh` 让后端先丢掉进程内的目录缓存再派生 —— 只给界面上的「刷新」用；
   * 普通读取继续吃缓存。
   *
   * `includeHidden` 为真时把被隐藏覆盖挡掉的条目也带回来：供应商编辑弹窗的「左栏」
   * 要能重新「加入」这些模型，否则隐藏之后在界面上就再也找不到了。共享缓存必须
   * 保持可见目录，所以这个参数只给单独的查询 key 用。
   */
  async getCatalog(options?: {
    refresh?: boolean;
    includeHidden?: boolean;
  }): Promise<GatewayCatalogEntry[]> {
    return invoke("get_gateway_catalog", {
      refresh: options?.refresh ?? false,
      includeHidden: options?.includeHidden ?? false,
    });
  },

  /**
   * 从该上游的 `/v1/models` 真拉一次模型列表并写库。
   *
   * 新出现的模型由后端写成 hidden 覆盖（= 未启用）：拉取只负责把上游的真实全集
   * 带进目录，要不要对用户开放仍然要人工确认。失败时后端 reject 出中文错误串。
   */
  /**
   * 刷新一家供应商的模型：传这张卡上的所有 upstream 行，后端按真实上游归并后
   * **每家只发一次请求**，再把结果写进它对应的每一行。
   */
  async fetchSupplierModels(
    upstreamIds: string[],
  ): Promise<SupplierModelFetch> {
    return invoke("fetch_supplier_models", { upstreamIds });
  },

  /** 读某 app 的模型配置；没配过返回 null。 */
  async getAppModelConfig(appType: string): Promise<AppModelConfig | null> {
    return invoke("get_app_model_config", { appType });
  },

  /** 写某 app 的模型配置（后端会先按目录校验引用）。 */
  async setAppModelConfig(
    appType: string,
    config: AppModelConfig,
  ): Promise<void> {
    return invoke("set_app_model_config", { appType, config });
  },

  /** 把旧的 Stack（聚合）配置一次性翻译成每 app 模型配置，幂等。 */
  async migrateFromStack(): Promise<AppMigration[]> {
    return invoke("migrate_gateway_from_stack");
  },

  /** 供应商卡片用：所有人工覆盖（改名 / 隐藏）。 */
  async getModelOverrides(): Promise<ModelOverride[]> {
    return invoke("get_model_overrides");
  },

  /** 写一条人工覆盖；`displayName` 传空表示用派生值，`hidden` 表示从目录里隐藏。 */
  async setModelOverride(input: {
    upstreamId: string;
    modelId: string;
    displayName?: string | null;
    hidden: boolean;
  }): Promise<void> {
    return invoke("set_model_override", {
      upstreamId: input.upstreamId,
      modelId: input.modelId,
      displayName: input.displayName ?? null,
      hidden: input.hidden,
    });
  },

  /** 删掉一条人工覆盖，回到 provider 行派生的值。 */
  async clearModelOverride(input: {
    upstreamId: string;
    modelId: string;
  }): Promise<void> {
    return invoke("clear_model_override", {
      upstreamId: input.upstreamId,
      modelId: input.modelId,
    });
  },

  /** P4-A：OpenCode 现在是否已经指向网关。 */
  async getOpencodeGatewayStatus(): Promise<boolean> {
    return invoke("get_opencode_gateway_status");
  },

  /** P4-A：Hermes 现在是否已经指向网关。 */
  async getHermesGatewayStatus(): Promise<boolean> {
    return invoke("get_hermes_gateway_status");
  },

  /** P4-A：把 Hermes 指到网关（写一条 cc-switch custom provider + 默认模型）。 */
  async adoptHermesToGateway(): Promise<void> {
    return invoke("adopt_hermes_to_gateway");
  },

  /** P4-A：释放 Hermes（删掉 cc-switch custom provider）。 */
  async releaseHermesFromGateway(): Promise<void> {
    return invoke("release_hermes_from_gateway");
  },

  /** P4-A：OpenClaw 现在是否已经指向网关。 */
  async getOpenclawGatewayStatus(): Promise<boolean> {
    return invoke("get_openclaw_gateway_status");
  },

  /** P4-A：把 OpenClaw 指到网关（写一条 provider + 默认模型）。 */
  async adoptOpenclawToGateway(): Promise<void> {
    return invoke("adopt_openclaw_to_gateway");
  },

  /** P4-A：释放 OpenClaw（删掉 cc-switch provider）。 */
  async releaseOpenclawFromGateway(): Promise<void> {
    return invoke("release_openclaw_from_gateway");
  },

  /** P4-A：Pi 现在是否已经指向网关。 */
  async getPiGatewayStatus(): Promise<boolean> {
    return invoke("get_pi_gateway_status");
  },

  /** P4-A：把 Pi 指到网关（写 models.json provider + settings.json 默认模型）。 */
  async adoptPiToGateway(): Promise<void> {
    return invoke("adopt_pi_to_gateway");
  },

  /** P4-A：释放 Pi（删 provider、清默认值）。 */
  async releasePiFromGateway(): Promise<void> {
    return invoke("release_pi_from_gateway");
  },

  /** P4-A：OMP 现在是否已经有 cc-switch provider。 */
  async getOmpGatewayStatus(): Promise<boolean> {
    return invoke("get_omp_gateway_status");
  },

  /** P4-A：把 OMP 的模型来源指向网关（写 models.yml 的 provider）。 */
  async adoptOmpToGateway(): Promise<void> {
    return invoke("adopt_omp_to_gateway");
  },

  /** P4-A：释放 OMP（删 provider）。 */
  async releaseOmpFromGateway(): Promise<void> {
    return invoke("release_omp_from_gateway");
  },

  /** P4-A：DSH 现在是否已经有 cc-switch provider。 */
  async getDshGatewayStatus(): Promise<boolean> {
    return invoke("get_dsh_gateway_status");
  },

  /** P4-A：把 DSH 的模型来源指向网关（写所有 profile 的 provider）。 */
  async adoptDshToGateway(): Promise<void> {
    return invoke("adopt_dsh_to_gateway");
  },

  /** P4-A：释放 DSH（删 provider、清凭据）。 */
  async releaseDshFromGateway(): Promise<void> {
    return invoke("release_dsh_from_gateway");
  },

  /** P4-A：把 OpenCode 指到网关（写一条 cc-switch provider + 默认模型）。 */
  async adoptOpencodeToGateway(): Promise<void> {
    return invoke("adopt_opencode_to_gateway");
  },

  /** P4-A：释放 OpenCode（删掉 cc-switch provider）。 */
  async releaseOpencodeFromGateway(): Promise<void> {
    return invoke("release_opencode_from_gateway");
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
