<script setup lang="ts">
import { formatDuration } from '~/utils/subsonic'

/** 再生位置のバー（docs/web.md の「再生」）。つまんで動かしている間は時間の表示だけを変え、離したら移る。`times` なら下に時間を出す。 */
defineProps<{ times?: boolean }>()

const { state, current, seek } = usePlayer()
const preview = ref<number>()
const duration = computed(() => current.value?.duration ?? 0)
const shown = computed(() => preview.value ?? state.value.position)

function commit(seconds: number) {
  preview.value = undefined
  seek(seconds)
}
</script>

<template>
  <div class="flex flex-col gap-1">
    <PlayerSlider
      :value="state.position"
      :max="duration"
      :step="5"
      label="再生位置"
      :value-text="`${formatDuration(shown)} / ${formatDuration(duration)}`"
      @preview="preview = $event"
      @commit="commit"
    />
    <div v-if="times" class="flex justify-between text-caption text-fg-subtle tabular-nums">
      <span>{{ formatDuration(shown) }}</span>
      <span>{{ formatDuration(duration) }}</span>
    </div>
  </div>
</template>
