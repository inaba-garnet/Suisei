<script setup lang="ts">
/**
 * 再生に使う `<audio>`。layout に一つだけ置き、画面を移っても作り直さない（docs/web.md の「再生」）。
 * 二つ置き、鳴らしていない方で次の曲を先に読む。曲が替わるときに役目を入れ替える。
 */
const { attach, detach, handlers } = usePlayerAudio()
// 残した音量はスマホでも当てる
usePlayerVolume()
const first = ref<HTMLAudioElement>()
const second = ref<HTMLAudioElement>()
const active = useActiveAudio()
usePlayerScrobble(active)
usePlayerMediaSession(active)
usePlayerTrace([first, second])

onMounted(() => attach(first.value!, second.value!))
onBeforeUnmount(detach)
</script>

<template>
  <audio
    ref="first"
    preload="auto"
    :data-testid="active === first ? 'audio' : 'audio-standby'"
    @play="handlers.play"
    @pause="handlers.pause"
    @timeupdate="handlers.timeupdate"
    @ended="handlers.ended"
    @playing="handlers.playing"
    @error="handlers.error"
  />
  <audio
    ref="second"
    preload="auto"
    :data-testid="active === second ? 'audio' : 'audio-standby'"
    @play="handlers.play"
    @pause="handlers.pause"
    @timeupdate="handlers.timeupdate"
    @ended="handlers.ended"
    @playing="handlers.playing"
    @error="handlers.error"
  />
</template>
