<script setup lang="ts">
import { Volume1, Volume2, VolumeX } from 'lucide-vue-next'

/** 音量（docs/web.md の「再生」）。PC だけに置く。左のボタンで消音と元の音量を切り替える。 */
const volume = usePlayerVolume()
/** 消音する前の音量 */
const before = ref(1)

function toggleMute() {
  if (volume.value > 0) {
    before.value = volume.value
    volume.value = 0
  }
  else {
    volume.value = before.value || 1
  }
}
</script>

<template>
  <div class="flex items-center gap-1.5">
    <Button variant="ghost" size="icon" :aria-label="volume > 0 ? '消音' : '消音を解除'" @click="toggleMute">
      <VolumeX v-if="volume <= 0" class="size-4" />
      <Volume1 v-else-if="volume < 0.5" class="size-4" />
      <Volume2 v-else class="size-4" />
    </Button>
    <PlayerSlider
      :value="volume"
      :max="1"
      :step="0.1"
      label="音量"
      :value-text="`${Math.round(volume * 100)}%`"
      class="w-24"
      @preview="volume = $event"
      @commit="volume = $event"
    />
  </div>
</template>
