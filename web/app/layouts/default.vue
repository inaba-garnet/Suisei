<template>
  <div class="flex h-dvh flex-col md:grid md:grid-cols-[240px_1fr] md:grid-rows-[1fr_72px]">
    <!--
      PC（md 以上）はサイドバー、内容、下端の再生バー。スマホは内容、再生バー、下のタブ（docs/web.md）。
      再生バーはページの外に一つだけ置き、画面を移っても作り直さない。
    -->
    <aside class="hidden border-r border-divider bg-surface-1 md:block">
      <AppSidebar />
    </aside>
    <main class="min-h-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-6xl px-4 pt-[calc(env(safe-area-inset-top)+16px)] pb-6 md:px-8 md:pt-6">
        <slot />
      </div>
    </main>
    <div class="shrink-0 md:col-span-2 md:border-t md:border-divider md:bg-surface-1">
      <PlayerBar />
    </div>
    <AppTabBar class="md:hidden" />
  </div>
</template>
