<script setup lang="ts">
import { MicVocal } from 'lucide-vue-next'

/** 歌詞を開け閉めするボタン（docs/web.md の「再生」）。歌詞のない曲では押せない。開くとキューを閉じる。 */
const open = usePlayerLyricsOpen()
const queue = usePlayerQueueOpen()
const lyrics = useLyrics()

function toggle() {
  open.value = !open.value
  if (open.value) {
    queue.value = false
  }
}

// 歌詞のない曲に移ったら閉じる
watch(lyrics, (value) => {
  if (value === null) {
    open.value = false
  }
})
</script>

<template>
  <Button
    variant="ghost"
    size="icon"
    aria-label="歌詞"
    :aria-pressed="open"
    :disabled="!lyrics"
    class="aria-pressed:text-accent-base"
    @click="toggle"
  >
    <MicVocal class="size-4" />
  </Button>
</template>
