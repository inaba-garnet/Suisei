<script setup lang="ts">
import type { Album } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { useElementSize } from '@vueuse/core'
import { gridColumns } from '~/utils/grid'

/**
 * アルバムのジャケットのグリッド。画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「一覧」）。
 * まだ読んでいないアルバムは `undefined` で受けて仮の枠を描き、見えている範囲を `range` で知らせる。
 */
const props = defineProps<{ albums: (Album | undefined)[], more: boolean }>()
const emit = defineEmits<{ more: [], range: [start: number, end: number] }>()

const COLUMN_GAP = 16
const ROW_GAP = 20
/** ジャケットの下の名前とアーティストの 2 行と、その間の余白。 */
const CAPTION = 44

const scroller = useScroller()
const { height: headerHeight } = usePageHeader()
const root = ref<HTMLElement>()
const { width } = useElementSize(root)
const columns = computed(() => gridColumns(width.value))
const cell = computed(() => Math.max(0, (width.value - COLUMN_GAP * (columns.value - 1)) / columns.value))
const rows = computed(() => Math.ceil(props.albums.length / columns.value))

const virtualizer = useVirtualizer(computed(() => ({
  count: rows.value,
  getScrollElement: () => scroller.value ?? null,
  estimateSize: () => cell.value + CAPTION + ROW_GAP,
  overscan: 3,
  // グリッドの上にある見出しなどの高さ。パネルの中でのグリッドの位置
  scrollMargin: root.value?.offsetTop ?? 0,
  // 索引で送った行を、見出しの帯の下に出す
  scrollPaddingStart: headerHeight.value,
})))

watch(cell, () => virtualizer.value.measure())

const virtualRows = computed(() => virtualizer.value.getVirtualItems())

watch([virtualRows, () => props.more], ([items, more]) => {
  const last = items.at(-1)
  if (more && (!last || last.index >= rows.value - 3)) {
    emit('more')
  }
  if (items.length) {
    emit('range', items[0]!.index * columns.value, (last!.index + 1) * columns.value)
  }
})

function albumsOf(row: number): { index: number, album: Album | undefined }[] {
  return props.albums.slice(row * columns.value, (row + 1) * columns.value)
    .map((album, i) => ({ index: row * columns.value + i, album }))
}

/** `index` 番目のアルバムの行を、見出しの帯のすぐ下まで送る。 */
function scrollToItem(index: number) {
  virtualizer.value.scrollToIndex(Math.floor(index / columns.value), { align: 'start' })
}

defineExpose({ scrollToItem })
</script>

<template>
  <div ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="album-grid">
    <div
      v-for="row in virtualRows"
      :key="row.index"
      class="absolute inset-x-0 top-0 grid"
      :style="{
        transform: `translateY(${row.start - virtualizer.options.scrollMargin}px)`,
        gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
        columnGap: `${COLUMN_GAP}px`,
      }"
    >
      <template v-for="{ index, album } in albumsOf(row.index)" :key="album?.id ?? `placeholder-${index}`">
        <NuxtLink
          v-if="album"
          :to="`/library/albums/${album.id}`"
          class="group flex min-w-0 flex-col gap-2"
        >
          <CoverArt
            :id="album.coverArt"
            :size="150"
            :alt="album.name"
            class="transition-all group-hover:border-border-strong group-hover:shadow-[0_0_14px_var(--glow-low)]"
          />
          <span class="min-w-0">
            <span class="block truncate text-body font-medium">{{ album.name }}</span>
            <span class="block truncate text-body-sm text-fg-subtle">{{ album.artist }}</span>
          </span>
        </NuxtLink>
        <div v-else class="flex min-w-0 flex-col gap-2" aria-hidden="true" data-testid="album-placeholder">
          <div class="aspect-square rounded-lg bg-surface-2" />
          <span class="flex flex-col gap-1.5 py-0.5">
            <span class="h-3.5 w-3/4 rounded bg-surface-2" />
            <span class="h-3 w-1/2 rounded bg-surface-2" />
          </span>
        </div>
      </template>
    </div>
  </div>
</template>
