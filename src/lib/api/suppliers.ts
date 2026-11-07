import { invoke } from "@tauri-apps/api/core";

/**
 * 一条接入明细：哪个 app 的哪条配置挂在这家供应商上，以及它自己的模型映射。
 *
 * 模型映射归「接入」而不归供应商：同一个上游，claude 里可以映射成 kimi-k2、
 * claude-desktop 里映射成别的，所以每条接入各带一份。
 */
export interface SupplierBindingDetail {
  /** app 类型，如 "claude" / "claude-desktop" */
  appType: string;
  /** 该 app 里挂在这家供应商上的 providerId */
  providerId: string;
  /** 槽位化映射（haiku/sonnet/opus/fable/subagent/default）；没配过时是空对象 */
  modelMap: Record<string, string>;
  /** 客户端 → 代理 这一段的协议（由 app 决定） */
  downstreamProtocol: string;
}

/** 一把凭据被哪条接入使用。 */
export interface SupplierCredentialApp {
  appType: string;
  providerId: string;
  providerName: string;
  /** 是否在该 app 的故障转移队列里 */
  inFailoverQueue: boolean;
  /** 这条接入当前是否就是该 app 正在使用的 provider（托管账号池标「使用中」） */
  isCurrent?: boolean;
}

/** 供应商的一把凭据：按密钥值去重，明文永不返回。 */
export interface SupplierCredential {
  keyHash: string;
  /** 掩码后的密钥，如 `sk-…abcd` */
  maskedSecret: string;
  secretKind: "literal" | "env_ref" | "managed" | string;
  envName: string | null;
  /** 托管账号 id（managed 凭据才有），前端用它关联授权中心里的登录名/头像 */
  accountId?: string | null;
  apps: SupplierCredentialApp[];
}

/**
 * 一家供应商 = 一个确切的上游 API（服务根 + 上游协议）+ 用它的一组接入。
 *
 * 同一家供应商在不同 app 的配置里可能写成不同的完整 URL（`.../v1`、
 * `.../v1/chat/completions`），后端会把它们归并到同一个服务根，所以界面上
 * 一家供应商只有一个地址。
 */
export interface Supplier {
  /** 稳定 id，如 "sup-1a2b3c…" */
  id: string;
  /** 供应商显示名（可能重名，靠 baseUrl / 协议区分） */
  name: string;
  /** 域名；托管登录账号为空串 */
  host: string;
  /** 服务根，如 https://ai.ctaigw.cn/v1；托管登录账号为空串 */
  baseUrl: string;
  /** 代理 → 上游 的协议：openai_chat / anthropic / openai_responses / gemini_native … */
  upstreamProtocol: string;
  /** 网关是否把请求路由到这家上游；停用只是关开关，各 app 里的配置一条不动。 */
  enabled: boolean;
  /** 使用它的 app，如 ["claude", "claude-desktop"] */
  apps: string[];
  /** `app:providerId` 明细，方便跳回各应用的原有配置 */
  consumers: string[];
  /** 接入明细（含模型映射与下游协议），模型映射编辑器用它预填 */
  bindings: SupplierBindingDetail[];
  /** 挂在这个地址上的凭据数 */
  credentialCount: number;
  /** 这个地址上出现过的凭据形态：literal / env_ref / managed / none 的子集 */
  credentialKinds: string[];
  /**
   * 明文密钥的去重指纹：同一个 Key 被多个 app 各存一份时只出现一次。
   * 界面用它显示「几把密钥」，而不是把每个 app 的接入记录都数一遍。
   */
  credentialKeyHashes?: string[];
  /** 凭据明细：每把不同的明文 Key / 环境变量一行 */
  credentials?: SupplierCredential[];
  /** 托管登录的认证供应商（如 `codex_oauth`）；非托管为空 */
  managedAuthProvider?: string | null;
  /** 绑定到这家供应商的 provider 各自选了账号池里的哪个账号 */
  boundAccountIds?: string[];
  /** 归到这家供应商的上游 id，模型目录与人工覆盖按它对上（老 fixture 可能没有） */
  upstreamIds?: string[];
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

/** 删除时被跳过的一条配置：哪个 app 的哪条，以及后端给的中文原因。 */
export interface DeleteSupplierSkipped {
  appType: string;
  providerId: string;
  reason: string;
}

/** 删除一家供应商的结果：删后的总览 + 实际删掉的配置条数 + 没删掉的明细。 */
export interface DeleteSupplierOutcome {
  suppliers: Supplier[];
  deleted: number;
  skipped: DeleteSupplierSkipped[];
}

export const suppliersApi = {
  // 建一家供应商（上游 = 服务根 + 上游协议 + 密钥），是否接进 app 是可选的：
  // apps 为空表示只建供应商、暂不接入任何应用。名称 / 密钥为空、地址不是完整的
  // http(s) 时后端 reject 一段中文原因（调用方原样 toast）。
  // 同一家（同地址 + 同协议）重复提交是幂等的，不会堆出重复的行。
  async create(
    name: string,
    baseUrl: string,
    apiKey: string,
    apps: string[],
    protocol = "anthropic",
  ): Promise<Supplier[]> {
    return invoke("create_supplier", { name, baseUrl, apiKey, apps, protocol });
  },

  // 导入本机登录态：把已登录的官方订阅（codex_oauth / claude_official / gemini_official）
  // 直接建成一家托管供应商，不接任何 app、不写明文密钥。幂等。
  async ensureManaged(authProvider: string): Promise<Supplier[]> {
    return invoke("ensure_managed_supplier", { authProvider });
  },

  // 供应商总览：网关级视角的只读聚合
  async overview(): Promise<Supplier[]> {
    return invoke("get_suppliers_overview");
  },

  // 改一家供应商的地址：所有使用它的 app 会一起被改写，返回更新后的总览
  async updateBaseUrl(
    supplierId: string,
    baseUrl: string,
  ): Promise<Supplier[]> {
    return invoke("update_supplier_base_url", { supplierId, baseUrl });
  },

  // 改一家供应商的名字：同一家供应商的所有底层端点共用一个上游名，返回更新后的总览
  async updateName(supplierId: string, name: string): Promise<Supplier[]> {
    return invoke("update_supplier_name", { supplierId, name });
  },

  // 换一把密钥：所有使用这家供应商的 app 一起换，返回更新后的总览。
  // 密钥只写不读——后端从不返回明文，只返回 credentialCount，界面也拿不到旧值。
  // 密钥存在环境变量里（库里没有明文）或这家供应商没有任何接入时会 reject 一段中文原因。
  async updateSecret(supplierId: string, secret: string): Promise<Supplier[]> {
    return invoke("update_supplier_secret", { supplierId, secret });
  },

  /** 只替换池子里的这一把 Key，其它 Key 原样保留。 */
  async updateCredential(
    supplierId: string,
    keyHash: string,
    secret: string,
  ): Promise<Supplier[]> {
    return invoke("update_supplier_credential", {
      supplierId,
      keyHash,
      secret,
    });
  },

  /** 给指定 app 再添加一把 Key（会新增一条接入并加入故障转移队列）。 */
  async addCredential(
    supplierId: string,
    appType: string,
    secret: string,
  ): Promise<Supplier[]> {
    return invoke("add_supplier_credential", {
      supplierId,
      appType,
      secret,
    });
  },

  /** 删除池子里的一把 Key；某 app 只剩这一把时后端拒绝。 */
  async deleteCredential(
    supplierId: string,
    keyHash: string,
  ): Promise<Supplier[]> {
    return invoke("delete_supplier_credential", { supplierId, keyHash });
  },

  // 托管账号池：给这家供应商添加一个已登录的账号。后端会为它接入的每个 app
  // 复制一条绑定该账号的 provider，并加入故障转移队列；name 用于生成可读名称。
  async addAccount(
    supplierIds: string[],
    accountId: string,
    name?: string,
  ): Promise<Supplier[]> {
    return invoke("add_supplier_account", {
      supplierIds,
      accountId,
      name: name ?? null,
    });
  },

  // 把账号池里的某个账号设为当前使用（只改 current，不合并/删除其它账号）。
  async setCurrentAccount(
    supplierIds: string[],
    accountId: string,
  ): Promise<Supplier[]> {
    return invoke("set_supplier_current_account", { supplierIds, accountId });
  },

  // 删除托管账号池里的一行：使用该账号的接入一起删；某个 app 只剩它时后端拒绝。
  async deleteAccount(
    supplierIds: string[],
    accountId: string,
  ): Promise<Supplier[]> {
    return invoke("delete_supplier_account", { supplierIds, accountId });
  },

  // 启停托管账号 = 该账号的全部 provider 是否进入故障转移队列。
  async setAccountEnabled(
    supplierIds: string[],
    accountId: string,
    enabled: boolean,
  ): Promise<Supplier[]> {
    return invoke("set_supplier_account_enabled", {
      supplierIds,
      accountId,
      enabled,
    });
  },

  /** 池内 Key 启停 = 是否进入该 app 的故障转移队列。 */
  async setCredentialEnabled(
    supplierId: string,
    keyHash: string,
    enabled: boolean,
  ): Promise<Supplier[]> {
    return invoke("set_supplier_credential_enabled", {
      supplierId,
      keyHash,
      enabled,
    });
  },

  // 启用 / 停用一家供应商：只拨开关，地址和密钥都不动，返回更新后的总览。
  // 停用后网关不再把请求路由到这家上游，但各 app 里的配置仍原样留着。
  async setEnabled(supplierId: string, enabled: boolean): Promise<Supplier[]> {
    return invoke("set_supplier_enabled", { supplierId, enabled });
  },

  // 让托管登录的这家供应商改用账号池里的某个账号：改掉它每条接入的
  // authBinding.accountId，并按当前模式重写 live / 代理契约，返回更新后的总览。
  // supplierIds 是同一张卡片上的全部 supplier 行（托管登录按服务合成一张卡）。
  async setManagedAccount(
    supplierIds: string[],
    accountId: string,
  ): Promise<Supplier[]> {
    return invoke("set_supplier_managed_account", { supplierIds, accountId });
  },

  // 改一个接入的模型映射（app + provider 粒度，不动这家供应商上别的 app）。
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

  // 一键采用：把供应商接进某个 agent，地址和密钥都从网关读，用户不用填 key。
  // 目前后端只支持 claude / claude-desktop；协议不匹配、密钥是环境变量引用或托管登录时
  // 会 reject 一段中文原因，调用方原样 toast 出来。
  async adopt(
    appType: string,
    supplierId: string,
  ): Promise<AdoptedSupplierProvider> {
    return invoke("adopt_supplier", { appType, supplierId });
  },

  // 删除一家供应商：连同它在各 app 里的配置一起删，返回删后的总览与统计。
  // 个别配置删不掉（已过期、写盘失败…）不会让整个删除失败，而是进 skipped 由界面提示。
  async remove(supplierId: string): Promise<DeleteSupplierOutcome> {
    return invoke("delete_supplier", { supplierId });
  },
};
