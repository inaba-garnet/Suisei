<script setup lang="ts">
import type { Song } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { formatDuration } from '~/utils/subsonic'

/** 曲の行。画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「一覧」）。 */
const props = defineProps<{ songs: Song[], more: boolean }>()
const emit = defineEmits<{ more: [] }>()

const ROW = 56

const scroller = useScroller()
const root = ref<HTMLElement>()

const virtualizer = useVirtualizer(computed(() => ({
  count: props.songs.length,
  getScrollElement: () => scroller.value ?? null,
  estimateSize: () => ROW,
  overscan: 8,
  scrollMargin: root.value?.offsetTop ?? 0,
})))

const items = computed(() => virtualizer.value.getVirtualItems())

watch([items, () => props.more], ([visible, more]) => {
  const last = visible.at(-1)
  if (more && (!last || last.index >= props.songs.length - 10)) {
    emit('more')
  }
})
</script>

<template>
  <ol ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="songs">
    <li
      v-for="item in items"
      :key="songs[item.index]!.id"
      class="absolute inset-x-0 top-0 flex items-center gap-3 border-b border-divider px-3"
      :style="{ height: `${ROW}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
    >
      <CoverArt :id="songs[item.index]!.coverArt" :size="40" :alt="songs[item.index]!.album" class="size-10 shrink-0" />
      <span class="min-w-0 flex-1">
        <span class="block truncate text-body">{{ songs[item.index]!.title }}</span>
        <span class="block truncate text-body-sm text-fg-subtle">{{ [songs[item.index]!.artist, songs[item.index]!.album].filter(Boolean).join(' · ') }}</span>
      </span>
      <span class="shrink-0 text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(songs[item.index]!.duration) }}</span>
    </li>
  </ol>
</template>
