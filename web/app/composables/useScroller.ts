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

/** スクロール位置を `top` に戻す。グリッドの幅を測るまでは高さが足りず、位置が頭打ちになるので、届くまで数フレーム試す。 */
export function restoreScroll(scroller: Ref<HTMLElement | undefined>, top: number) {
  let frames = 0
  const restore = () => {
    const el = scroller.value
    if (!el) {
      return
    }
    el.scrollTop = top
    if (Math.abs(el.scrollTop - top) > 1 && ++frames < 30) {
      requestAnimationFrame(restore)
    }
  }
  requestAnimationFrame(restore)
}
