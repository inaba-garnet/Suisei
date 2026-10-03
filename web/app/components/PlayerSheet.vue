<script setup lang="ts">
import { onKeyStroke } from '@vueuse/core'

/**
 * スマホで下から引き出す再生画面（docs/web.md の「再生」）。中身は右の欄と同じ再生プレイヤー。
 * パネルのどこでも下に引くか、後ろの画面を押すと閉じる。上の取っ手は引けることを示す目印。
 */
const open = usePlayerSheet()
const queue = usePlayerQueueOpen()
const { current } = usePlayer()

/** 取っ手をこれより下まで引いて離したら閉じる（px）。 */
const CLOSE_DISTANCE = 120

function close() {
  open.value = false
}

// 画面を移ったら、移った先が見えるように閉じる
const route = useRoute()
watch(() => route.fullPath, close)
onKeyStroke('Escape', close)
// 閉じたら、次に開いたときはジャケットの表示から始める
watch(open, (value) => {
  if (!value) {
    queue.value = false
  }
})
watch(current, (song) => {
  if (!song) {
    close()
  }
})

/** 下にこれだけ動いたら、タップではなくパネルを引いているとみなす（px）。 */
const START_DISTANCE = 8

// パネルのどこでも、下に引いている間は指に付いて動かし、離したら閉じるか元に戻す。
// 中身（キュー）がスクロールしていれば先にスクロールし、`data-sheet-no-drag` の上（シークのバー、並べ替えの取っ手など）で始めた操作は引かない
const sheet = ref<HTMLElement>()
const drag = ref<{ start: number, offset: number, active: boolean }>()

function scrolled(target: EventTarget | null) {
  for (let el = target as HTMLElement | null; el && el !== sheet.value; el = el.parentElement) {
    if (el.scrollTop > 0) {
      return true
    }
  }
  return false
}

function onTouchStart(e: TouchEvent) {
  const touch = e.touches[0]
  if (e.touches.length !== 1 || !touch || (e.target as Element).closest('[data-sheet-no-drag]') || scrolled(e.target)) {
    drag.value = undefined
    return
  }
  drag.value = { start: touch.clientY, offset: 0, active: false }
}

function onTouchMove(e: TouchEvent) {
  const d = drag.value
  const touch = e.touches[0]
  if (!d || !touch) {
    return
  }
  const dy = touch.clientY - d.start
  if (!d.active) {
    // 上に動かしたなら中身のスクロールに任せる
    if (dy < -START_DISTANCE) {
      drag.value = undefined
      return
    }
    if (dy <= START_DISTANCE) {
      return
    }
    d.active = true
  }
  e.preventDefault()
  d.offset = Math.max(0, dy)
}

function onTouchEnd() {
  if (drag.value?.active && drag.value.offset > CLOSE_DISTANCE) {
    close()
  }
  drag.value = undefined
}
</script>

<template>
  <div class="md:hidden">
    <Transition
      enter-active-class="transition-opacity duration-300"
      enter-from-class="opacity-0"
      leave-active-class="transition-opacity duration-200"
      leave-to-class="opacity-0"
    >
      <div v-if="open && current" class="fixed inset-0 z-50 bg-black/50" data-testid="player-sheet-backdrop" @click="close" />
    </Transition>
    <Transition
      enter-active-class="transition-transform duration-300 ease-out"
      enter-from-class="translate-y-full"
      leave-active-class="transition-transform duration-200 ease-in"
      leave-to-class="translate-y-full"
    >
      <div
        v-if="open && current"
        ref="sheet"
        class="fixed inset-x-0 top-[calc(env(safe-area-inset-top)+16px)] bottom-0 z-50 overflow-hidden rounded-t-2xl bg-surface-0 shadow-[0_-8px_32px_rgb(0_0_0/0.35)]"
        :class="{ 'transition-transform duration-200': !drag?.active }"
        :style="drag?.active ? { transform: `translateY(${drag.offset}px)` } : undefined"
        role="dialog"
        aria-modal="true"
        aria-label="再生画面"
        @touchstart="onTouchStart"
        @touchmove="onTouchMove"
        @touchend="onTouchEnd"
        @touchcancel="onTouchEnd"
      >
        <div class="pointer-events-none absolute inset-x-0 top-0 z-10 flex h-7 items-center justify-center" data-testid="player-sheet-handle">
          <span class="h-1.5 w-10 rounded-full bg-fg-subtle/50" />
        </div>
        <PlayerPanel sheet @close="close" />
      </div>
    </Transition>
  </div>
</template>
