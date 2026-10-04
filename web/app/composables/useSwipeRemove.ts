/** 左にこれより大きく払って離したら消す（px）。 */
const REMOVE_DISTANCE = 96
/** 横にこれだけ動いたら、タップやスクロールではなく払っているとみなす（px）。 */
const SWIPE_START = 10

/**
 * スマホで行を左に払って消す操作（docs/web.md の「再生」）。`enabled` が偽のときと、`[data-swipe-ignore]` の上で始めた操作では払わない。
 * 縦に動かしたならスクロールや、再生画面を引く操作に任せる。
 */
export function useSwipeRemove(enabled: () => boolean, onRemove: () => void) {
  const swiping = ref<{ x: number, y: number, dx: number, active: boolean }>()

  function onTouchStart(e: TouchEvent) {
    const touch = e.touches[0]
    if (!enabled() || e.touches.length !== 1 || !touch || (e.target as Element).closest('[data-swipe-ignore]')) {
      swiping.value = undefined
      return
    }
    swiping.value = { x: touch.clientX, y: touch.clientY, dx: 0, active: false }
  }

  function onTouchMove(e: TouchEvent) {
    const s = swiping.value
    const touch = e.touches[0]
    if (!s || !touch) {
      return
    }
    const dx = touch.clientX - s.x
    const dy = touch.clientY - s.y
    if (!s.active) {
      if (Math.abs(dy) > SWIPE_START && Math.abs(dy) >= Math.abs(dx)) {
        swiping.value = undefined
        return
      }
      if (Math.abs(dx) <= SWIPE_START) {
        return
      }
      s.active = true
    }
    // 払っている間は、再生画面を引く操作に渡さない
    e.preventDefault()
    e.stopPropagation()
    s.dx = Math.min(0, dx)
  }

  function onTouchEnd() {
    const s = swiping.value
    swiping.value = undefined
    if (s?.active && s.dx < -REMOVE_DISTANCE) {
      onRemove()
    }
  }

  const handlers = { touchstart: onTouchStart, touchmove: onTouchMove, touchend: onTouchEnd, touchcancel: onTouchEnd }
  return { swiping, handlers }
}
