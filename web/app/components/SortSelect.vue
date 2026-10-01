<script setup lang="ts" generic="T extends string">
/** 並び順の選択。見出しの帯の下に置く。 */
defineProps<{ options: { value: T, label: string, disabled?: boolean }[] }>()
const model = defineModel<T>({ required: true })
</script>

<template>
  <select
    v-model="model"
    aria-label="並び順"
    class="h-8 rounded-md border border-border-subtle bg-surface-1 px-2 text-body-sm text-fg-muted"
  >
    <option v-for="option in options" :key="option.value" :value="option.value" :disabled="option.disabled">
      {{ option.label }}
    </option>
  </select>
</template>
