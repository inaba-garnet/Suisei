<script setup lang="ts">
import type { SpotifyStatus } from '~/utils/spotify'
import { useIntervalFn } from '@vueuse/core'
import { ChevronRight, ExternalLink, RefreshCw, Unplug } from 'lucide-vue-next'
import { callbackErrorMessage, lastSyncMessage } from '~/utils/spotify'

/** 設定の画面の「Spotify」の項目（docs/web.md の「Spotify」）。サーバーに Client ID がなければ何も出さない。 */
const spotify = useSpotify()
// 「5 分前」の表示を進める
const now = ref(Date.now())
useIntervalFn(() => {
  now.value = Date.now()
}, 30_000)

const status = ref<SpotifyStatus>()
const configured = computed(() => status.value?.configured === true ? status.value : undefined)
const unmatched = computed(() => configured.value ? configured.value.total - configured.value.matched : 0)

async function refresh() {
  try {
    status.value = await spotify.status()
    now.value = Date.now()
  }
  catch {
    // 状態を読めなければ項目を出さない。ほかの設定は使えるようにする
    status.value = undefined
  }
}
onMounted(refresh)

// 取り込み中は 2 秒ごとに状態を取り直す
const polling = useIntervalFn(refresh, 2000, { immediate: false })
watch(() => configured.value?.syncing, (syncing) => {
  if (syncing) {
    polling.resume()
  }
  else {
    polling.pause()
  }
})

// 認可を始めたら、開けなかったページの URL を貼る欄を出す
const started = ref(false)
const pasted = ref('')
const pasteError = ref('')
const busy = ref(false)

async function onConnect() {
  // 非同期の処理の後に開くとポップアップとして止められるので、先にタブを開いてから URL を入れる
  const tab = window.open('', '_blank')
  try {
    const { url } = await spotify.authorize()
    if (tab) {
      tab.opener = null
      tab.location.href = url
    }
    else {
      window.location.assign(url)
    }
    started.value = true
    pasteError.value = ''
  }
  catch {
    tab?.close()
    pasteError.value = callbackErrorMessage(undefined)
  }
}

async function onPaste() {
  busy.value = true
  pasteError.value = ''
  try {
    await spotify.callback(pasted.value)
  }
  catch (err) {
    pasteError.value = callbackErrorMessage(spotifyErrorOf(err))
    busy.value = false
    return
  }
  started.value = false
  pasted.value = ''
  try {
    // 次のスキャンを待たずに結果を見せる
    await spotify.sync()
  }
  finally {
    await refresh()
    busy.value = false
  }
}

async function onSync() {
  busy.value = true
  try {
    await spotify.sync()
    await refresh()
  }
  finally {
    busy.value = false
  }
}

async function onDisconnect() {
  busy.value = true
  try {
    await spotify.disconnect()
    await refresh()
  }
  finally {
    busy.value = false
  }
}
</script>

<template>
  <section v-if="configured" class="mt-8 flex flex-col gap-3" data-testid="spotify-settings">
    <h2 class="text-h3 font-semibold">
      Spotify
    </h2>

    <template v-if="configured.connected">
      <p class="text-body" data-testid="spotify-counts">
        Spotify のお気に入り {{ configured.total }} 曲のうち、{{ configured.matched }} 曲をお気に入りにしています。
      </p>
      <p class="text-body-sm text-fg-subtle" data-testid="spotify-last-sync">
        <template v-if="configured.syncing">
          取り込み中…
        </template>
        <template v-else-if="configured.lastSync">
          {{ lastSyncMessage(configured.lastSync, now) }}
        </template>
        <template v-else>
          スキャンの後に、1 時間おきに取り込みます。
        </template>
      </p>
      <div class="flex flex-wrap gap-2">
        <Button variant="secondary" :disabled="busy || configured.syncing" @click="onSync">
          <RefreshCw class="size-3.5" :class="{ 'animate-spin': configured.syncing }" />
          今すぐ取り込む
        </Button>
        <Button variant="ghost" :disabled="busy" @click="onDisconnect">
          <Unplug class="size-3.5" />
          接続を解除
        </Button>
      </div>
    </template>

    <template v-else>
      <p class="text-body text-fg-muted">
        Spotify でお気に入りにした曲を、ライブラリの同じ曲でもお気に入りにします。
        Spotify for Developers で作ったアプリに、次の Redirect URI を登録してください。
      </p>
      <code class="self-start rounded-md bg-surface-2 px-2 py-1 text-body-sm" data-testid="spotify-redirect-uri">{{ configured.redirectUri }}</code>
      <Button class="self-start" :disabled="busy" @click="onConnect">
        <ExternalLink class="size-3.5" />
        Spotify に接続
      </Button>
      <form v-if="started" class="flex flex-col gap-2" @submit.prevent="onPaste">
        <Label for="spotify-pasted">許可した後に開けなかったページの URL を貼り付けてください</Label>
        <div class="flex gap-2">
          <Input
            id="spotify-pasted"
            v-model="pasted"
            :placeholder="`${configured.redirectUri}?code=…`"
            autocomplete="off"
            :aria-invalid="pasteError ? 'true' : undefined"
          />
          <Button type="submit" size="lg" :disabled="busy || !pasted.trim()">
            接続
          </Button>
        </div>
      </form>
      <p v-if="pasteError" class="text-body-sm text-danger" role="alert">
        {{ pasteError }}
      </p>
    </template>

    <NuxtLink
      v-if="unmatched > 0"
      to="/settings/spotify"
      class="flex items-center gap-2 border-y border-divider px-3 py-3 text-body transition-colors hover:bg-surface-2"
    >
      <span class="flex-1">未対応の曲</span>
      <span class="text-fg-subtle" data-testid="spotify-unmatched">{{ unmatched }} 曲</span>
      <ChevronRight class="size-4 text-fg-subtle" />
    </NuxtLink>
  </section>
</template>
