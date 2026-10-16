<script setup lang="ts">
import type { Range } from '@tanstack/vue-virtual'
import type { ArtistView } from '~/utils/artist'
import type { ArtistIndex } from '~/utils/subsonic'
import { defaultRangeExtractor, useVirtualizer } from '@tanstack/vue-virtual'

/**
 * 読みの行の見出しで区切ったアーティストの行（docs/web.md の「一覧」）。
 * 画面に見える行だけを描き、今いる行の見出しは見出しの帯の下に残す。詳細は `view` の表示で開く。
 * 右端の頭文字の索引で、その見出しまで送る。
 */
const props = defineProps<{ index: ArtistIndex[], view: ArtistView }>()

const HEADING = 32
const ROW = 56

type Row = { kind: 'heading', name: string } | { kind: 'artist', id: string, name: string, albumCount: number }

const rows = computed<Row[]>(() => props.index.flatMap(group => [
  { kind: 'heading' as const, name: group.name },
  ...(group.artist ?? []).map(artist => ({ kind: 'artist' as const, ...artist })),
]))
const headings = computed(() => rows.value.flatMap((row, i) => row.kind === 'heading' ? [i] : []))

const scroller = useScroller()
const root = ref<HTMLElement>()

// 見出しの帯の高さ。今いる行の見出しを、その下に残す
const { height: headerHeight } = usePageHeader()

/** `start` 番目の行が属する見出しの位置。 */
function headingOf(start: number): number {
  return [...headings.value].reverse().find(i => start >= i) ?? 0
}

const virtualizer = useVirtualizer(computed(() => ({
  count: rows.value.length,
  getScrollElement: () => scroller.value ?? null,
  estimateSize: (i: number) => rows.value[i]?.kind === 'heading' ? HEADING : ROW,
  overscan: 8,
  scrollMargin: root.value?.offsetTop ?? 0,
  // 索引で送った見出しを、見出しの帯の下に出す
  scrollPaddingStart: headerHeight.value,
  rangeExtractor: (range: Range) => {
    const indexes = new Set([headingOf(range.startIndex), ...defaultRangeExtractor(range)])
    return [...indexes].sort((a, b) => a - b)
  },
})))

const items = computed(() => virtualizer.value.getVirtualItems())
// 帯のすぐ下の行が属する見出しを、帯の下に残す。帯に隠れた行で決めると、索引で送った見出しに前の見出しが重なるため
const active = computed(() => {
  if (!items.value.length) {
    return 0
  }
  const below = virtualizer.value.getVirtualItemForOffset((virtualizer.value.scrollOffset ?? 0) + headerHeight.value)
  return headingOf(below?.index ?? virtualizer.value.range?.startIndex ?? 0)
})

const headingNames = computed(() => headings.value.map(i => (rows.value[i] as { name: string }).name))

function jump(name: string) {
  const i = headings.value[headingNames.value.indexOf(name)]
  if (i !== undefined) {
    virtualizer.value.scrollToIndex(i, { align: 'start' })
  }
}
</script>

<template>
  <div ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="artist-list">
    <template v-for="item in items" :key="item.index">
      <div
        v-if="rows[item.index]?.kind === 'heading'"
        class="z-[5] flex items-end border-b border-divider bg-surface-0 px-3 pb-1.5 text-caption font-semibold text-fg-subtle md:bg-surface-1"
        :style="item.index === active
          ? { position: 'sticky', top: `${headerHeight}px`, height: `${HEADING}px` }
          : { position: 'absolute', top: 0, left: 0, right: 0, height: `${HEADING}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
        role="heading"
        aria-level="2"
      >
        {{ (rows[item.index] as { name: string }).name }}
      </div>
      <NuxtLink
        v-else
        :to="{ path: `/library/artists/${(rows[item.index] as { id: string }).id}`, query: { view } }"
        class="absolute inset-x-0 top-0 flex items-center gap-3 border-b border-divider px-3 transition-colors hover:bg-surface-2"
        :style="{ height: `${ROW}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
      >
        <ArtistInitial :name="(rows[item.index] as { name: string }).name" class="size-9 text-body" />
        <span class="min-w-0 flex-1">
          <span class="block truncate text-body">{{ (rows[item.index] as { name: string }).name }}</span>
          <span class="block truncate text-body-sm text-fg-subtle">アルバム {{ (rows[item.index] as { albumCount: number }).albumCount }} 枚</span>
        </span>
      </NuxtLink>
    </template>
    <IndexRail :headings="headingNames" :content-height="virtualizer.getTotalSize()" @select="jump" />
  </div>
</template>
