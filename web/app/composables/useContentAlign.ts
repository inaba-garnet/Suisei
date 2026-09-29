import type { ContentAlign } from '~/utils/layout'
import { CONTENT_ALIGN_STORAGE_KEY, parseContentAlign } from '~/utils/layout'

/** PC で内容を置く位置の選択（docs/web.md）。 */
export function useContentAlign() {
  const align = useState<ContentAlign>('content-align', () => parseContentAlign(readStorage()))

  function setAlign(value: ContentAlign) {
    align.value = value
    try {
      if (value === 'center') {
        localStorage.removeItem(CONTENT_ALIGN_STORAGE_KEY)
      }
      else {
        localStorage.setItem(CONTENT_ALIGN_STORAGE_KEY, value)
      }
    }
    catch {
      // プライベートブラウズなどで保存できなくても、この画面の間は切り替える
    }
  }

  return { align, setAlign }
}

function readStorage(): string | null {
  if (!import.meta.client) {
    return null
  }
  try {
    return localStorage.getItem(CONTENT_ALIGN_STORAGE_KEY)
  }
  catch {
    return null
  }
}
