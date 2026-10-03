<script setup lang="ts">
import { ChevronDown, SkipBack, SkipForward } from 'lucide-vue-next'
import { coverArtUrl } from '~/utils/subsonic'

/**
 * 再生プレイヤー（docs/web.md の「画面構成」「再生」）。広い画面の右の欄に置き、
 * `sheet` ならスマホの下から引き出す再生画面として、枠を付けずに閉じる矢印を出す。
 * キューを開くと、ジャケットと曲名の場所をキューに入れ替える。下の操作の段はそのまま残す。
 */
const props = defineProps<{ sheet?: boolean }>()
defineEmits<{ close: [] }>()

const { state, current, canPrevious, canNext, previous, next } = usePlayer()
const queue = usePlayerQueueOpen()
const showQueue = computed(() => queue.value && !!current.value)

/**
 * 余白と間隔。キューを出している間は、下の操作の段の間隔と下の余白を詰めて下に寄せ、キューを大きく見せる。
 * 操作の段はジャケットの表示と同じものを残す。
 */
const layout = computed(() => {
  if (props.sheet) {
    return showQueue.value ? 'gap-3 px-6 pt-6 pb-[calc(env(safe-area-inset-bottom)+8px)]' : 'gap-5 px-6 pt-6 pb-[calc(env(safe-area-inset-bottom)+24px)]'
  }
  return showQueue.value ? 'gap-3 px-5 pt-5 pb-3' : 'gap-5 p-5'
})
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
      <!-- 曲名、キュー、操作の段の文字は背景に直接乗るので、明るいジャケットでも薄い文字が読めるよう、全体を暗くする -->
      <div class="absolute inset-0 bg-gradient-to-b from-surface-0/70 to-surface-0/85" />
    </div>
    <div
      class="flex h-full flex-col"
      :class="[layout, { 'overflow-y-auto': !showQueue }]"
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
          {{ showQueue ? 'キュー' : '再生中' }}
        </h2>
        <span class="size-11 shrink-0" />
      </div>
      <h2 v-else class="text-body font-semibold">
        {{ showQueue ? 'キュー' : '再生中' }}
      </h2>
      <PlayerQueue v-if="showQueue" :sheet="sheet" class="-mx-1 min-h-0 flex-1 px-1" />
      <template v-else>
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
      </template>
      <PlayerSeek v-if="current" times />
      <div class="flex items-center justify-center gap-4" :class="{ 'mb-auto': sheet && !showQueue }">
        <PlayerModes mode="shuffle" />
        <Button variant="ghost" size="icon" :disabled="!canPrevious" aria-label="前の曲" @click="previous">
          <SkipBack class="size-4" />
        </Button>
        <PlayerToggle size="lg" />
        <Button variant="ghost" size="icon" :disabled="!canNext" aria-label="次の曲" @click="next">
          <SkipForward class="size-4" />
        </Button>
        <PlayerModes mode="repeat" />
      </div>
      <!-- 音量は PC だけ（スマホは本体のボタンで変える） -->
      <PlayerVolume v-if="!sheet" class="justify-center" />
      <!-- 表示を切り替えるボタンの段。どちらの表示でも下端に置き、後で歌詞のボタンも同じ段に並べる -->
      <div class="flex items-center justify-end" :class="{ 'mt-auto': !sheet }">
        <PlayerQueueButton />
      </div>
    </div>
  </section>
</template>
