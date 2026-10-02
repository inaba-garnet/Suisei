<script setup lang="ts">
import type { RepeatMode } from '~/composables/usePlayer'
import { Repeat, Repeat1, Shuffle } from 'lucide-vue-next'

/** シャッフルかリピートのボタン（docs/web.md の「再生」）。オンのときはアクセント色にする。 */
defineProps<{ mode: 'shuffle' | 'repeat' }>()

const { state, current, toggleShuffle, cycleRepeat } = usePlayer()

const repeatLabel: Record<RepeatMode, string> = { off: 'なし', all: 'キュー全体', one: '1 曲' }
</script>

<template>
  <Button
    v-if="mode === 'shuffle'"
    variant="ghost"
    size="icon"
    aria-label="シャッフル"
    :aria-pressed="state.shuffle"
    :disabled="!current"
    class="aria-pressed:text-accent-base"
    @click="toggleShuffle"
  >
    <Shuffle class="size-4" />
  </Button>
  <Button
    v-else
    variant="ghost"
    size="icon"
    :aria-label="`リピート（${repeatLabel[state.repeat]}）`"
    :aria-pressed="state.repeat !== 'off'"
    :disabled="!current"
    class="aria-pressed:text-accent-base"
    @click="cycleRepeat"
  >
    <Repeat1 v-if="state.repeat === 'one'" class="size-4" />
    <Repeat v-else class="size-4" />
  </Button>
</template>
