<script setup lang="ts">
import type { Album } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { useElementSize } from '@vueuse/core'
import { gridColumns } from '~/utils/grid'

/** アルバムのジャケットの格子。画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「一覧」）。 */
const props = defineProps<{ albums: Album[], more: boolean }>()
const emit = defineEmits<{ more: [] }>()

const COLUMN_GAP = 16
const ROW_GAP = 20
/** ジャケットの下の名前とアーティストの 2 行と、その間の余白。 */
const CAPTION = 44

const scroller = useScroller()
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
  // 格子の上にある見出しなどの高さ。パネルの中での格子の位置
  scrollMargin: root.value?.offsetTop ?? 0,
})))

watch(cell, () => virtualizer.value.measure())

const virtualRows = computed(() => virtualizer.value.getVirtualItems())

watch([virtualRows, () => props.more], ([items, more]) => {
  const last = items.at(-1)
  if (more && (!last || last.index >= rows.value - 3)) {
    emit('more')
  }
})

function albumsOf(row: number): Album[] {
  return props.albums.slice(row * columns.value, (row + 1) * columns.value)
}
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
      <NuxtLink
        v-for="album in albumsOf(row.index)"
        :key="album.id"
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
    </div>
  </div>
</template>
