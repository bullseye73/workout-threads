// 게시 글 편집 상태: 화면 글, 저장된 글과의 차이, 복사할 전체 글.
// ActivityPane이 만들어 PostTab에 넘기므로, 탭을 벗어나도 저장 안 한 수정 여부를 그대로 읽을 수 있다.
import { mergeEditor } from './enrich'
import { parseHashtags } from './format'

type Saved = { body: string; hashtags: string[] }

export function createPostEditor() {
  let saved = $state<Saved | null>(null)
  let body = $state('')
  let tags = $state('')
  const hashtags = $derived(parseHashtags(tags))
  const dirty = $derived(!!saved && (body !== saved.body || hashtags.join(' ') !== saved.hashtags.join(' ')))
  const fullText = $derived(hashtags.length ? `${body}\n\n${hashtags.join(' ')}` : body)

  return {
    get body() { return body },
    set body(v: string) { body = v },
    get tags() { return tags },
    set tags(v: string) { tags = v },
    get hashtags() { return hashtags },
    get dirty() { return dirty },
    get fullText() { return fullText },
    get loaded() { return saved !== null },
    /** force: 사용자가 일부러 글을 바꾼 경우(다시 쓰기·적용)라 화면 글도 서버 값으로 바꾼다. */
    load(p: Saved, force = false) {
      const merged = mergeEditor(p, { body, tags }, dirty && !force)
      saved = { body: p.body, hashtags: [...p.hashtags] }
      body = merged.body
      tags = merged.tags
    },
    /** 저장 안 한 수정을 버리고 저장된 글로 되돌린다. */
    discard() {
      if (!saved) return
      body = saved.body
      tags = saved.hashtags.join(' ')
    },
  }
}

export type PostEditor = ReturnType<typeof createPostEditor>
