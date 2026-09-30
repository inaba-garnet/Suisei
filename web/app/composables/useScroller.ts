import type { InjectionKey, Ref } from 'vue'

/** 内容をスクロールするレイアウトのパネル。window はスクロールしない（docs/web.md の「画面構成」）。 */
export const scrollerKey: InjectionKey<Ref<HTMLElement | undefined>> = Symbol('scroller')

export function useScroller(): Ref<HTMLElement | undefined> {
  return inject(scrollerKey, ref())
}

/** 直前の画面の移動が、ブラウザの「戻る」「進む」だったか。 */
export function usePopped() {
  return useState('navigation-popped', () => false)
}
