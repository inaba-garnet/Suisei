/** 画面の移動がブラウザの「戻る」「進む」だったかを記録する（docs/web.md の「一覧」）。 */
export default defineNuxtPlugin(() => {
  const router = useRouter()
  const popped = usePopped()
  let pending = false
  // 呼ばれるのは「戻る」「進む」のときだけで、そのあとに afterEach が続く
  router.options.history.listen(() => {
    pending = true
  })
  router.afterEach(() => {
    popped.value = pending
    pending = false
  })
})
