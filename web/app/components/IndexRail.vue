<script setup lang="ts">
import { useElementBounding, useElementSize } from '@vueuse/core'

/**
 * 一覧の右端に縦に並べる頭文字の索引（docs/web.md の「一覧」）。押すかなぞると `select` で頭文字を渡す。
 * パネルの右端、見出しの帯の下に固定する。一覧が一画面を大きく超えるときだけ出す。
 * `contentHeight` は一覧の高さ。
 */
const props = defineProps<{ headings: string[], contentHeight: number }>()
const emit = defineEmits<{ select: [heading: string] }>()

/** 一文字ずつ読める高さ。これより詰まるときは間引いて点にする */
const LEGIBLE = 13
/** 一文字の高さの上限 */
const MAX = 18

const scroller = useScroller()
const { header } = usePageHeader()
const scrollerBox = useElementBounding(scroller)
const headerBox = useElementBounding(header)
const { height: viewport } = useElementSize(scroller)

const visible = computed(() => props.headings.length >= 2 && props.contentHeight > viewport.value * 2)

// スクロールバーの内側の右端に合わせる
const style = computed(() => {
  const el = scroller.value
  const right = scrollerBox.left.value + (el?.clientLeft ?? 0) + (el?.clientWidth ?? 0)
  return {
    top: `${(header.value ? headerBox.bottom.value : scrollerBox.top.value) + 8}px`,
    bottom: `${window.innerHeight - scrollerBox.bottom.value + 8}px`,
    left: `${right - 24}px`,
  }
})

const rail = ref<HTMLElement>()
const { height: railHeight } = useElementSize(rail)
const itemHeight = computed(() => Math.min(MAX, railHeight.value / Math.max(1, props.headings.length)))
// 詰まるときは `step` 個に一つだけ文字を出し、間に点を置く
const step = computed(() => Math.max(1, Math.ceil(LEGIBLE / Math.max(1, itemHeight.value))))

function mark(i: number): string {
  if (i % step.value === 0) {
    return props.headings[i]!
  }
  return i % step.value === Math.floor(step.value / 2) ? '•' : ''
}

const column = ref<HTMLElement>()
const current = ref<string>()
const pointerY = ref(0)

/** 指の高さにある頭文字。文字の列の外では端の頭文字にする。 */
function headingAt(y: number): string | undefined {
  const box = column.value?.getBoundingClientRect()
  if (!box || box.height === 0) {
    return undefined
  }
  const i = Math.floor((y - box.top) / box.height * props.headings.length)
  return props.headings[Math.min(props.headings.length - 1, Math.max(0, i))]
}

function move(event: PointerEvent) {
  const heading = headingAt(event.clientY)
  const box = rail.value?.getBoundingClientRect()
  pointerY.value = box ? Math.min(Math.max(event.clientY - box.top, 28), box.height - 28) : 0
  if (heading && heading !== current.value) {
    current.value = heading
    emit('select', heading)
  }
}

function start(event: PointerEvent) {
  event.preventDefault()
  rail.value?.setPointerCapture(event.pointerId)
  current.value = undefined
  move(event)
}

function onMove(event: PointerEvent) {
  if (current.value !== undefined) {
    move(event)
  }
}

function end() {
  current.value = undefined
}
</script>

<template>
  <nav
    v-if="visible"
    ref="rail"
    aria-label="頭文字の索引"
    class="fixed z-20 flex w-6 touch-none flex-col items-center justify-center select-none"
    :style="style"
    data-testid="index-rail"
    @pointerdown="start"
    @pointermove="onMove"
    @pointerup="end"
    @pointercancel="end"
    @lostpointercapture="end"
  >
    <div ref="column" class="flex flex-col items-center">
      <button
        v-for="(heading, i) in headings"
        :key="heading"
        type="button"
        :aria-label="heading"
        class="flex w-6 items-center justify-center text-[10px] leading-none font-semibold text-accent-base"
        :style="{ height: `${itemHeight}px` }"
        @click="emit('select', heading)"
      >
        <span aria-hidden="true">{{ mark(i) }}</span>
      </button>
    </div>
    <div
      v-if="current"
      aria-hidden="true"
      class="pointer-events-none absolute right-10 flex size-14 -translate-y-1/2 items-center justify-center rounded-xl bg-surface-3 text-h1 font-semibold text-fg shadow-elevation-3"
      :style="{ top: `${pointerY}px` }"
      data-testid="index-rail-bubble"
    >
      {{ current }}
    </div>
  </nav>
</template>
