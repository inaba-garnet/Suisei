<script setup lang="ts">
import { MicVocal } from 'lucide-vue-next'

useHead({ title: 'アーティスト' })

// アルバムアーティストか、曲のアーティスト全員か（docs/web.md の「一覧」）。
// 「戻る」で戻ったときは選んでいた方に、開き直したときはアルバムアーティストに戻す
type Role = 'albumartist' | 'artist'
const role = useState<Role>('artists-role', () => 'albumartist')
if (!usePopped().value) {
  role.value = 'albumartist'
}
// 切り替えたときは、残した位置を使わずに先頭から出す
const popped = usePopped()
const scroller = useScroller()
watch(role, () => {
  popped.value = false
  if (scroller.value) {
    scroller.value.scrollTop = 0
  }
})

const roles: { value: Role, label: string }[] = [
  { value: 'albumartist', label: 'アルバムアーティスト' },
  { value: 'artist', label: 'アーティスト' },
]
</script>

<template>
  <div>
    <PageHeader title="アーティスト" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <SegmentedControl v-model="role" label="表示するアーティスト" :options="roles" />
    </PageHeader>

    <ArtistBrowser :key="role" :role="role" :empty-icon="MicVocal" empty-title="アーティストがいません" />
  </div>
</template>
