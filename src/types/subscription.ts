export type CredentialStatus =
  | "valid"
  | "expired"
  // 访问令牌过期、刷新令牌还在：客户端下次运行时自己会换新的
  | "refresh_pending"
  | "not_found"
  | "parse_error";

export interface QuotaTier {
  name: string;
  utilization: number; // 0-100
  resetsAt: string | null;
  usedValueUsd?: number | null;
  maxValueUsd?: number | null;
  planLabel?: string | null;
}

export interface ExtraUsage {
  isEnabled: boolean;
  monthlyLimit: number | null;
  usedCredits: number | null;
  utilization: number | null;
  currency: string | null;
}

/** 一张存下的限额重置卡（用来清空已用完的额度窗口，用掉就没了） */
export interface ResetCredit {
  id: string;
  /** 到期时间；null 表示不过期 */
  expiresAt: string | null;
}

/** ChatGPT 订阅存下的限额重置：每一次的到期时间，先到期的在前，null 表示不过期 */
export interface ResetCredits {
  expiresAt: (string | null)[];
  /** 每张卡的 id，顺序同 expiresAt；上游没给 id 时可能缺省 */
  credits?: ResetCredit[];
}

export interface SubscriptionQuota {
  tool: string;
  credentialStatus: CredentialStatus;
  credentialMessage: string | null;
  success: boolean;
  tiers: QuotaTier[];
  extraUsage: ExtraUsage | null;
  /** 只有 ChatGPT 订阅有；没查到时缺省 */
  resetCredits?: ResetCredits | null;
  /** ChatGPT 订阅买的 Codex Credits 余额（额度用完后才扣）；没有时缺省 */
  creditsBalance?: number | null;
  /** 套餐名（「Pro 200」「Google AI Pro」「SuperGrok」…），授权中心的账号额度会带；没有时缺省 */
  planName?: string | null;
  error: string | null;
  queriedAt: number | null;
}

/** Claude 账号的套餐：Claude Code 记在凭据里（`subscriptionType` / `rateLimitTier`） */
export interface ClaudeAccountPlan {
  subscriptionType: string | null;
  rateLimitTier: string | null;
}

/** 绑号卡的额度，带上这个号的套餐（读不到时缺省） */
export interface ClaudeAccountQuota extends SubscriptionQuota {
  plan?: ClaudeAccountPlan | null;
}
