<script setup lang="ts">
import { Disc3 } from 'lucide-vue-next'
import { coverArtUrl } from '~/utils/subsonic'

/** アルバムのジャケット。`size` は表示のおおよその大きさで、サーバーにはその 2 倍を頼む（docs/web.md の「一覧」）。 */
const props = defineProps<{ id?: string, size: number, alt?: string }>()

const failed = ref(false)
watch(() => props.id, () => {
  failed.value = false
})
</script>

<template>
  <div class="aspect-square overflow-hidden rounded-lg border border-border-subtle bg-surface-3">
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
