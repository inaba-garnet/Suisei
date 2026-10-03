<script setup lang="ts">
import type { Album } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'

/** アルバムの行。ジャケットを見分けやすいよう曲の行より高くし、画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「一覧」）。 */
const props = defineProps<{ albums: Album[], more: boolean }>()
const emit = defineEmits<{ more: [] }>()

const ROW = 72

const scroller = useScroller()
const root = ref<HTMLElement>()

const virtualizer = useVirtualizer(computed(() => ({
  count: props.albums.length,
  getScrollElement: () => scroller.value ?? null,
  estimateSize: () => ROW,
  overscan: 8,
  scrollMargin: root.value?.offsetTop ?? 0,
})))

const items = computed(() => virtualizer.value.getVirtualItems())

watch([items, () => props.more], ([visible, more]) => {
  const last = visible.at(-1)
  if (more && (!last || last.index >= props.albums.length - 10)) {
    emit('more')
  }
})
</script>

<template>
  <ol ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="album-list">
    <li
      v-for="item in items"
      :key="albums[item.index]!.id"
      class="absolute inset-x-0 top-0 border-b border-divider"
      :style="{ height: `${ROW}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
    >
      <NuxtLink :to="`/library/albums/${albums[item.index]!.id}`" class="flex h-full items-center gap-3 px-3 transition-colors hover:bg-surface-2">
        <CoverArt :id="albums[item.index]!.coverArt" :size="56" :alt="albums[item.index]!.name" class="size-14 shrink-0" />
        <span class="min-w-0 flex-1">
          <span class="block truncate text-body">{{ albums[item.index]!.name }}</span>
          <span class="block truncate text-body-sm text-fg-subtle">{{ [albums[item.index]!.artist, albums[item.index]!.year].filter(Boolean).join(' · ') }}</span>
        </span>
        <span class="shrink-0 text-body-sm text-fg-subtle tabular-nums">{{ albums[item.index]!.songCount }} 曲</span>
      </NuxtLink>
    </li>
  </ol>
</template>
