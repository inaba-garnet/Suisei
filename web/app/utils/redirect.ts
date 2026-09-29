/**
 * ログインの後に戻る先。同じサイトのパスに限り、ほかは `/` にする。
 * `//example.com` や `https://...` を受けると、ログインの画面から別のサイトへ飛ばせてしまうため。
 */
export function safeRedirect(value: unknown): string {
  if (typeof value !== 'string' || !value.startsWith('/') || value.startsWith('//') || value.startsWith('/\\')) {
    return '/'
  }
  if (value === '/login' || value.startsWith('/login?') || value.startsWith('/login#')) {
    return '/'
  }
  return value
}
