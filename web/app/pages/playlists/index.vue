<script setup lang="ts">
import { ListMusic } from 'lucide-vue-next'

useHead({ title: 'プレイリスト' })
</script>

<template>
  <div>
    <PageHeader title="プレイリスト" />
    <EmptyState :icon="ListMusic" title="準備中です" />
  </div>
</template>
