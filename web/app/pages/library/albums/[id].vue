<script setup lang="ts">
import type { AlbumWithSongs, Song } from '~/utils/subsonic'
import { Play } from 'lucide-vue-next'
import { coverArtUrl, formatDuration } from '~/utils/subsonic'

const route = useRoute()
const subsonic = useSubsonic()
const { current, start } = usePlayer()
const id = computed(() => String(route.params.id))

const { data: album, status, refresh } = useAsyncData(
  () => `album:${id.value}`,
  async () => (await subsonic<{ album: AlbumWithSongs }>('getAlbum', { id: id.value })).album,
)

useHead({ title: () => album.value?.name ?? 'アルバム' })

const songs = computed(() => album.value?.song ?? [])
const multiDisc = computed(() => new Set(songs.value.map(song => song.discNumber ?? 1)).size > 1)
const details = computed(() => {
  const a = album.value
  if (!a) {
    return ''
  }
  return [
    a.year,
    a.genres?.map(genre => genre.name).join('、'),
    `${a.songCount} 曲`,
    formatDuration(a.duration),
  ].filter(Boolean).join(' · ')
})

/** 前の曲とディスクが変わるところで見出しを出す。 */
function discHeading(index: number): number | undefined {
  if (!multiDisc.value) {
    return undefined
  }
  const disc = songs.value[index]?.discNumber ?? 1
  const previous = index > 0 ? songs.value[index - 1]?.discNumber ?? 1 : undefined
  return disc !== previous ? disc : undefined
}

/** 曲のアーティストがアルバムと違うときだけ出す。 */
function songArtist(song: Song): string | undefined {
  return song.artist && song.artist !== album.value?.artist ? song.artist : undefined
}

// 大きな見出しが帯の下に隠れたら、帯にアルバムの名前を出す
const heading = ref<HTMLElement>()
const cover = ref()
const { titleVisible, bandOpacity, coverOpacity } = useDetailHeader(heading, cover)
</script>

<template>
  <div>
    <!-- ジャケットをぼかした背景。パネルの幅いっぱいに敷き、内容と一緒にスクロールする（docs/web.md の「一覧」） -->
    <div
      v-if="album?.coverArt"
      aria-hidden="true"
      class="pointer-events-none absolute inset-x-0 top-0 h-[30rem] overflow-hidden md:h-96"
      data-testid="album-hero"
    >
      <!-- スマホは幅が狭く、ぼかすと色が薄まりやすいので、濃く出して暗くする範囲を下に寄せる -->
      <img :src="coverArtUrl(album.coverArt, 300)" alt="" class="size-full scale-125 object-cover opacity-90 blur-xl md:opacity-50 md:blur-2xl">
      <div class="absolute inset-0 bg-gradient-to-b from-surface-0/20 via-surface-0/55 to-surface-0 md:from-surface-1/30 md:via-surface-1/65 md:to-surface-1" />
    </div>

    <PageHeader
      :title="album?.name ?? 'アルバム'"
      :back="{ to: '/library/albums' }"
      detail
      overlay
      :title-visible="titleVisible"
      :cover="album?.coverArt"
      :band-opacity="bandOpacity"
    />

    <p v-if="status === 'pending' && !album" class="relative py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="!album" class="relative flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        アルバムを読み込めませんでした
      </p>
      <Button variant="secondary" @click="refresh()">
        もう一度読み込む
      </Button>
    </div>
    <div v-else class="relative">
      <header class="mb-8 flex flex-col gap-5 sm:flex-row sm:items-end">
        <CoverArt :id="album.coverArt" ref="cover" :size="240" :style="{ opacity: coverOpacity }" :alt="album.name" class="w-44 shrink-0 shadow-[0_8px_32px_rgb(0_0_0/0.35)] sm:w-56" data-testid="album-cover" />
        <div class="flex min-w-0 flex-col gap-1.5">
          <h1 ref="heading" class="line-clamp-2 text-display font-semibold break-words" :title="album.name">
            {{ album.name }}
          </h1>
          <p class="truncate text-body-lg text-fg-muted">
            <template v-if="album.artists?.length">
              <template v-for="(artist, i) in album.artists" :key="artist.id">
                <span v-if="i > 0">、</span>
                <NuxtLink :to="{ path: `/library/artists/${artist.id}`, query: { view: 'albums' } }" class="hover:text-fg hover:underline">
                  {{ artist.name }}
                </NuxtLink>
              </template>
            </template>
            <template v-else>
              {{ album.artist }}
            </template>
          </p>
          <p class="text-body-sm text-fg-subtle">
            {{ details }}
          </p>
          <div class="mt-3 flex items-center gap-3">
            <Button size="lg" :disabled="!songs.length" @click="start(songs)">
              <Play class="size-4" />
              すべて再生
            </Button>
            <HeartButton :id="album.id" target="album" :starred="!!album.starred" label="アルバムをお気に入りにする" size="lg" />
            <SongMenuButton :target="{ kind: 'album', album, songs }" label="アルバムのメニュー" size="lg" />
          </div>
        </div>
      </header>

      <ol data-testid="songs">
        <template v-for="(song, index) in songs" :key="song.id">
          <li v-if="discHeading(index)" class="border-b border-divider px-3 pt-4 pb-2 text-caption font-medium text-fg-subtle">
            ディスク {{ discHeading(index) }}
          </li>
          <li class="flex h-14 items-center gap-3 border-b border-divider px-3">
            <button type="button" class="flex min-w-0 flex-1 items-center gap-3 self-stretch text-left" @click="start(songs, index)">
              <span class="w-6 shrink-0 text-right text-body-sm text-fg-subtle tabular-nums">{{ song.track ?? '' }}</span>
              <span class="min-w-0 flex-1">
                <span class="block truncate text-body" :class="{ 'text-accent-base': current?.id === song.id }">{{ song.title }}</span>
                <span v-if="songArtist(song)" class="block truncate text-body-sm text-fg-subtle">{{ songArtist(song) }}</span>
              </span>
            </button>
            <HeartButton :id="song.id" target="song" :starred="!!song.starred" :label="`${song.title}をお気に入りにする`" />
            <span class="w-12 shrink-0 text-right text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(song.duration) }}</span>
            <SongMenuButton :target="{ kind: 'song', song }" :label="`${song.title}のメニュー`" />
          </li>
        </template>
      </ol>
    </div>
  </div>
</template>
