<script setup lang="ts">
import { Disc3 } from 'lucide-vue-next'

useHead({ title: 'アルバム' })
</script>

<template>
  <div>
    <PageHeader title="アルバム" />
    <EmptyState :icon="Disc3" title="準備中です" />
  </div>
</template>
