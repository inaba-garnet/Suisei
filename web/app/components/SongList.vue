<script setup lang="ts">
import type { Song } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { formatDuration } from '~/utils/subsonic'

/**
 * 曲の行。画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「一覧」）。
 * 行を押すと、その曲から一覧の曲を順に鳴らす（docs/web.md の「再生」）。
 * まだ読んでいない曲は `undefined` で受けて仮の行を描き、見えている範囲を `range` で知らせる。
 */
const props = defineProps<{ songs: (Song | undefined)[], more: boolean }>()
const emit = defineEmits<{ more: [], range: [start: number, end: number] }>()

const ROW = 56

const { current, start } = usePlayer()

const scroller = useScroller()
const { height: headerHeight } = usePageHeader()
const root = ref<HTMLElement>()

const virtualizer = useVirtualizer(computed(() => ({
  count: props.songs.length,
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
  if (more && (!last || last.index >= props.songs.length - 10)) {
    emit('more')
  }
  if (last) {
    emit('range', visible[0]!.index, last.index + 1)
  }
})

/** 押した曲から鳴らす。まだ読んでいない曲は飛ばし、読み込んだ曲だけをキューに入れる。 */
function play(index: number) {
  const loaded = props.songs.filter((song): song is Song => !!song)
  start(loaded, props.songs.slice(0, index).filter(Boolean).length)
}

/** `index` 番目の曲を、見出しの帯のすぐ下まで送る。 */
function scrollToItem(index: number) {
  virtualizer.value.scrollToIndex(index, { align: 'start' })
}

defineExpose({ scrollToItem })
</script>

<template>
  <ol ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="songs">
    <li
      v-for="item in items"
      :key="songs[item.index]?.id ?? `placeholder-${item.index}`"
      class="absolute inset-x-0 top-0 flex items-center gap-3 border-b border-divider px-3"
      :style="{ height: `${ROW}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
    >
      <div v-if="!songs[item.index]" class="flex flex-1 items-center gap-3" aria-hidden="true" data-testid="song-placeholder">
        <span class="size-10 shrink-0 rounded bg-surface-2" />
        <span class="flex flex-1 flex-col gap-1.5">
          <span class="h-3.5 w-1/2 rounded bg-surface-2" />
          <span class="h-3 w-1/3 rounded bg-surface-2" />
        </span>
      </div>
      <template v-else>
      <button type="button" class="flex min-w-0 flex-1 items-center gap-3 self-stretch text-left" @click="play(item.index)">
        <CoverArt :id="songs[item.index]!.coverArt" :size="40" :alt="songs[item.index]!.album" class="size-10 shrink-0" />
        <span class="min-w-0 flex-1">
          <span class="block truncate text-body" :class="{ 'text-accent-base': current?.id === songs[item.index]!.id }">{{ songs[item.index]!.title }}</span>
          <span class="block truncate text-body-sm text-fg-subtle">{{ [songs[item.index]!.artist, songs[item.index]!.album].filter(Boolean).join(' · ') }}</span>
        </span>
      </button>
      <HeartButton :id="songs[item.index]!.id" target="song" :starred="!!songs[item.index]!.starred" :label="`${songs[item.index]!.title}をお気に入りにする`" />
      <span class="w-12 shrink-0 text-right text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(songs[item.index]!.duration) }}</span>
      <SongMenuButton :target="{ kind: 'song', song: songs[item.index]! }" :label="`${songs[item.index]!.title}のメニュー`" />
      </template>
    </li>
  </ol>
</template>
