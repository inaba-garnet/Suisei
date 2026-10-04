<script setup lang="ts">
import type { SongMenuTarget } from '~/composables/useSongMenu'
import { Ellipsis } from 'lucide-vue-next'

/** 曲とアルバムの「…」のボタン（docs/web.md の「メニュー」）。押すと layout のメニューを開く。 */
const props = withDefaults(defineProps<{ target: SongMenuTarget, label: string, size?: 'default' | 'lg' }>(), { size: 'default' })
const { open } = useSongMenu()
</script>

<template>
  <Button
    v-if="size === 'lg'"
    variant="secondary"
    size="icon-lg"
    :aria-label="label"
    aria-haspopup="menu"
    @click="open(props.target, $event)"
  >
    <Ellipsis class="size-5" />
  </Button>
  <button
    v-else
    type="button"
    :aria-label="label"
    aria-haspopup="menu"
    class="-mr-2 flex size-8 shrink-0 items-center justify-center rounded-full text-fg-subtle transition-colors hover:bg-surface-2 hover:text-fg"
    @click="open(props.target, $event)"
  >
    <Ellipsis class="size-4" />
  </button>
</template>
