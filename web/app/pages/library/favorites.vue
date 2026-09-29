<script setup lang="ts">
import { Heart } from 'lucide-vue-next'

useHead({ title: 'お気に入り' })
</script>

<template>
  <div>
    <PageHeader title="お気に入り" />
    <EmptyState :icon="Heart" title="準備中です" />
  </div>
</template>
