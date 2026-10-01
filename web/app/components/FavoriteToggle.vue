<script setup lang="ts">
import { Heart } from 'lucide-vue-next'

/** お気に入りだけに絞り込む切り替え（docs/web.md の「一覧」）。 */
const model = defineModel<boolean>({ required: true })
</script>

<template>
  <button
    type="button"
    :aria-pressed="model"
    class="inline-flex h-8 items-center gap-1.5 rounded-full border border-border-subtle bg-surface-1 px-3 text-body-sm font-medium text-fg-subtle transition-all hover:text-fg-muted aria-pressed:border-border-strong aria-pressed:bg-accent-soft aria-pressed:text-fg aria-pressed:shadow-[0_0_8px_var(--glow-low)]"
    @click="model = !model"
  >
    <Heart class="size-3.5" :fill="model ? 'currentColor' : 'none'" />
    お気に入り
  </button>
</template>
