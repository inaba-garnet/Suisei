<script setup lang="ts">
import { onKeyStroke, useMediaQuery } from '@vueuse/core'
import { X } from 'lucide-vue-next'

/**
 * 下端の再生バーを出す PC（md 以上 2xl 未満）で、キューか歌詞を右端に重ねて出すパネル（docs/web.md の「再生」）。
 * 中身は再生プレイヤーのキューや歌詞と同じ。× のボタン、キューや歌詞のボタン、Esc で閉じる。
 */
const queue = usePlayerQueueOpen()
const lyrics = usePlayerLyricsOpen()
const { current } = usePlayer()
const shown = useMediaQuery('(min-width: 48rem) and (max-width: 89.98rem)')
// キューと歌詞は同時に開かないが、両方立っていればキューを出す
const mode = computed(() => queue.value ? 'queue' : lyrics.value ? 'lyrics' : undefined)
const title = computed(() => mode.value === 'lyrics' ? '歌詞' : 'キュー')

function close() {
  queue.value = false
  lyrics.value = false
}

onKeyStroke('Escape', () => {
  if (shown.value) {
    close()
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
      v-if="shown && mode && current"
      class="fixed top-3 right-5 bottom-[84px] z-40 flex w-[360px] flex-col rounded-xl border border-divider bg-surface-1 p-5 shadow-[0_8px_32px_rgb(0_0_0/0.25)]"
      role="dialog"
      :aria-label="title"
      data-testid="player-queue-drawer"
    >
      <div class="-my-1.5 mb-3.5 flex items-center justify-between">
        <h2 class="text-body font-semibold">
          {{ title }}
        </h2>
        <Button variant="ghost" size="icon" :aria-label="`${title}を閉じる`" @click="close">
          <X class="size-4" />
        </Button>
      </div>
      <PlayerQueue v-if="mode === 'queue'" class="-mx-1 min-h-0 flex-1 px-1" />
      <PlayerLyrics v-else class="min-h-0 flex-1" />
    </div>
  </Transition>
</template>
