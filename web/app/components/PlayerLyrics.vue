<script setup lang="ts">
/**
 * 再生中の曲の歌詞（docs/web.md の「再生」）。再生プレイヤーのジャケットと曲名の場所に入れ替えて出す。
 * 時刻付きなら鳴っている行を明るくして上から 3 分の 1 の高さへ送り、行を押すとその位置へ移る。
 * 手でスクロールしたら、しばらくは送らずに読ませる。時刻なしなら並べるだけにする。
 */
const props = defineProps<{ sheet?: boolean }>()

const { state, seek } = usePlayer()
const lyrics = useLyrics()

/** 手でスクロールしてから、また鳴っている行へ送り始めるまで（ミリ秒）。 */
const RESUME_AFTER = 3000

const scroller = ref<HTMLElement>()
const lines: HTMLElement[] = []

/** 行の始まり（ミリ秒）。LRC の `[offset:]` と同じく、正の `offset` は歌詞を早める。 */
function startOf(index: number): number | undefined {
  const start = lyrics.value?.line[index]?.start
  return start === undefined ? undefined : start - (lyrics.value?.offset ?? 0)
}

/** 鳴っている行。最初の行より前なら -1。 */
const active = computed(() => {
  if (!lyrics.value?.synced) {
    return -1
  }
  const now = state.value.position * 1000
  let found = -1
  lyrics.value.line.forEach((_, i) => {
    const start = startOf(i)
    if (start !== undefined && start <= now) {
      found = i
    }
  })
  return found
})

const following = ref(true)
let resume: ReturnType<typeof setTimeout> | undefined

/** 手で動かしたら送るのを止め、しばらく動かさなければ戻す。送る側のスクロールでは呼ばない。 */
function onManualScroll() {
  following.value = false
  clearTimeout(resume)
  resume = setTimeout(() => {
    following.value = true
  }, RESUME_AFTER)
}
onBeforeUnmount(() => clearTimeout(resume))

function scrollToActive(behavior: ScrollBehavior) {
  const el = scroller.value
  const line = lines[active.value]
  if (!el || !line) {
    return
  }
  el.scrollTo({ top: line.offsetTop - el.clientHeight / 3, behavior })
}

watch(active, () => {
  if (following.value) {
    scrollToActive('smooth')
  }
})
watch(following, (value) => {
  if (value) {
    scrollToActive('smooth')
  }
})
// 開いたときと歌詞を取り終えたときは、動かして見せずにその場へ出す
watch(lyrics, () => nextTick(() => {
  if (scroller.value) {
    scroller.value.scrollTop = 0
  }
  scrollToActive('instant')
}), { flush: 'post' })
onMounted(() => scrollToActive('instant'))

function jump(index: number) {
  const start = startOf(index)
  if (start === undefined) {
    return
  }
  seek(Math.max(0, start) / 1000)
  following.value = true
  clearTimeout(resume)
}

const size = computed(() => props.sheet ? 'text-h1' : 'text-h2')
</script>

<template>
  <div class="relative min-h-0">
    <p v-if="lyrics === undefined" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <!-- 上下の端は透かして、行が途中で切れて見えないようにする。最後の行も上の 3 分の 1 まで送れるよう、下に余白を取る -->
    <div
      v-else-if="lyrics"
      ref="scroller"
      class="h-full overflow-y-auto pb-[60%] [mask-image:linear-gradient(to_bottom,transparent,black_24px,black_calc(100%-24px),transparent)]"
      data-testid="lyrics"
      @wheel.passive="onManualScroll"
      @touchmove.passive="onManualScroll"
    >
      <ol class="flex flex-col gap-4 pt-6">
        <li v-for="(line, index) in lyrics.line" :key="index" :ref="el => { lines[index] = el as HTMLElement }">
          <button
            v-if="lyrics.synced"
            type="button"
            class="w-full text-left font-semibold break-words transition-colors duration-300"
            :class="[size, index === active ? 'text-fg' : 'text-fg-subtle hover:text-fg-muted']"
            :aria-current="index === active ? 'true' : undefined"
            @click="jump(index)"
          >
            {{ line.value.trim() || '♪' }}
          </button>
          <p v-else class="font-semibold break-words text-fg" :class="size">
            {{ line.value || '&nbsp;' }}
          </p>
        </li>
      </ol>
    </div>
  </div>
</template>
