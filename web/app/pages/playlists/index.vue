<script setup lang="ts">
import type { Playlist } from '~/utils/subsonic'
import { ListMusic, Plus } from 'lucide-vue-next'
import { formatDuration } from '~/utils/subsonic'

useHead({ title: 'プレイリスト' })

const subsonic = useSubsonic()
const { data: playlists, status, refresh } = useAsyncData(
  'playlists',
  async () => (await subsonic<{ playlists: { playlist?: Playlist[] } }>('getPlaylists')).playlists.playlist ?? [],
)

// 新しいプレイリストは、見出しの帯の下に名前の欄を出して作る（docs/web.md の「プレイリスト」）
const creating = ref(false)
const name = ref('')
const saving = ref(false)
const failed = ref(false)

function startCreate() {
  name.value = ''
  failed.value = false
  creating.value = true
}

async function create() {
  const value = name.value.trim()
  if (!value || saving.value) {
    return
  }
  saving.value = true
  failed.value = false
  try {
    const res = await subsonic<{ playlist: Playlist }>('createPlaylist', { name: value })
    creating.value = false
    await navigateTo(`/playlists/${res.playlist.id}`)
  }
  catch {
    failed.value = true
  }
  finally {
    saving.value = false
  }
}

function details(playlist: Playlist): string {
  return `${playlist.songCount} 曲 · ${formatDuration(playlist.duration)}`
}
</script>

<template>
  <div>
    <PageHeader title="プレイリスト">
      <form v-if="creating" class="flex items-center gap-2" @submit.prevent="create">
        <label class="min-w-0 flex-1">
          <span class="sr-only">新しいプレイリストの名前</span>
          <Input v-model="name" placeholder="プレイリストの名前" autocomplete="off" autofocus enterkeyhint="done" />
        </label>
        <Button type="submit" :disabled="!name.trim() || saving">
          作成
        </Button>
        <Button type="button" variant="ghost" @click="creating = false">
          キャンセル
        </Button>
      </form>
      <div v-else class="flex justify-end">
        <Button variant="secondary" @click="startCreate">
          <Plus class="size-4" />
          新規プレイリスト
        </Button>
      </div>
      <p v-if="failed" class="mt-2 text-body-sm text-danger">
        プレイリストを作れませんでした
      </p>
    </PageHeader>

    <p v-if="status === 'pending' && !playlists" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="!playlists" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        プレイリストを読み込めませんでした
      </p>
      <Button variant="secondary" @click="refresh()">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="!playlists.length" :icon="ListMusic" title="プレイリストがありません" description="「新規プレイリスト」から作れます。" />
    <ol v-else data-testid="playlists">
      <li v-for="playlist in playlists" :key="playlist.id" class="h-[72px] border-b border-divider">
        <NuxtLink :to="`/playlists/${playlist.id}`" class="flex h-full items-center gap-3 px-3 transition-colors hover:bg-surface-2">
          <CoverArt :id="playlist.coverArt" :size="56" :alt="playlist.name" class="size-14 shrink-0" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-body">{{ playlist.name }}</span>
            <span class="block truncate text-body-sm text-fg-subtle">{{ details(playlist) }}</span>
          </span>
        </NuxtLink>
      </li>
    </ol>
  </div>
</template>
