// 첫 화면부터 저장된 테마로 (mode-watcher와 같은 저장 키). 번쩍임 방지.
// 앱 CSP가 인라인 스크립트를 막아서 파일로 둔다.
try {
  const m = localStorage.getItem('mode-watcher-mode') || 'system'
  const dark = m === 'dark' || (m === 'system' && matchMedia('(prefers-color-scheme: dark)').matches)
  document.documentElement.classList.toggle('dark', dark)
} catch { /* 저장소를 못 쓰면 밝게로 시작 */ }
