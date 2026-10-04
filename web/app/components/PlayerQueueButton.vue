<script setup lang="ts">
import { ListMusic } from 'lucide-vue-next'

/** キューを開け閉めするボタン（docs/web.md の「再生」）。開いているときはアクセント色にする。開くと歌詞を閉じる。 */
const open = usePlayerQueueOpen()
const lyrics = usePlayerLyricsOpen()
const { current } = usePlayer()

function toggle() {
  open.value = !open.value
  if (open.value) {
    lyrics.value = false
  }
}
</script>

<template>
  <Button
    variant="ghost"
    size="icon"
    aria-label="キュー"
    :aria-pressed="open"
    :disabled="!current"
    class="aria-pressed:text-accent-base"
    @click="toggle"
  >
    <ListMusic class="size-4" />
  </Button>
</template>
