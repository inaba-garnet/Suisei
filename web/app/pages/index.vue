<script setup lang="ts">
import { House } from 'lucide-vue-next'

useHead({ title: 'ホーム' })
</script>

<template>
  <div>
    <PageHeader title="ホーム" />
    <EmptyState :icon="House" title="準備中です" description="最近再生した、最近追加した、よく聴く、お気に入りのアルバムをここに並べます。" />
  </div>
</template>
