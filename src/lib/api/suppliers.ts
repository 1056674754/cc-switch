import { invoke } from "@tauri-apps/api/core";

/**
 * 一条接入明细：哪个 app 的哪条配置挂在这个端点上，以及它自己的模型映射。
 *
 * 模型映射归「接入」而不归网关：同一个端点，claude 里可以映射成 kimi-k2、
 * claude-desktop 里映射成别的，所以每条接入各带一份。
 */
export interface SupplierEndpointBinding {
  /** app 类型，如 "claude" / "claude-desktop" */
  appType: string;
  /** 该 app 里挂在这个端点上的 providerId */
  providerId: string;
  /** 槽位化映射（haiku/sonnet/opus/fable/subagent/default）；没配过时是空对象 */
  modelMap: Record<string, string>;
}

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
  /** 接入明细（含模型映射），模型映射编辑器用它预填 */
  bindings: SupplierEndpointBinding[];
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
  // 在网关侧一次配齐：建一家供应商，并直接接进选中的 app。
  // apps 目前只支持 claude / claude-desktop；名称 / 密钥为空、地址不是完整的 http(s)、
  // 一个 app 都没选时后端 reject 一段中文原因（调用方原样 toast）。
  // 同一家（同名 + 同地址 + 同 app）重复提交是幂等的，不会堆出重复的行。
  async create(
    name: string,
    baseUrl: string,
    apiKey: string,
    apps: string[],
  ): Promise<Supplier[]> {
    return invoke("create_supplier", { name, baseUrl, apiKey, apps });
  },

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

  // 换一把密钥：所有使用这个端点的 app 一起换，返回更新后的总览。
  // 密钥只写不读——后端从不返回明文，只返回 credentialCount，界面也拿不到旧值。
  // 密钥存在环境变量里（库里没有明文）或端点没有任何接入时会 reject 一段中文原因。
  async updateSecret(endpointId: string, secret: string): Promise<Supplier[]> {
    return invoke("update_supplier_secret", { endpointId, secret });
  },

  // 改一个接入的模型映射（app + provider 粒度，不动同一端点上别的 app）。
  // modelMap 的槽位是固定的 haiku/sonnet/opus/fable/subagent/default，
  // 值为空串（或整个槽位缺省）表示清掉该槽位对应的模型字段。
  async updateBindingModelMap(
    appType: string,
    providerId: string,
    modelMap: Record<string, string>,
  ): Promise<Supplier[]> {
    return invoke("update_binding_model_map", {
      appType,
      providerId,
      modelMap,
    });
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
