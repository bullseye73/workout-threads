#!/usr/bin/env python3
"""
BullsNote에 코드 리뷰 노트를 저장하는 헬퍼 스크립트.
표준 입력(stdin)으로 마크다운 리뷰 본문을 받아서 BullsNote vault에 새 노트를 생성합니다.
"""

import json
import os
import random
import sys
import time

def base36(n: int) -> str:
    digits = "0123456789abcdefghijklmnopqrstuvwxyz"
    s = ""
    while n:
        n, r = divmod(n, 36)
        s = digits[r] + s
    return s or "0"

def save_note(title: str, content: str, project_name_hint: str = "AI 노트", tags: list = None) -> str:
    vault_path = os.environ.get("BULLSNOTE_DEFAULT_VAULT") or os.path.expanduser("~/Documents/BullsNote")
    if not os.path.isdir(vault_path):
        os.makedirs(vault_path, exist_ok=True)

    sidecar_file = os.path.join(vault_path, ".bullsnote.json")
    project_id = "proj-ai-notes"
    if os.path.isfile(sidecar_file):
        try:
            with open(sidecar_file, "r", encoding="utf-8") as f:
                sidecar = json.load(f)
                for p in sidecar.get("projects", []):
                    if p.get("name") == project_name_hint or p.get("id") == project_name_hint:
                        project_id = p["id"]
                        break
        except Exception:
            pass

    now_ms = int(time.time() * 1000)
    rand_part = base36(random.getrandbits(32)).rjust(7, "0")[:7]
    note_id = f"{base36(now_ms)}-{rand_part}"

    tags = tags or ["코드리뷰", "workout-app"]
    tag_suffix = " ".join(f"#{t.replace(' ', '_')}" for t in tags)

    frontmatter = [
        "---",
        f'id: "{note_id}"',
        f'title: {json.dumps(title, ensure_ascii=False)}',
        f'projectId: "{project_id}"',
        'color: "blue"',
        f'tags: {json.dumps(tags, ensure_ascii=False)}',
        f'createdAt: {now_ms}',
        f'updatedAt: {now_ms}',
        "---",
        "",
        content.strip(),
        "",
        tag_suffix,
        ""
    ]

    file_path = os.path.join(vault_path, f"{note_id}.md")
    tmp_path = f"{file_path}.{os.getpid()}.tmp"
    with open(tmp_path, "w", encoding="utf-8") as f:
        f.write("\n".join(frontmatter))
    os.replace(tmp_path, file_path)
    return file_path

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: save_to_bullsnote.py <note_title> [tag1,tag2,...]", file=sys.stderr)
        sys.exit(1)

    title = sys.argv[1]
    tags = sys.argv[2].split(",") if len(sys.argv) > 2 else ["코드리뷰", "workout-app"]
    content = sys.stdin.read()

    saved_path = save_note(title, content, "AI 노트", tags)
    print(f"📝 BullsNote에 노트가 저장되었습니다: {os.path.basename(saved_path)}")
