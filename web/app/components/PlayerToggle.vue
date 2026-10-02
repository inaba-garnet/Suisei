<script setup lang="ts">
import { Pause, Play } from 'lucide-vue-next'

/** 再生と一時停止を切り替える丸いボタン。何も再生していなければ押せない。再生中は強く、止めているときは弱く光らせる。 */
defineProps<{ size?: 'lg' }>()

const { state, current, toggle } = usePlayer()
</script>

<template>
  <button
    type="button"
    class="flex items-center justify-center rounded-full border border-accent-base bg-accent-subtle text-fg transition-shadow duration-500 disabled:pointer-events-none disabled:opacity-50"
    :class="[size === 'lg' ? 'size-12' : 'size-10', state.playing ? 'shadow-[0_0_20px_var(--glow-high)]' : 'shadow-[0_0_6px_var(--glow-low)] hover:shadow-[0_0_12px_var(--glow-medium)]']"
    :disabled="!current"
    :aria-label="state.playing ? '一時停止' : '再生'"
    @click="toggle"
  >
    <Pause v-if="state.playing" :class="size === 'lg' ? 'size-5' : 'size-4'" />
    <Play v-else :class="size === 'lg' ? 'size-5' : 'size-4'" />
  </button>
</template>
