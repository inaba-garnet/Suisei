<script setup lang="ts">
import { Repeat, Shuffle, SkipBack, SkipForward } from 'lucide-vue-next'
import { formatDuration } from '~/utils/subsonic'

/** 広い画面の右の欄に置く再生プレイヤー（docs/web.md の「画面構成」「再生」）。 */
const { state, current, canPrevious, canNext, previous, next } = usePlayer()
const dev = useDev()
</script>

<template>
  <section
    class="flex h-full flex-col gap-5 overflow-y-auto rounded-xl border border-divider bg-surface-1 p-5"
    aria-label="再生プレイヤー"
    data-testid="player-panel"
  >
    <h2 class="text-body font-semibold">
      再生中
    </h2>
    <CoverArt :id="current?.coverArt" :size="400" :alt="current?.album" />
    <div v-if="current" class="min-w-0">
      <p class="truncate text-body-lg font-medium">
        {{ current.title }}
      </p>
      <p v-if="state.failed" class="truncate text-body-sm text-danger">
        再生できませんでした
      </p>
      <p v-else class="truncate text-body-sm text-fg-subtle">
        {{ [current.artist, current.album].filter(Boolean).join(' · ') }}
      </p>
    </div>
    <p v-else class="truncate text-body-lg text-fg-muted">
      再生していません
    </p>
    <div v-if="current" class="flex flex-col gap-1.5">
      <PlayerProgress class="h-1 rounded-full" />
      <div class="flex justify-between text-caption text-fg-subtle tabular-nums">
        <span>{{ formatDuration(state.position) }}</span>
        <span>{{ formatDuration(current.duration) }}</span>
      </div>
    </div>
    <div class="flex items-center justify-center gap-4">
      <!-- シャッフルとリピートはまだ使えないので、開発モードのときだけ押せない状態で出す（docs/web.md の「開発」） -->
      <Button v-if="dev" variant="ghost" size="icon" disabled aria-label="シャッフル">
        <Shuffle class="size-4" />
      </Button>
      <Button variant="ghost" size="icon" :disabled="!canPrevious" aria-label="前の曲" @click="previous">
        <SkipBack class="size-4" />
      </Button>
      <PlayerToggle size="lg" />
      <Button variant="ghost" size="icon" :disabled="!canNext" aria-label="次の曲" @click="next">
        <SkipForward class="size-4" />
      </Button>
      <Button v-if="dev" variant="ghost" size="icon" disabled aria-label="リピート">
        <Repeat class="size-4" />
      </Button>
    </div>
  </section>
</template>
