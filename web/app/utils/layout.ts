/** PC で内容を置く位置。実機で見比べるための一時的な設定（docs/web.md）。 */
export type ContentAlign = 'center' | 'left'

export const CONTENT_ALIGN_STORAGE_KEY = 'suisei-dev-content-align'

/** 保存された値を読む。知らない値は中央にする。 */
export function parseContentAlign(value: string | null): ContentAlign {
  return value === 'left' ? 'left' : 'center'
}
