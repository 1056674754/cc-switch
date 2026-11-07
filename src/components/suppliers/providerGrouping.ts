import type { ManagedAuthProvider } from "@/lib/api/auth";
import type {
  Supplier,
  SupplierBindingDetail,
  SupplierCredential,
} from "@/lib/api/suppliers";
import { isManagedAuthProvider } from "./SupplierManagedAccounts";

/**
 * 供应商卡片：一家供应商可能由多条底层 supplier 行（即各 app 的接入行）组成。
 *
 * - key 型：同一个名称 + 同一个服务根的多个上游协议合成一张卡（例如阿里云百炼的
 *   Alibaba / OpenAI Chat 两个协议），地址、密钥改一次要逐条写回这些 supplier 行。
 * - 托管型：同一个账号服务（ChatGPT / Claude）的多个官方接入合成一张卡，
 *   账号就是它的凭据；Codex 和 OpenAI Official 只在「被使用」里各出现一次。
 */
export interface ProviderGroup {
  id: string;
  name: string;
  managedAuthProvider: ManagedAuthProvider | null;
  /** 组内所有底层 supplier 行 */
  suppliers: Supplier[];
  /** 展示地址、协议、开关状态的那一条 */
  primary: Supplier;
  apps: string[];
  bindings: SupplierBindingDetail[];
  credentialKinds: string[];
  credentialCount: number;
  /** 去重后的明文密钥指纹：同一个 Key 在多个 app 里各存一份只算一把 */
  keyHashes: string[];
  distinctKeyCount: number;
  /** 按密钥值去重的凭据列表（含使用它的接入） */
  credentials: SupplierCredential[];
  upstreamIds: string[];
  /** Kimi：一张卡里既有托管登录账号，也有给其它 app 的明文密钥 */
  kimiManaged: boolean;
}

/** 服务根归一化：去掉末尾的 /v1，让 /coding 和 /coding/v1 落到同一张卡里。 */
export function rootKey(baseUrl: string): string {
  let url = baseUrl.trim().toLowerCase().replace(/\/+$/, "");
  if (url.endsWith("/v1")) url = url.slice(0, -3);
  return url;
}

/** 一个协议只留一条 supplier 行；合成卡时取第一条非托管的行做展示。 */
function pickPrimary(suppliers: Supplier[]): Supplier {
  return (
    suppliers.find(
      (supplier) =>
        supplier.baseUrl.trim() !== "" &&
        !supplier.credentialKinds.includes("managed"),
    ) ??
    suppliers.find((supplier) => supplier.baseUrl.trim() !== "") ??
    suppliers[0]
  );
}

export function groupSuppliers(suppliers: Supplier[]): ProviderGroup[] {
  const groups: ProviderGroup[] = [];
  const index = new Map<string, ProviderGroup>();

  for (const supplier of suppliers) {
    const managedProvider = isManagedAuthProvider(supplier.managedAuthProvider)
      ? supplier.managedAuthProvider
      : null;
    const key = managedProvider
      ? `managed:${managedProvider}`
      : `key:${supplier.name}|${rootKey(supplier.baseUrl)}`;

    let group = index.get(key);
    if (!group) {
      group = {
        id: key,
        name: supplier.name,
        managedAuthProvider: managedProvider,
        suppliers: [],
        primary: supplier,
        apps: [],
        bindings: [],
        credentialKinds: [],
        credentialCount: 0,
        keyHashes: [],
        distinctKeyCount: 0,
        credentials: [],
        upstreamIds: [],
        kimiManaged: false,
      };
      index.set(key, group);
      groups.push(group);
    }

    group.suppliers.push(supplier);
    group.credentialCount += supplier.credentialCount;
    for (const kind of supplier.credentialKinds) {
      if (!group.credentialKinds.includes(kind)) {
        group.credentialKinds.push(kind);
      }
    }
    for (const hash of supplier.credentialKeyHashes ?? []) {
      if (!group.keyHashes.includes(hash)) group.keyHashes.push(hash);
    }
    for (const credential of supplier.credentials ?? []) {
      const existing = group.credentials.find(
        (current) => current.keyHash === credential.keyHash,
      );
      if (!existing) {
        group.credentials.push({ ...credential, apps: [...credential.apps] });
        continue;
      }
      for (const app of credential.apps) {
        if (
          !existing.apps.some(
            (current) =>
              current.appType === app.appType &&
              current.providerId === app.providerId,
          )
        ) {
          existing.apps.push(app);
        }
      }
    }
    for (const app of supplier.apps) {
      if (!group.apps.includes(app)) group.apps.push(app);
    }
    for (const binding of supplier.bindings) {
      if (
        !group.bindings.some(
          (current) =>
            current.appType === binding.appType &&
            current.providerId === binding.providerId,
        )
      ) {
        group.bindings.push(binding);
      }
    }
    for (const upstreamId of supplier.upstreamIds ?? []) {
      if (!group.upstreamIds.includes(upstreamId)) {
        group.upstreamIds.push(upstreamId);
      }
    }
    if (
      supplier.credentialKinds.includes("managed") &&
      supplier.consumers.some((consumer) => consumer.startsWith("kimi:"))
    ) {
      group.kimiManaged = true;
    }
  }

  for (const group of groups) {
    group.primary = pickPrimary(group.suppliers);
    if (group.managedAuthProvider) {
      group.name = group.primary.name;
    }
    group.distinctKeyCount = group.keyHashes.length;
    for (const supplier of group.suppliers) {
      for (const binding of supplier.bindings) {
        if (!group.apps.includes(binding.appType)) {
          group.apps.push(binding.appType);
        }
      }
    }
  }

  return groups;
}

export function isEnabled(supplier: Supplier): boolean {
  return supplier.enabled !== false;
}

export function hasBaseUrl(supplier: Supplier): boolean {
  return supplier.baseUrl.trim() !== "";
}

/** 只有托管登录、没有密钥的行没有可改写的网关地址。 */
export function isManagedOnly(supplier: Supplier): boolean {
  return (
    supplier.credentialKinds.length > 0 &&
    supplier.credentialKinds.every(
      (kind) => kind === "managed" || kind === "none",
    )
  );
}

/** 改地址只写有网关地址、且不是纯托管登录的行；托管端点不能被改写。 */
export function baseUrlTargets(suppliers: Supplier[]): Supplier[] {
  return suppliers.filter(
    (supplier) => hasBaseUrl(supplier) && !isManagedOnly(supplier),
  );
}

/** 换密钥只写有明文密钥的行；环境变量行没有明文可换。 */
export function secretTargets(suppliers: Supplier[]): Supplier[] {
  return suppliers.filter((supplier) =>
    supplier.credentialKinds.includes("literal"),
  );
}

/** 启停只拨有网关地址的行；托管登录没有网关可关。 */
export function toggleTargets(suppliers: Supplier[]): Supplier[] {
  return suppliers.filter(hasBaseUrl);
}

/** 一张卡里能「采用到」的直连上游：有地址、已启用、带明文/环境变量密钥。 */
export function adoptTarget(suppliers: Supplier[]): Supplier | null {
  return (
    suppliers.find(
      (candidate) =>
        hasBaseUrl(candidate) &&
        isEnabled(candidate) &&
        candidate.credentialKinds.some(
          (kind) => kind === "literal" || kind === "env_ref",
        ),
    ) ?? null
  );
}

export function collectAppTypes(group: ProviderGroup): string[] {
  return group.apps;
}
