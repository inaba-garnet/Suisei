<script setup lang="ts">
import { Music } from 'lucide-vue-next'

useHead({ title: 'トラック' })
</script>

<template>
  <div>
    <PageHeader title="トラック" />
    <EmptyState :icon="Music" title="準備中です" />
  </div>
</template>
