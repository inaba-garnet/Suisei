<script setup lang="ts">
import { Search } from 'lucide-vue-next'

useHead({ title: '検索' })
</script>

<template>
  <div>
    <PageHeader title="検索" />
    <EmptyState :icon="Search" title="準備中です" description="アルバム、アーティスト、トラックをまとめて探せるようにします。" />
  </div>
</template>
