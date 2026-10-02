<script setup lang="ts">
import { ChevronDown, Repeat, Shuffle, SkipBack, SkipForward } from 'lucide-vue-next'
import { coverArtUrl, formatDuration } from '~/utils/subsonic'

/**
 * 再生プレイヤー（docs/web.md の「画面構成」「再生」）。広い画面の右の欄に置き、
 * `sheet` ならスマホの下から引き出す再生画面として、枠を付けずに閉じる矢印を出す。
 */
defineProps<{ sheet?: boolean }>()
defineEmits<{ close: [] }>()

const { state, current, canPrevious, canNext, previous, next } = usePlayer()
const dev = useDev()
</script>

<template>
  <section
    class="relative isolate h-full overflow-hidden"
    :class="sheet ? 'bg-surface-0' : 'rounded-xl border border-divider bg-surface-1'"
    aria-label="再生プレイヤー"
    data-testid="player-panel"
  >
    <!--
      スマホの再生画面では、再生中の曲のジャケットをぼかして暗くした背景を敷き、曲が変わったら重ねて入れ替える（docs/web.md の「再生」）。
      ぼかすと色が薄まりやすいので、アルバムの詳細と同じく濃く出す
    -->
    <div v-if="sheet" aria-hidden="true" class="pointer-events-none absolute inset-0 -z-10" data-testid="player-backdrop">
      <Transition
        enter-active-class="transition-opacity duration-700"
        enter-from-class="opacity-0"
        leave-active-class="transition-opacity duration-700"
        leave-to-class="opacity-0"
      >
        <img
          v-if="current?.coverArt"
          :key="current.coverArt"
          :src="coverArtUrl(current.coverArt, 300)"
          alt=""
          class="absolute inset-0 size-full scale-125 object-cover opacity-90 blur-xl"
        >
      </Transition>
      <div class="absolute inset-0 bg-gradient-to-b from-surface-0/20 via-surface-0/45 to-surface-0/80" />
    </div>
    <div
      class="flex h-full flex-col gap-5 overflow-y-auto"
      :class="sheet ? 'px-6 pt-6 pb-[calc(env(safe-area-inset-bottom)+24px)]' : 'p-5'"
    >
      <div v-if="sheet" class="flex min-h-11 items-center">
        <button
          type="button"
          aria-label="再生画面を閉じる"
          class="-ml-2.5 flex size-11 shrink-0 items-center justify-center rounded-full text-fg transition-colors hover:bg-surface-2"
          @click="$emit('close')"
        >
          <ChevronDown class="size-6" />
        </button>
        <h2 class="flex-1 text-center text-body font-semibold">
          再生中
        </h2>
        <span class="size-11 shrink-0" />
      </div>
      <h2 v-else class="text-body font-semibold">
        再生中
      </h2>
      <!-- ジャケットは枠を付けず、再生のボタンと同じく再生中は強く、止めているときは弱く光らせる -->
      <CoverArt
        :id="current?.coverArt"
        :size="400"
        :alt="current?.album"
        frameless
        class="transition-shadow duration-500"
        :class="[state.playing ? 'shadow-[0_0_40px_var(--glow-high)]' : 'shadow-[0_0_12px_var(--glow-low)]', { 'mt-auto': sheet }]"
      />
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
      <div class="flex items-center justify-center gap-4" :class="{ 'mb-auto': sheet }">
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
    </div>
  </section>
</template>
