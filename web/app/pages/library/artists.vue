<script setup lang="ts">
import { MicVocal } from 'lucide-vue-next'

useHead({ title: 'アーティスト' })
</script>

<template>
  <div>
    <PageHeader title="アーティスト" />
    <EmptyState :icon="MicVocal" title="準備中です" />
  </div>
</template>
