#!/bin/sh
# 컴포넌트에 색을 직접 쓰지 않았는지 검사한다 (색은 src/app.css 토큰만).
cd "$(dirname "$0")/.." || exit 1
DIRS="src/lib/components src/components/workbench src/components/detail"
found=$(for d in $DIRS; do [ -d "$d" ] && grep -rnE '#[0-9a-fA-F]{3,8}\b|\bwhite\b|\bblack\b' --include=*.svelte "$d" | grep -v '^src/lib/components/ui/'; done)
if [ -n "$found" ]; then
  echo "직접 쓴 색이 있어요. app.css 토큰을 쓰세요:"; echo "$found"; exit 1
fi
echo "색 검사 통과"
