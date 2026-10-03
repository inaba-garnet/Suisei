<script setup lang="ts">
import type { Artist } from '~/utils/subsonic'
import { useVirtualizer } from '@tanstack/vue-virtual'

/**
 * 見出しで区切らないアーティストの行。画面に見える行だけを描き、末尾に近づいたら `more` を出す（docs/web.md の「検索」）。
 * 詳細は曲のアーティストの表示で開く。検索では曲にだけ参加している人も出るため。
 */
const props = defineProps<{ artists: Artist[], more: boolean }>()
const emit = defineEmits<{ more: [] }>()

const ROW = 56

const scroller = useScroller()
const root = ref<HTMLElement>()

const virtualizer = useVirtualizer(computed(() => ({
  count: props.artists.length,
  getScrollElement: () => scroller.value ?? null,
  estimateSize: () => ROW,
  overscan: 8,
  scrollMargin: root.value?.offsetTop ?? 0,
})))

const items = computed(() => virtualizer.value.getVirtualItems())

watch([items, () => props.more], ([visible, more]) => {
  const last = visible.at(-1)
  if (more && (!last || last.index >= props.artists.length - 10)) {
    emit('more')
  }
})
</script>

<template>
  <ol ref="root" class="relative" :style="{ height: `${virtualizer.getTotalSize()}px` }" data-testid="artist-rows">
    <li
      v-for="item in items"
      :key="artists[item.index]!.id"
      class="absolute inset-x-0 top-0 border-b border-divider"
      :style="{ height: `${ROW}px`, transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }"
    >
      <NuxtLink
        :to="{ path: `/library/artists/${artists[item.index]!.id}`, query: { view: 'tracks' } }"
        class="flex h-full items-center gap-3 px-3 transition-colors hover:bg-surface-2"
      >
        <ArtistInitial :name="artists[item.index]!.name" class="size-9 text-body" />
        <span class="min-w-0 flex-1 truncate text-body">{{ artists[item.index]!.name }}</span>
      </NuxtLink>
    </li>
  </ol>
</template>
