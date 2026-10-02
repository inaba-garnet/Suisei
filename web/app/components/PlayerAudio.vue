<script setup lang="ts">
/** 再生に使う `<audio>`。layout に一つだけ置き、画面を移っても作り直さない（docs/web.md の「再生」）。 */
const { attach, detach, handlers } = usePlayerAudio()
const el = ref<HTMLAudioElement>()

onMounted(() => attach(el.value!))
onBeforeUnmount(detach)
</script>

<template>
  <audio
    ref="el"
    preload="auto"
    data-testid="audio"
    @play="handlers.play"
    @pause="handlers.pause"
    @timeupdate="handlers.timeupdate"
    @ended="handlers.ended"
    @error="handlers.error"
  />
</template>
