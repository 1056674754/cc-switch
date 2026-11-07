import { invoke } from "@tauri-apps/api/core";
import type { AppId } from "./types";

// ===== 连通性检查类型 =====
// 注意：本检查只探测 base_url 是否可达，不发真实大模型请求，也不触碰故障转移熔断器。

export type HealthStatus = "operational" | "degraded" | "failed";

export interface StreamCheckResult {
  status: HealthStatus;
  success: boolean;
  message: string;
  responseTimeMs?: number;
  httpStatus?: number;
  testedAt: number;
  retryCount: number;
}

/** CCR 式真实模型检查结果：每个模型一条，会消耗供应商额度。 */
export interface ModelCheckResult {
  model: string;
  success: boolean;
  message: string;
  responseTimeMs?: number;
  httpStatus?: number;
}

// ===== 连通性检查 API =====

/**
 * 连通性检查（单个供应商）
 */
export async function streamCheckProvider(
  appType: AppId,
  providerId: string,
): Promise<StreamCheckResult> {
  return invoke("stream_check_provider", { appType, providerId });
}

/**
 * 真实模型检查：对每个模型发一次最小请求（最多 1 token），
 * 会消耗供应商额度；调用前必须先让用户确认。
 */
export async function checkProviderModels(
  appType: AppId,
  providerId: string,
  models: string[],
): Promise<ModelCheckResult[]> {
  return invoke("check_provider_models", { appType, providerId, models });
}
