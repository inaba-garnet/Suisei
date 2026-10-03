<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { useVModel } from '@vueuse/core'
import { cn } from '@/lib/utils'

const props = defineProps<{
  defaultValue?: string | number
  modelValue?: string | number
  class?: HTMLAttributes['class']
}>()

const emits = defineEmits<{
  (e: 'update:modelValue', payload: string | number): void
}>()

const modelValue = useVModel(props, 'modelValue', emits, {
  passive: true,
  defaultValue: props.defaultValue,
})
</script>

<template>
  <!-- 見た目は Design の Input に合わせる（docs/web.md）。iPhone の Safari は 16px 未満の入力欄に触れると拡大するので、スマホでは 16px にする -->
  <input
    v-model="modelValue"
    data-slot="input"
    :class="cn(
      'h-10 w-full min-w-0 rounded-md border border-border-subtle bg-surface-1 px-3 text-[length:var(--text-h3)] text-fg md:text-body-lg transition-all outline-none placeholder:text-fg-subtle hover:border-border-default focus-visible:border-glow-strong focus-visible:shadow-glow-soft disabled:pointer-events-none disabled:opacity-40 aria-invalid:border-danger/60',
      props.class,
    )"
  >
</template>
