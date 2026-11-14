// 스크롤하는 동안만 스크롤바를 보이게: 스크롤된 요소에 data-scrolling을 붙였다가 잠시 멈추면 뗀다.
export function installScrollReveal(root: Document = document, hideAfter = 900): () => void {
  const timers = new Map<Element, ReturnType<typeof setTimeout>>()
  const onScroll = (e: Event) => {
    const el = e.target instanceof Element ? e.target : root.documentElement
    el.setAttribute('data-scrolling', '')
    clearTimeout(timers.get(el))
    timers.set(el, setTimeout(() => { el.removeAttribute('data-scrolling'); timers.delete(el) }, hideAfter))
  }
  root.addEventListener('scroll', onScroll, { capture: true, passive: true })
  return () => {
    root.removeEventListener('scroll', onScroll, { capture: true })
    timers.forEach(t => clearTimeout(t))
    timers.clear()
  }
}
