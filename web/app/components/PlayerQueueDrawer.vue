<script setup lang="ts">
import { onKeyStroke, useMediaQuery } from '@vueuse/core'
import { X } from 'lucide-vue-next'

/**
 * 下端の再生バーを出す PC（md 以上 2xl 未満）で、キューを右端に重ねて出すパネル（docs/web.md の「再生」）。
 * 中身はスマホのキューと同じ。× のボタン、キューのボタン、Esc で閉じる。
 */
const open = usePlayerQueueOpen()
const { current } = usePlayer()
const shown = useMediaQuery('(min-width: 48rem) and (max-width: 89.98rem)')

onKeyStroke('Escape', () => {
  if (shown.value) {
    open.value = false
  }
})
</script>

<template>
  <Transition
    enter-active-class="transition duration-200 ease-out"
    enter-from-class="translate-x-4 opacity-0"
    leave-active-class="transition duration-150 ease-in"
    leave-to-class="translate-x-4 opacity-0"
  >
    <div
      v-if="shown && open && current"
      class="fixed top-3 right-5 bottom-[84px] z-40 flex w-[360px] flex-col rounded-xl border border-divider bg-surface-1 p-5 shadow-[0_8px_32px_rgb(0_0_0/0.25)]"
      role="dialog"
      aria-label="キュー"
      data-testid="player-queue-drawer"
    >
      <div class="-my-1.5 mb-3.5 flex items-center justify-between">
        <h2 class="text-body font-semibold">
          キュー
        </h2>
        <Button variant="ghost" size="icon" aria-label="キューを閉じる" @click="open = false">
          <X class="size-4" />
        </Button>
      </div>
      <PlayerQueue class="-mx-1 min-h-0 flex-1 px-1" />
    </div>
  </Transition>
</template>
