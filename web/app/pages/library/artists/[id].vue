<script setup lang="ts">
import type { ArtistWithAlbums } from '~/utils/subsonic'
import { useScroll } from '@vueuse/core'

const route = useRoute()
const subsonic = useSubsonic()
const id = computed(() => String(route.params.id))

const { data: artist, status, refresh } = useAsyncData(
  () => `artist:${id.value}`,
  async () => (await subsonic<{ artist: ArtistWithAlbums }>('getArtist', { id: id.value })).artist,
)

useHead({ title: () => artist.value?.name ?? 'アーティスト' })

// 大きな見出しが帯の下に隠れたら、帯にアーティストの名前を出す
const scroller = useScroller()
const { y } = useScroll(scroller)
const heading = ref<HTMLElement>()
const titleVisible = computed(() => {
  const el = heading.value
  const bar = scroller.value?.querySelector<HTMLElement>('[data-testid="page-header"]')
  return !!el && !!bar && y.value + bar.offsetHeight > el.offsetTop + el.offsetHeight
})
</script>

<template>
  <div>
    <PageHeader
      :title="artist?.name ?? 'アーティスト'"
      :back="{ to: '/library/artists' }"
      detail
      :title-visible="titleVisible"
    />

    <p v-if="status === 'pending' && !artist" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="!artist" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        アーティストを読み込めませんでした
      </p>
      <Button variant="secondary" @click="refresh()">
        もう一度読み込む
      </Button>
    </div>
    <template v-else>
      <header class="mb-6 flex items-center gap-4">
        <ArtistInitial :name="artist.name" class="size-20 text-h1 sm:size-24" />
        <div class="flex min-w-0 flex-col gap-1">
          <h1 ref="heading" class="line-clamp-2 break-words text-h1 font-semibold" :title="artist.name">
            {{ artist.name }}
          </h1>
          <p class="text-body-sm text-fg-subtle">
            アルバム {{ artist.albumCount }} 枚
          </p>
        </div>
      </header>

      <AlbumGrid :albums="artist.album ?? []" :more="false" />
    </template>
  </div>
</template>
