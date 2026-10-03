<script setup lang="ts">
import { ListMusic } from 'lucide-vue-next'

/** キューを開け閉めするボタン（docs/web.md の「再生」）。開いているときはアクセント色にする。 */
const open = usePlayerQueueOpen()
const { current } = usePlayer()
</script>

<template>
  <Button
    variant="ghost"
    size="icon"
    aria-label="キュー"
    :aria-pressed="open"
    :disabled="!current"
    class="aria-pressed:text-accent-base"
    @click="open = !open"
  >
    <ListMusic class="size-4" />
  </Button>
</template>
