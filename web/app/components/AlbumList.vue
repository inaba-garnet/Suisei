<script setup lang="ts">
import type { Album } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'

/**
 * アルバムの行。ジャケットを見分けやすいよう曲の行より高くし、画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「一覧」）。
 * まだ読んでいないアルバムは `undefined` で受けて仮の行を描き、見えている範囲を `range` で知らせる。
 */
const props = defineProps<{ albums: (Album | undefined)[], more: boolean }>()
const emit = defineEmits<{ more: [], range: [start: number, end: number] }>()

const ROW = 72

const scroller = useScroller()
const { height: headerHeight } = usePageHeader()
const root = ref<HTMLElement>()

const virtualizer = useVirtualizer(computed(() => ({
  count: props.albums.length,
  getScrollElement: () => scroller.value ?? null,
  estimateSize: () => ROW,
  overscan: 8,
  scrollMargin: root.value?.offsetTop ?? 0,
  // 索引で送った行を、見出しの帯の下に出す
  scrollPaddingStart: headerHeight.value,
})))

const items = computed(() => virtualizer.value.getVirtualItems())

watch([items, () => props.more], ([visible, more]) => {
  const last = visible.at(-1)
  if (more && (!last || last.index >= props.albums.length - 10)) {
    emit('more')
  }
  if (last) {
    emit('range', visible[0]!.index, last.index + 1)
  }
})

/** `index` 番目のアルバムを、見出しの帯のすぐ下まで送る。 */
function scrollToItem(index: number) {
  virtualizer.value.scrollToIndex(index, { align: 'start' })
}

defineExpose({ scrollToItem })
</script>

<template>
  <ol ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="album-list">
    <li
      v-for="item in items"
      :key="albums[item.index]?.id ?? `placeholder-${item.index}`"
      class="absolute inset-x-0 top-0 border-b border-divider"
      :style="{ height: `${ROW}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
    >
      <div v-if="!albums[item.index]" class="flex h-full items-center gap-3 px-3" aria-hidden="true" data-testid="album-placeholder">
        <span class="size-14 shrink-0 rounded-md bg-surface-2" />
        <span class="flex flex-1 flex-col gap-1.5">
          <span class="h-3.5 w-1/2 rounded bg-surface-2" />
          <span class="h-3 w-1/3 rounded bg-surface-2" />
        </span>
      </div>
      <NuxtLink v-else :to="`/library/albums/${albums[item.index]!.id}`" class="flex h-full items-center gap-3 px-3 transition-colors hover:bg-surface-2">
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
