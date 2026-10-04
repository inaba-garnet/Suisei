<script setup lang="ts">
import { useElementBounding, useScroll } from '@vueuse/core'
import { ChevronLeft } from 'lucide-vue-next'
import { coverArtUrl } from '~/utils/subsonic'

/**
 * 画面の上端に残る見出しの帯（docs/web.md の「画面構成」）。スクロールしても今どの画面かが分かるようにする。
 * スマホでは画面の端まで広げ、端まで広がる横スクロール（AlbumShelf）が帯の横から覗かないようにする。
 * `back` があれば戻る矢印を出す。`label` のない `back` は前にいた画面に戻り、前の画面がなければ `to` を開く。
 * `detail` の画面は大きな見出しを自分で持ち、帯の名前は `titleVisible` のときだけ出す。
 * `overlay` なら、スクロールするまで背景を透かし、画面の背景（ヒーロー）を見せる。
 * `cover` があれば、帯を単色ではなくジャケットをぼかした背景で塗り、濃さを `bandOpacity`（0〜1）に合わせる。
 * 塗りはパネルの幅いっぱいに広げ、内容の幅で止めない。背景のヒーローがパネルの幅いっぱいに敷かれ、帯の横から覗くため。
 */
const props = defineProps<{
  title: string
  description?: string
  back?: { to: string, label?: string, mobileOnly?: boolean }
  detail?: boolean
  titleVisible?: boolean
  overlay?: boolean
  cover?: string
  bandOpacity?: number
}>()

const router = useRouter()
const scroller = useScroller()
const { y } = useScroll(scroller)
const scrolled = computed(() => y.value > 0)
const band = computed(() => props.overlay && !!props.cover)

// 塗りをパネルの左端から右端まで広げる。スクロールバーの幅は除く
const header = ref<HTMLElement>()
const headerBox = useElementBounding(header)
const scrollerBox = useElementBounding(scroller)
const bandStyle = computed(() => {
  const el = scroller.value
  return {
    left: `${scrollerBox.left.value + (el?.clientLeft ?? 0) - headerBox.left.value}px`,
    width: `${el?.clientWidth ?? 0}px`,
    opacity: props.bandOpacity ?? 0,
  }
})

// 戻る先（`label` がなければアプリの中の前の画面）から来たなら「戻る」で戻り、一覧の位置を戻す。そうでなければ戻る先を開く
function goBack() {
  if (!props.back) {
    return
  }
  const previous = window.history.state?.back
  if (props.back.label ? previous === props.back.to : previous) {
    router.back()
  }
  else {
    navigateTo(props.back.to)
  }
}
</script>

<template>
  <header
    ref="header"
    class="sticky top-0 z-10 -mx-4 mb-4 border-b px-4 pt-[calc(env(safe-area-inset-top)+8px)] pb-3 transition-colors md:mx-0 md:px-0 md:pt-5"
    :class="band
      ? 'border-transparent bg-transparent'
      : [scrolled ? 'border-divider' : 'border-transparent', overlay && !scrolled ? 'bg-transparent' : 'bg-surface-0 md:bg-surface-1']"
    data-testid="page-header"
  >
    <div v-if="band" aria-hidden="true" class="pointer-events-none absolute inset-y-0 -z-10 overflow-hidden" :style="bandStyle" data-testid="page-header-band">
      <img :src="coverArtUrl(cover!, 300)" alt="" class="absolute top-1/2 left-0 w-full -translate-y-1/2 scale-125 blur-xl">
      <div class="absolute inset-0 bg-surface-0/60 md:bg-surface-1/70" />
    </div>
    <div class="flex min-h-11 items-center gap-1">
      <button
        v-if="back"
        type="button"
        :aria-label="back.label ? `${back.label}に戻る` : '戻る'"
        class="-ml-2.5 flex size-11 shrink-0 items-center justify-center rounded-full text-fg transition-colors hover:bg-surface-2"
        :class="{ 'md:hidden': back.mobileOnly }"
        @click="goBack"
      >
        <ChevronLeft class="size-6" />
      </button>
      <span
        v-if="detail"
        aria-hidden="true"
        class="min-w-0 flex-1 truncate text-body-lg font-semibold transition-opacity"
        :class="titleVisible ? 'opacity-100' : 'opacity-0'"
      >{{ title }}</span>
      <div v-else class="flex min-w-0 items-baseline gap-2.5">
        <h1 class="truncate text-h1 font-semibold">
          {{ title }}
        </h1>
        <span v-if="description" class="text-body-sm text-fg-subtle">{{ description }}</span>
      </div>
    </div>
    <div v-if="$slots.default" class="mt-3">
      <slot />
    </div>
  </header>
</template>
