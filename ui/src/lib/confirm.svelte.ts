// 앱 안 확인창 (브라우저 confirm() 대신). App에 ConfirmHost를 한 번 두고 어디서나 ask()로 묻는다.
export type Question = { title: string; description?: string; action: string; cancel?: string; destructive?: boolean }

export const confirmState = $state<Question & { open: boolean }>({ open: false, title: '', action: '' })
let pending: ((ok: boolean) => void) | null = null

export function ask(q: Question): Promise<boolean> {
  pending?.(false)
  Object.assign(confirmState, { description: undefined, cancel: undefined, destructive: false }, q, { open: true })
  return new Promise(resolve => (pending = resolve))
}

export function answer(ok: boolean) {
  const r = pending
  pending = null
  confirmState.open = false
  r?.(ok)
}
