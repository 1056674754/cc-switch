/**
 * 端点协议的显示名。
 *
 * 协议标识是技术名词，不做翻译；后端也可能拼出 `opencode:<npm>` 这类值，原样显示。
 * 供应商页和接入端页共用一份，免得两边把同一个协议显示成两种写法。
 */
const PROTOCOL_LABELS: Record<string, string> = {
  anthropic: "Anthropic",
  openai_chat: "OpenAI Chat",
  openai_responses: "OpenAI Responses",
  gemini_native: "Gemini",
  kimi_managed: "Kimi",
  alibaba: "Alibaba",
};

export function protocolLabel(protocol: string): string {
  return PROTOCOL_LABELS[protocol] ?? protocol;
}
