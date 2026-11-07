/**
 * 地址必须是完整的 http(s)：和后端 p1_normalize_url 的口径对齐（没有 scheme 的
 * 裸域名不算数），前端先挡一道，省一次往返。
 */
export function isCompleteHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return (
      (url.protocol === "http:" || url.protocol === "https:") &&
      url.hostname.trim() !== ""
    );
  } catch {
    return false;
  }
}
