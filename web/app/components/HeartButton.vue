<script setup lang="ts">
import type { StarTarget } from '~/composables/useStar'
import { Heart } from 'lucide-vue-next'

/** お気に入りのハート（docs/web.md の「一覧」）。押すとすぐに切り替え、失敗したら元に戻す。 */
const props = defineProps<{ target: StarTarget, id: string, starred: boolean, label: string, size?: 'sm' | 'lg' }>()

const { starred, toggle } = useStar(props.target, () => props.id, () => props.starred)
</script>

<template>
  <button
    type="button"
    :aria-label="label"
    :aria-pressed="starred"
    class="flex shrink-0 items-center justify-center rounded-full text-fg-subtle transition-colors hover:text-fg aria-pressed:text-accent-base"
    :class="size === 'lg' ? 'size-10 border border-border-default bg-surface-2 hover:border-border-strong' : 'size-8'"
    @click.stop.prevent="toggle"
  >
    <Heart :class="size === 'lg' ? 'size-5' : 'size-4'" :fill="starred ? 'currentColor' : 'none'" />
  </button>
</template>
