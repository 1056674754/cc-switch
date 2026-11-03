/**
 * 供应商总览的查询键。
 *
 * 供应商页和接入端页看的是同一份总览：一个页面里改地址 / 采用供应商之后，
 * 另一个页面下次进入时也读到新的，所以键放在这里共用。
 */
export const suppliersKeys = {
  overview: ["suppliersOverview"] as const,
};
