export default defineNuxtPlugin(() => {
  const { apply, media } = useTheme()
  apply()
  media?.addEventListener('change', apply)
})
