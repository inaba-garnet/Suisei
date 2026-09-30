<script setup lang="ts">
import { useScroll } from '@vueuse/core'
import { ChevronLeft } from 'lucide-vue-next'

/**
 * 画面の上端に残る見出しの帯（docs/web.md の「画面構成」）。スクロールしても今どの画面かが分かるようにする。
 * `back` があれば戻る矢印を出す。`detail` の画面は大きな見出しを自分で持ち、帯の名前は `titleVisible` のときだけ出す。
 */
const props = defineProps<{
  title: string
  description?: string
  back?: { to: string, label: string, mobileOnly?: boolean }
  detail?: boolean
  titleVisible?: boolean
}>()

const router = useRouter()
const scroller = useScroller()
const { y } = useScroll(scroller)
const scrolled = computed(() => y.value > 0)

// 戻る先から来たなら「戻る」で戻り、一覧の位置を戻す。そうでなければ戻る先を開く
function goBack() {
  if (!props.back) {
    return
  }
  if (window.history.state?.back === props.back.to) {
    router.back()
  }
  else {
    navigateTo(props.back.to)
  }
}
</script>

<template>
  <header
    class="sticky top-0 z-10 mb-4 border-b bg-surface-0 pt-[calc(env(safe-area-inset-top)+8px)] pb-3 transition-colors md:bg-surface-1 md:pt-5"
    :class="scrolled ? 'border-divider' : 'border-transparent'"
    data-testid="page-header"
  >
    <div class="flex min-h-11 items-center gap-1">
      <button
        v-if="back"
        type="button"
        :aria-label="`${back.label}に戻る`"
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
