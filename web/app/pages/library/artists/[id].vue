<script setup lang="ts">
import type { ArtistWithAlbums, Song } from '~/utils/subsonic'
import { useLocalStorage, useScroll } from '@vueuse/core'
import { Music } from 'lucide-vue-next'
import { albumsAsAlbumArtist, artistSort, artistView, groupByAlbum, songRoles, sortAlbums, sortGroups, sortOptions, sortSongs } from '~/utils/artist'
import { formatDuration } from '~/utils/subsonic'

const route = useRoute()
const subsonic = useSubsonic()
const id = computed(() => String(route.params.id))
// どの一覧から開いたか（docs/web.md の「一覧」）
const view = computed(() => artistView(route.query.view))

const { data: artist, status, refresh } = useAsyncData(
  () => `artist:${id.value}:${view.value}`,
  async () => {
    const songs = songRoles[view.value]
    const params: Record<string, string> = songs ? { id: id.value, songs } : { id: id.value }
    return (await subsonic<{ artist: ArtistWithAlbums }>('getArtist', params)).artist
  },
)

useHead({ title: () => artist.value?.name ?? 'アーティスト' })

// 選んだ並び順は表示ごとに残す（docs/web.md の「一覧」）
const stored = {
  albums: useLocalStorage('suisei-artist-sort-albums', 'newest'),
  tracks: useLocalStorage('suisei-artist-sort-tracks', 'newest'),
  composer: useLocalStorage('suisei-artist-sort-composer', 'newest'),
}
const sort = computed({
  get: () => artistSort(stored[view.value].value),
  set: (value) => {
    stored[view.value].value = value
  },
})

const albums = computed(() => sortAlbums(albumsAsAlbumArtist(artist.value?.album ?? [], id.value), sort.value))
const songs = computed(() => artist.value?.song ?? [])
const sortedSongs = computed(() => sortSongs(songs.value, sort.value))
const groups = computed(() => sortGroups(groupByAlbum(songs.value), sort.value, artist.value?.album ?? []))
const summary = computed(() => view.value === 'albums' ? `アルバム ${albums.value.length} 枚` : `${songs.value.length} 曲`)

/** 曲のアーティストがこのアーティストだけなら出さない。 */
function songArtist(song: Song): string | undefined {
  return song.artist && song.artist !== artist.value?.name ? song.artist : undefined
}

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
      :back="{ to: view === 'composer' ? '/library/composers' : '/library/artists' }"
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
      <header class="mb-4 flex items-center gap-4">
        <ArtistInitial :name="artist.name" class="size-20 text-h1 sm:size-24" />
        <div class="flex min-w-0 flex-col gap-1">
          <h1 ref="heading" class="line-clamp-2 break-words text-h1 font-semibold" :title="artist.name">
            {{ artist.name }}
          </h1>
          <p class="text-body-sm text-fg-subtle">
            {{ summary }}
          </p>
        </div>
      </header>

      <div class="mb-4 flex justify-end">
        <SortSelect v-model="sort" :options="sortOptions[view]" />
      </div>

      <AlbumGrid v-if="view === 'albums'" :albums="albums" :more="false" />

      <template v-else-if="view === 'tracks'">
        <section v-if="albums.length" class="mb-8">
          <h2 class="mb-3 text-h2 font-semibold">
            アルバム
          </h2>
          <AlbumShelf :albums="albums" />
        </section>
        <section>
          <h2 class="mb-3 text-h2 font-semibold">
            曲
          </h2>
          <ol data-testid="songs">
            <template v-for="group in groups" :key="group.albumId">
              <li>
                <NuxtLink
                  :to="`/library/albums/${group.albumId}`"
                  class="flex items-center gap-4 border-b border-divider px-3 pt-5 pb-3 transition-colors hover:text-fg"
                  data-testid="song-group"
                >
                  <CoverArt :id="group.coverArt" :size="64" :alt="group.album" class="size-16 shrink-0" />
                  <span class="min-w-0 line-clamp-2 break-words text-body-lg font-semibold">{{ group.album }}</span>
                </NuxtLink>
              </li>
              <li v-for="song in group.songs" :key="song.id" class="flex h-12 items-center gap-3 border-b border-divider px-3">
                <span class="w-6 shrink-0 text-right text-body-sm text-fg-subtle tabular-nums">{{ song.track ?? '' }}</span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-body">{{ song.title }}</span>
                  <span v-if="songArtist(song)" class="block truncate text-body-sm text-fg-subtle">{{ songArtist(song) }}</span>
                </span>
                <span class="shrink-0 text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(song.duration) }}</span>
              </li>
            </template>
          </ol>
        </section>
      </template>

      <ol v-else data-testid="songs">
        <li v-for="song in sortedSongs" :key="song.id" class="flex h-14 items-center gap-3 border-b border-divider px-3">
          <CoverArt :id="song.coverArt" :size="40" :alt="song.album" class="size-10 shrink-0" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-body">{{ song.title }}</span>
            <span class="block truncate text-body-sm text-fg-subtle">{{ [song.artist, song.album].filter(Boolean).join(' · ') }}</span>
          </span>
          <span class="shrink-0 text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(song.duration) }}</span>
        </li>
      </ol>

      <EmptyState v-if="view !== 'albums' && status !== 'pending' && songs.length === 0" :icon="Music" title="曲がありません" />
    </template>
  </div>
</template>
