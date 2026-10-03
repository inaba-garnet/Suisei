<script setup lang="ts">
import type { DeepReadonly } from 'vue'
import type { Song } from '~/utils/subsonic'
import { GripVertical, X } from 'lucide-vue-next'

/**
 * キューの一行（docs/web.md の「再生」）。押すとその曲に移る。
 * 消すのは、`swipe` なら行を左に払う操作、そうでなければマウスを乗せると出る × のボタン。再生中の曲は消さない。
 */
const props = defineProps<{
  song: DeepReadonly<Song>
  index: number
  /** 再生履歴の曲なら薄く出す */
  played?: boolean
  /** 取っ手を出して並べ替えられるようにする */
  sortable?: boolean
  swipe?: boolean
}>()

const { state, jump, remove } = usePlayer()
const playing = computed(() => props.index === state.value.index)

/** 左にこれより大きく払って離したら消す（px）。 */
const REMOVE_DISTANCE = 96
/** 横にこれだけ動いたら、タップやスクロールではなく払っているとみなす（px）。 */
const SWIPE_START = 10

const swiping = ref<{ x: number, y: number, dx: number, active: boolean }>()

function onTouchStart(e: TouchEvent) {
  const touch = e.touches[0]
  if (!props.swipe || playing.value || e.touches.length !== 1 || !touch || (e.target as Element).closest('[data-queue-handle]')) {
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
    // 縦に動かしたならスクロールか、パネルを引く操作に任せる
    if (Math.abs(dy) > SWIPE_START && Math.abs(dy) >= Math.abs(dx)) {
      swiping.value = undefined
      return
    }
    if (Math.abs(dx) <= SWIPE_START) {
      return
    }
    s.active = true
  }
  // 払っている間は、パネルを引く操作に渡さない
  e.preventDefault()
  e.stopPropagation()
  s.dx = Math.min(0, dx)
}

function onTouchEnd() {
  const s = swiping.value
  swiping.value = undefined
  if (s?.active && s.dx < -REMOVE_DISTANCE) {
    remove(props.index)
  }
}
</script>

<template>
  <li class="relative overflow-hidden rounded-md">
    <!-- スマホで左に払うと、後ろの「削除」が見える -->
    <div
      v-if="swiping?.active"
      aria-hidden="true"
      class="absolute inset-0 flex items-center justify-end bg-danger px-4 text-body-sm font-medium text-white"
    >
      削除
    </div>
    <div
      class="group relative flex items-center gap-2"
      :class="swiping?.active ? 'bg-surface-1' : 'transition-transform duration-200'"
      :style="swiping?.active ? { transform: `translateX(${swiping.dx}px)` } : undefined"
      @touchstart="onTouchStart"
      @touchmove="onTouchMove"
      @touchend="onTouchEnd"
      @touchcancel="onTouchEnd"
    >
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-3 py-1 text-left"
        :class="{ 'opacity-60': played }"
        :aria-current="playing ? 'true' : undefined"
        @click="jump(index)"
      >
        <!--
          再生プレイヤーの中なので、ジャケットに枠線を付けない。小さいので角丸も控えめにする。
          再生中の曲は背景を敷かず、ジャケットと曲名を大きくしてアルバム名も添え、一覧の見出しのように見せる
        -->
        <CoverArt
          :id="song.coverArt"
          :size="playing ? 64 : 40"
          :alt="song.album"
          frameless
          class="shrink-0 rounded-md"
          :class="playing ? 'size-16' : 'size-10'"
        />
        <span class="min-w-0 flex-1">
          <span class="block truncate" :class="playing ? 'text-h3 font-semibold text-accent-base' : 'text-body'" data-testid="queue-title">{{ song.title }}</span>
          <span v-if="playing && state.failed" class="block truncate text-body-sm text-danger">再生できませんでした</span>
          <span v-else class="block truncate text-body-sm text-fg-subtle">{{ playing ? [song.artist, song.album].filter(Boolean).join(' · ') : song.artist }}</span>
        </span>
      </button>
      <button
        v-if="!swipe && !playing"
        type="button"
        :aria-label="`${song.title}をキューから消す`"
        class="flex size-8 shrink-0 items-center justify-center rounded-full text-fg-subtle opacity-0 transition-opacity group-hover:opacity-100 hover:bg-surface-2 hover:text-fg focus-visible:opacity-100"
        @click="remove(index)"
      >
        <X class="size-4" />
      </button>
      <span
        v-if="sortable"
        :aria-label="`${song.title}を並べ替える`"
        role="img"
        class="flex size-8 shrink-0 cursor-grab touch-none items-center justify-center text-fg-subtle active:cursor-grabbing"
        data-queue-handle
        data-sheet-no-drag
      >
        <GripVertical class="size-4" />
      </span>
    </div>
  </li>
</template>
