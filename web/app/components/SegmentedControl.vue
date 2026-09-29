<script setup lang="ts" generic="T extends string">
import type { Component } from 'vue'

/** Design の SegmentedControl。選択肢から一つを選ぶ。 */
defineProps<{
  label: string
  options: { value: T, label: string, icon?: Component }[]
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
      class="inline-flex h-8 items-center gap-1.5 rounded-sm border border-transparent px-3 text-body text-fg-muted transition-all hover:text-fg aria-checked:border-border-strong aria-checked:bg-accent-soft aria-checked:text-fg"
      @click="model = option.value"
    >
      <component :is="option.icon" v-if="option.icon" class="size-3.5" />
      {{ option.label }}
    </button>
  </div>
</template>
