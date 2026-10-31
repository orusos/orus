#!/bin/sh
# Refuse to package the runtime EULA while it still has blanks or the
# publication checklist. ORUS_DRAFT_EULA=1 allows a development build.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
eula="$root/licenses/EULA-waid-runtime.txt"

if grep -qE '\[[A-Z ,]+\]|PUBLICATION CHECKLIST' "$eula"; then
    if [ "${ORUS_DRAFT_EULA:-}" = 1 ]; then
        echo "warning: $eula is still a draft (blanks or checklist); packaging it anyway" >&2
        exit 0
    fi
    echo "error: $eula still has blanks or the publication checklist." >&2
    echo "Complete it, or set ORUS_DRAFT_EULA=1 for a development build." >&2
    exit 1
fi
