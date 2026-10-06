<script setup lang="ts">
/** 設定の画面の、名前と候補から選ぶ欄の一行。 */
defineProps<{ label: string, description?: string, options: { value: number, label: string }[], disabled?: boolean }>()
const model = defineModel<number>({ required: true })
const id = useId()
</script>

<template>
  <div class="flex items-center gap-3 border-b border-divider px-3 py-3">
    <div class="min-w-0 flex-1">
      <label :for="id" class="block text-body">{{ label }}</label>
      <p v-if="description" class="text-body-sm text-fg-subtle">
        {{ description }}
      </p>
    </div>
    <select
      :id="id"
      v-model="model"
      :disabled="disabled"
      class="h-8 rounded-md border border-border-subtle bg-surface-1 px-2 text-body-sm text-fg disabled:opacity-40"
    >
      <option v-for="option in options" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
  </div>
</template>
