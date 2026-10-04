<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
import { useEventListener, useResizeObserver } from '@vueuse/core'
import { ChevronLeft, ChevronRight } from 'lucide-vue-next'

/**
 * 見出しと横スクロールの棚（docs/web.md の「デザイン」）。棚のスクロールバーは隠し、PC は見出しの右の「‹」「›」で一画面分ずつ送る。
 * スマホは指で送るので矢印を出さない。`count` が 0 なら棚の代わりに `fallback` を出す。
 */
const props = defineProps<{ title: string, count: number, more?: RouteLocationRaw, testid?: string }>()

const list = ref<HTMLElement>()
const canPrevious = ref(false)
const canNext = ref(false)

function update() {
  const el = list.value
  if (!el) {
    canPrevious.value = canNext.value = false
    return
  }
  canPrevious.value = el.scrollLeft > 1
  canNext.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1
}
useEventListener(list, 'scroll', update, { passive: true })
useResizeObserver(list, update)
watch(() => props.count, () => nextTick(update))

/** 一画面分送る。端のジャケットが半分見えている分は残し、続きがどこから始まるか分かるようにする。 */
function page(direction: 1 | -1) {
  const el = list.value
  el?.scrollBy({ left: direction * el.clientWidth * 0.9, behavior: 'smooth' })
}
</script>

<template>
  <section :aria-label="title">
    <div class="mb-3 flex items-center gap-1">
      <h2 class="min-w-0 flex-1 truncate text-h2 font-semibold">
        {{ title }}
      </h2>
      <template v-if="count && (canPrevious || canNext)">
        <button
          type="button"
          :aria-label="`${title}の前へ`"
          :disabled="!canPrevious"
          class="hidden size-8 items-center justify-center rounded-full text-fg-subtle transition-colors hover:bg-surface-2 hover:text-fg disabled:pointer-events-none disabled:opacity-30 md:flex"
          @click="page(-1)"
        >
          <ChevronLeft class="size-5" />
        </button>
        <button
          type="button"
          :aria-label="`${title}の次へ`"
          :disabled="!canNext"
          class="hidden size-8 items-center justify-center rounded-full text-fg-subtle transition-colors hover:bg-surface-2 hover:text-fg disabled:pointer-events-none disabled:opacity-30 md:flex"
          @click="page(1)"
        >
          <ChevronRight class="size-5" />
        </button>
      </template>
      <NuxtLink v-if="count && more" :to="more" class="ml-2 text-body-sm text-fg-subtle transition-colors hover:text-fg">
        すべて見る
      </NuxtLink>
    </div>
    <ul
      v-if="count"
      ref="list"
      class="scrollbar-none -mx-4 flex snap-x gap-4 overflow-x-auto scroll-px-4 px-4 pb-2 md:mx-0 md:scroll-px-0 md:px-0"
      :data-testid="testid"
    >
      <slot />
    </ul>
    <slot v-else name="fallback" />
  </section>
</template>
