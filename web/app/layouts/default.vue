<script setup lang="ts">
const { align } = useContentAlign()
const dev = useDev()
// 内容の配置は開発モードだけの一時的な項目なので、それ以外では既定の中央にする
const centered = computed(() => !dev.value || align.value === 'center')

const scroller = ref<HTMLElement>()
provide(scrollerKey, scroller)

// 内容をスクロールするのはパネルなので、画面を移ったら先頭に戻す。「戻る」のときは一覧が位置を戻す
const route = useRoute()
const popped = usePopped()
watch(() => route.fullPath, () => {
  if (!popped.value && scroller.value) {
    scroller.value.scrollTop = 0
  }
}, { flush: 'post' })
</script>

<template>
  <div class="flex h-dvh flex-col md:grid md:grid-cols-[240px_1fr] md:grid-rows-[1fr_72px] 2xl:grid-cols-[240px_1fr_320px] 2xl:grid-rows-[1fr] 3xl:grid-cols-[240px_1fr_380px]">
    <!--
      PC（md 以上）はサイドバー、内容、下端の再生バー。スマホは内容、再生バー、下のタブ（docs/web.md）。
      2xl 以上は下端の再生バーの代わりに、右の欄に再生プレイヤーを置く。
      再生バーはページの外に一つだけ置き、画面を移っても作り直さない。
    -->
    <aside class="hidden border-r border-divider bg-surface-1 md:block">
      <AppSidebar />
    </aside>
    <!--
      PC は内容を一枚のパネルに収め、中の一覧は枠を持たない行にする（docs/web.md）。
      スマホはパネルを画面いっぱいに広げ、角丸と外側の余白を付けない。上の余白は各画面の見出しの帯（PageHeader）が持つ。
    -->
    <main class="min-h-0 flex-1 md:py-3 md:pr-5 md:pl-5">
      <div ref="scroller" data-testid="scroller" class="relative h-full overflow-y-auto px-4 pb-6 md:rounded-xl md:border md:border-divider md:bg-surface-1 md:px-5 md:pb-5">
        <div class="max-w-[640px] xl:max-w-[1040px]" :class="{ 'mx-auto': centered }" data-testid="content">
          <slot />
        </div>
      </div>
    </main>
    <aside class="hidden min-h-0 py-3 pr-5 2xl:block">
      <PlayerPanel />
    </aside>
    <div class="shrink-0 md:col-span-2 md:border-t md:border-divider md:bg-surface-1 md:px-4 md:py-2.5 2xl:hidden">
      <PlayerBar />
    </div>
    <AppTabBar class="md:hidden" />
  </div>
</template>
