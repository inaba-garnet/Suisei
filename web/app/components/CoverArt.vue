<script setup lang="ts">
import { Disc3 } from 'lucide-vue-next'
import { coverArtUrl } from '~/utils/subsonic'

/**
 * アルバムのジャケット。`size` は表示のおおよその大きさで、サーバーにはその 2 倍を頼む（docs/web.md の「一覧」）。
 * `frameless` なら枠線を付けない。
 */
const props = defineProps<{ id?: string, size: number, alt?: string, frameless?: boolean }>()

const failed = ref(false)
watch(() => props.id, () => {
  failed.value = false
})
</script>

<template>
  <div class="aspect-square overflow-hidden rounded-lg bg-surface-3" :class="{ 'border border-border-subtle': !frameless }">
    <img
      v-if="id && !failed"
      :src="coverArtUrl(id, size * 2)"
      :alt="alt ?? ''"
      loading="lazy"
      decoding="async"
      class="size-full object-cover"
      @error="failed = true"
    >
    <div v-else class="flex size-full items-center justify-center text-fg-subtle">
      <Disc3 class="size-1/3" />
    </div>
  </div>
</template>
