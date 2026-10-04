import type { MaybeElementRef } from '@vueuse/core'
import { unrefElement, useScroll } from '@vueuse/core'

/**
 * 詳細の画面で、スクロールに合わせて見出しの帯とジャケットの見え方を決める（docs/web.md の「画面構成」）。
 * ジャケットは帯の下に入るにつれて薄れ、帯は大きな見出しが帯の下に入るにつれて塗りを濃くし、隠れきったら名前を出す。
 * ジャケットが見えている間に帯を塗ると、ジャケットの上に四角が被さって浮いて見えるため。
 */
export function useDetailHeader(heading: MaybeElementRef, cover: MaybeElementRef) {
  const scroller = useScroller()
  const { y } = useScroll(scroller)

  /** 要素が帯の下端を越えて隠れた割合（0〜1）。 */
  function hidden(target: MaybeElementRef): number {
    // スクロールのたびに測り直す
    void y.value
    const el = unrefElement(target)
    const bar = scroller.value?.querySelector<HTMLElement>('[data-testid="page-header"]')
    if (!el || !bar) {
      return 0
    }
    const { top, height } = el.getBoundingClientRect()
    const edge = bar.getBoundingClientRect().bottom
    return height > 0 ? Math.min(Math.max((edge - top) / height, 0), 1) : 0
  }

  const bandOpacity = computed(() => hidden(heading))
  const titleVisible = computed(() => bandOpacity.value >= 1)
  const coverOpacity = computed(() => 1 - hidden(cover))
  return { titleVisible, bandOpacity, coverOpacity }
}
