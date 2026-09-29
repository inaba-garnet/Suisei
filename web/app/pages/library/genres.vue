<script setup lang="ts">
import { Shapes } from 'lucide-vue-next'

useHead({ title: 'ジャンル' })
</script>

<template>
  <div>
    <PageHeader title="ジャンル" />
    <EmptyState :icon="Shapes" title="準備中です" />
  </div>
</template>
