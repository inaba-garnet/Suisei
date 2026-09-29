import { safeRedirect } from '~/utils/redirect'

/** ログインしていなければログインの画面に移し、戻る先を `redirect` に残す（docs/web.md）。 */
export default defineNuxtRouteMiddleware(async (to) => {
  const { user, fetchMe } = useAuth()
  if (user.value === undefined) {
    await fetchMe()
  }
  if (to.path === '/login') {
    if (user.value) {
      return navigateTo(safeRedirect(to.query.redirect))
    }
    return
  }
  if (!user.value) {
    return navigateTo({ path: '/login', query: { redirect: to.fullPath } })
  }
})
