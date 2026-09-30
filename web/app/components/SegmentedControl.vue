<script setup lang="ts" generic="T extends string">
import type { Component } from 'vue'

/** Design の SegmentedControl。選択肢から一つを選ぶ。 */
/** `iconOnly` なら名前は出さず、読み上げとツールチップにだけ使う。 */
defineProps<{
  label: string
  options: { value: T, label: string, icon?: Component, disabled?: boolean }[]
  iconOnly?: boolean
}>()

const model = defineModel<T>({ required: true })
</script>

<template>
  <div role="radiogroup" :aria-label="label" class="inline-flex w-fit gap-0.5 rounded-md border border-border-subtle bg-surface-1 p-0.5">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      role="radio"
      :aria-checked="model === option.value"
      :disabled="option.disabled"
      :aria-label="iconOnly ? option.label : undefined"
      :title="iconOnly ? option.label : undefined"
      class="inline-flex h-7 items-center gap-1.5 rounded-sm border border-transparent text-body-sm font-medium text-fg-subtle transition-all hover:text-fg-muted disabled:pointer-events-none disabled:opacity-40 aria-checked:border-border-strong aria-checked:bg-accent-soft aria-checked:text-fg aria-checked:shadow-[0_0_8px_var(--glow-low)]"
      :class="iconOnly ? 'w-8 justify-center' : 'px-3'"
      @click="model = option.value"
    >
      <component :is="option.icon" v-if="option.icon" class="size-3.5" />
      <template v-if="!iconOnly">
        {{ option.label }}
      </template>
    </button>
  </div>
</template>
