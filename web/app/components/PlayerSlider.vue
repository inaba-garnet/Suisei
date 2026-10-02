<script setup lang="ts">
/**
 * 再生プレイヤーのつまみ付きのバー（シークと音量）。つまんで動かしている間は `preview` を、離したら `commit` を出す。
 * 矢印キーでも `step` ずつ動かせる。引き出す再生画面を引く操作と取り合わないよう、`data-sheet-no-drag` を付ける。
 */
const props = defineProps<{ value: number, max: number, label: string, valueText?: string, step?: number }>()
const emit = defineEmits<{ preview: [value: number], commit: [value: number] }>()

const track = ref<HTMLElement>()
/** つまんで動かしている間の値 */
const dragging = ref<number>()

const shown = computed(() => dragging.value ?? props.value)
const ratio = computed(() => (props.max > 0 ? Math.min(1, Math.max(0, shown.value / props.max)) : 0))

function valueAt(clientX: number) {
  const rect = track.value!.getBoundingClientRect()
  return Math.min(1, Math.max(0, (clientX - rect.left) / rect.width)) * props.max
}

function onPointerDown(e: PointerEvent) {
  if (props.max <= 0) {
    return
  }
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  dragging.value = valueAt(e.clientX)
  emit('preview', dragging.value)
}

function onPointerMove(e: PointerEvent) {
  if (dragging.value !== undefined) {
    dragging.value = valueAt(e.clientX)
    emit('preview', dragging.value)
  }
}

function onPointerUp() {
  if (dragging.value !== undefined) {
    emit('commit', dragging.value)
    dragging.value = undefined
  }
}

function onKeyDown(e: KeyboardEvent) {
  const step = props.step ?? props.max / 20
  const next = { ArrowLeft: props.value - step, ArrowDown: props.value - step, ArrowRight: props.value + step, ArrowUp: props.value + step, Home: 0, End: props.max }[e.key]
  if (next === undefined || props.max <= 0) {
    return
  }
  e.preventDefault()
  emit('commit', Math.min(props.max, Math.max(0, next)))
}
</script>

<template>
  <div
    class="group relative flex h-4 cursor-pointer touch-none items-center"
    role="slider"
    tabindex="0"
    :aria-label="label"
    :aria-valuemin="0"
    :aria-valuemax="max"
    :aria-valuenow="Math.round(shown)"
    :aria-valuetext="valueText"
    data-sheet-no-drag
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
    @keydown="onKeyDown"
  >
    <div ref="track" class="relative h-1 w-full overflow-hidden rounded-full bg-surface-3">
      <div class="h-full bg-accent-base" :style="{ width: `${ratio * 100}%` }" />
    </div>
    <span
      class="pointer-events-none absolute size-3 -translate-x-1/2 rounded-full bg-fg opacity-0 shadow transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 [@media(hover:none)]:opacity-100"
      :class="{ 'opacity-100': dragging !== undefined }"
      :style="{ left: `${ratio * 100}%` }"
    />
  </div>
</template>
