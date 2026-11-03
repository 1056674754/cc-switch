import { invoke } from "@tauri-apps/api/core";

/** 网关端点：某家供应商在某一种协议上的上游地址。 */
export interface SupplierEndpoint {
  /** 端点主键 */
  id: string;
  /** 协议：anthropic / openai_chat / openai_responses / gemini_native …（未知协议原样透出） */
  protocol: string;
  /** 归一化后的地址；托管 OAuth 类供应商没有地址时是空串 */
  baseUrl: string;
  /** 使用该端点的 app，如 ["claude", "claude-desktop"] */
  apps: string[];
  /** `app:providerId` 明细，方便跳回各应用的原有配置 */
  consumers: string[];
  /** 挂在这个端点上的凭据数 */
  credentialCount: number;
}

/** 供应商（网关级上游）：一家服务商有几个端点、被哪些 app 使用。 */
export interface Supplier {
  id: string;
  /** 供应商显示名 */
  name: string;
  /** 域名；托管 OAuth 类供应商为空串 */
  host: string;
  endpoints: SupplierEndpoint[];
  /** 被几个 app 使用 */
  appCount: number;
  /** 这家供应商下出现过的凭据形态：literal / env_ref / managed / none 的子集 */
  credentialKinds: string[];
}

/** 一键采用的结果：agent 侧新建的那条配置（地址和密钥都来自网关）。 */
export interface AdoptedSupplierProvider {
  /** 采用进的 app，如 "claude-desktop" */
  appType: string;
  /** agent 侧新配置的 providerId */
  providerId: string;
  /** agent 侧新配置的名字，沿用供应商名 */
  providerName: string;
}

export const suppliersApi = {
  // 供应商总览：网关级视角的只读聚合
  async overview(): Promise<Supplier[]> {
    return invoke("get_suppliers_overview");
  },

  // 改一个端点的地址：所有使用它的 app 会一起被改写，返回更新后的总览
  async updateBaseUrl(
    endpointId: string,
    baseUrl: string,
  ): Promise<Supplier[]> {
    return invoke("update_supplier_base_url", { endpointId, baseUrl });
  },

  // 一键采用：把端点接进某个 agent，地址和密钥都从网关读，用户不用填 key。
  // 目前后端只支持 claude / claude-desktop；协议不匹配、密钥是环境变量引用或托管登录时
  // 会 reject 一段中文原因，调用方原样 toast 出来。
  async adopt(
    appType: string,
    endpointId: string,
  ): Promise<AdoptedSupplierProvider> {
    return invoke("adopt_supplier", { appType, endpointId });
  },
};
