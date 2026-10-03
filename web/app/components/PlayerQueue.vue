<script setup lang="ts">
import { useEventListener, useResizeObserver } from '@vueuse/core'
import Sortable from 'sortablejs'

/**
 * 再生のキュー（docs/web.md の「再生」）。再生中の曲を上に固定し、その下に「再生履歴」と「次に再生」を並べる。
 * 開いたときは「次に再生」を上に送って再生履歴を隠し、下にスクロールすると再生履歴が出る。並べ替えられるのは次に再生する曲だけ。
 */
/** `sheet` ならスマホの再生画面の中に置き、曲を消すのは行を左に払う操作にする */
defineProps<{ sheet?: boolean }>()

const { state, current, move } = usePlayer()

/** 同じ曲が二度入っていても行を見分けられるよう、何度目かを鍵に足す。 */
const rows = computed(() => {
  const seen = new Map<string, number>()
  return state.value.queue.map((song, index) => {
    const n = seen.get(song.id) ?? 0
    seen.set(song.id, n + 1)
    return { song, index, key: `${song.id}#${n}` }
  })
})
const history = computed(() => rows.value.slice(0, Math.max(0, state.value.index)))
const upcoming = computed(() => rows.value.slice(state.value.index + 1))

const scroller = ref<HTMLElement>()
const upcomingSection = ref<HTMLElement>()
const list = ref<HTMLElement>()

/**
 * 一覧の上端に重ねて出す見出し。見出しに背景を敷かずに上に残すため、一覧の上端は見出しの高さぶん透かし、
 * そこに今いる区切りの見出しを重ねる。
 */
const stuck = ref('次に再生')
function updateStuck() {
  const el = scroller.value
  const section = upcomingSection.value
  stuck.value = el && section && history.value.length && el.scrollTop < section.offsetTop ? '再生履歴' : '次に再生'
}
useEventListener(scroller, 'scroll', updateStuck, { passive: true })
watch(() => history.value.length, () => nextTick(updateStuck))

// 再生履歴は上に隠し、「次に再生」から見せる。隠れた右の欄で開いたときは高さがないので、見えたときに合わせる
let positioned = false
useResizeObserver(scroller, () => {
  if (!positioned && scroller.value && upcomingSection.value && scroller.value.clientHeight > 0) {
    scroller.value.scrollTop = upcomingSection.value.offsetTop
    positioned = true
    updateStuck()
  }
})

let sortable: Sortable | undefined
/** 引き始めた行の次の node。SortableJS が動かした行を元の位置に戻すために覚える */
let nextSibling: Node | null = null

onMounted(() => {
  sortable = Sortable.create(list.value!, {
    handle: '[data-queue-handle]',
    // HTML の drag and drop ではなく pointer の event で動かし、マウスと指で同じ動きにする
    forceFallback: true,
    fallbackOnBody: true,
    animation: 150,
    ghostClass: 'opacity-40',
    onStart: ({ item }) => {
      nextSibling = item.nextSibling
    },
    onEnd: ({ item, from, oldIndex, newIndex }) => {
      if (oldIndex === undefined || newIndex === undefined || oldIndex === newIndex) {
        return
      }
      // 行の並べ方は Vue に任せるので、SortableJS が動かした行を元の位置に戻してからキューを並べ替える
      from.insertBefore(item, nextSibling)
      const first = state.value.index + 1
      move(first + oldIndex, first + newIndex)
    },
  })
})
onBeforeUnmount(() => sortable?.destroy())
</script>

<template>
  <div class="flex flex-col" aria-label="キュー" role="region" data-testid="player-queue">
    <ol v-if="current" class="shrink-0" data-testid="queue-playing">
      <PlayerQueueRow :song="current" :index="state.index" :swipe="sheet" />
    </ol>
    <div class="relative min-h-0 flex-1">
      <p aria-hidden="true" class="pointer-events-none absolute inset-x-0 top-0 z-10 pt-2 pb-1 text-caption text-fg-subtle">
        {{ stuck }}
      </p>
      <!-- 上端の見出しの高さぶんは行を透かして消し、重ねた見出しを読めるようにする。下端も透かして、行が途中で切れて見えないようにする -->
      <div
        ref="scroller"
        class="relative h-full overflow-y-auto [mask-image:linear-gradient(to_bottom,transparent_26px,black_34px,black_calc(100%-16px),transparent)]"
        data-testid="queue-scroller"
      >
        <div v-if="history.length" data-testid="queue-history">
          <h3 class="pt-2 pb-1 text-caption text-fg-subtle">
            再生履歴
          </h3>
          <ol>
            <PlayerQueueRow v-for="row in history" :key="row.key" :song="row.song" :index="row.index" played :swipe="sheet" />
          </ol>
        </div>
        <!-- 次に再生する曲が少なくても「次に再生」を上まで送れるよう、スクロールする高さいっぱいに広げる -->
        <div ref="upcomingSection" class="min-h-full" data-testid="queue-upcoming-section">
          <h3 class="pt-2 pb-1 text-caption text-fg-subtle">
            次に再生
          </h3>
          <ol ref="list" data-testid="queue-upcoming">
            <PlayerQueueRow v-for="row in upcoming" :key="row.key" :song="row.song" :index="row.index" sortable :swipe="sheet" />
          </ol>
          <p v-if="!upcoming.length" class="py-2 text-body-sm text-fg-muted">
            次に再生する曲はありません
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
