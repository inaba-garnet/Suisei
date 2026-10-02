<script setup lang="ts">
/** 再生中の曲の進み具合。シークは後から入れるので、今は表示だけ（docs/web.md の「再生」）。高さと角丸は置く側で決める。 */
const { state, current } = usePlayer()

const ratio = computed(() => {
  const duration = current.value?.duration ?? 0
  return duration > 0 ? Math.min(1, state.value.position / duration) : 0
})
</script>

<template>
  <div class="overflow-hidden bg-surface-3" role="progressbar" aria-label="再生位置" :aria-valuenow="Math.round(state.position)" :aria-valuemax="current?.duration ?? 0">
    <div class="h-full rounded-[inherit] bg-accent-base" :style="{ width: `${ratio * 100}%` }" />
  </div>
</template>
