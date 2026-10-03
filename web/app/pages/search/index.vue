<script setup lang="ts">
import type { SearchResult } from '~/utils/search'
import { watchDebounced } from '@vueuse/core'
import { Search, SearchX } from 'lucide-vue-next'
import { searchKinds, searchQuery } from '~/utils/search'

useHead({ title: '検索' })

const route = useRoute()
const router = useRouter()
const subsonic = useSubsonic()

// 検索語は URL の `q` に持ち、「戻る」や再読み込みでも同じ結果を出す（docs/web.md の「検索」）
const query = computed(() => searchQuery(route.query.q))
const text = ref(typeof route.query.q === 'string' ? route.query.q : '')

// 打ち終えてから呼ぶ。履歴は積まずに置き換える
watchDebounced(text, (value) => {
  const q = value.trim()
  if (q !== query.value) {
    router.replace({ query: q ? { q } : {} })
  }
}, { debounce: 300 })

// タブを押し直して `q` が消えたら、入力も空にする
watch(query, (q) => {
  if (q !== text.value.trim()) {
    text.value = q
  }
})

const result = shallowRef<SearchResult>()
const loading = ref(false)
const error = ref(false)
let latest = 0

// 「すべて見る」を出すかを知るため、出す件数より 1 件多く頼む
async function search(q: string) {
  const id = ++latest
  if (!q) {
    result.value = undefined
    return
  }
  loading.value = true
  error.value = false
  try {
    const [artists, albums, songs] = searchKinds.map(k => k.preview + 1) as [number, number, number]
    const res = await subsonic<{ searchResult3: SearchResult }>('search3', {
      query: q,
      artistCount: artists,
      albumCount: albums,
      songCount: songs,
    })
    if (id === latest) {
      result.value = res.searchResult3
    }
  }
  catch {
    if (id === latest) {
      error.value = true
    }
  }
  finally {
    if (id === latest) {
      loading.value = false
    }
  }
}
watch(query, search, { immediate: true })

/** 種類ごとに、出す分と「すべて見る」を出すか。結果がまだなければ `undefined`。 */
const sections = computed(() => {
  const res = result.value
  if (!res) {
    return undefined
  }
  const [artists, albums, songs] = searchKinds.map(k => k.preview) as [number, number, number]
  return {
    artists: { items: (res.artist ?? []).slice(0, artists), more: (res.artist?.length ?? 0) > artists },
    albums: { items: (res.album ?? []).slice(0, albums), more: (res.album?.length ?? 0) > albums },
    songs: { items: (res.song ?? []).slice(0, songs), more: (res.song?.length ?? 0) > songs },
  }
})
const empty = computed(() => !!sections.value && Object.values(sections.value).every(s => s.items.length === 0))
</script>

<template>
  <div>
    <PageHeader title="検索">
      <label class="relative block">
        <span class="sr-only">検索語</span>
        <Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-fg-subtle" />
        <Input
          v-model="text"
          type="search"
          placeholder="アーティスト、アルバム、曲"
          enterkeyhint="search"
          autocomplete="off"
          :autofocus="!query"
          class="pl-9"
        />
      </label>
    </PageHeader>

    <EmptyState v-if="!query" :icon="Search" title="ライブラリを探す" description="アーティスト、アルバム、曲の名前や読みで探せます。" />
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        検索できませんでした
      </p>
      <Button variant="secondary" @click="search(query)">
        もう一度検索する
      </Button>
    </div>
    <p v-else-if="!sections && loading" class="py-6 text-center text-body-sm text-fg-subtle">
      検索中…
    </p>
    <EmptyState v-else-if="empty" :icon="SearchX" :title="`「${query}」は見つかりませんでした`" />
    <div v-else-if="sections" class="space-y-8 transition-opacity" :class="{ 'opacity-60': loading }" data-testid="search-results">
      <template v-for="{ kind, label } in searchKinds" :key="kind">
        <section v-if="sections[kind].items.length" :aria-label="label">
          <div class="mb-3 flex items-baseline justify-between">
            <h2 class="text-h2 font-semibold">
              {{ label }}
            </h2>
            <NuxtLink
              v-if="sections[kind].more"
              :to="{ path: `/search/${kind}`, query: { q: query } }"
              class="text-body-sm text-fg-subtle transition-colors hover:text-fg"
            >
              すべて見る
            </NuxtLink>
          </div>
          <ArtistRows v-if="kind === 'artists'" :artists="sections.artists.items" :more="false" />
          <AlbumShelf v-else-if="kind === 'albums'" :albums="sections.albums.items" caption="artist" />
          <SongList v-else :songs="sections.songs.items" :more="false" />
        </section>
      </template>
    </div>
  </div>
</template>
