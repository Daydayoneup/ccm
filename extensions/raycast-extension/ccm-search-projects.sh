#!/bin/bash

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title Search Projects
# @raycast.mode fullOutput
# @raycast.packageName CCM

# Optional parameters:
# @raycast.icon 🔍
# @raycast.argument1 { "type": "text", "placeholder": "Search keyword (optional)", "optional": true }

# Documentation:
# @raycast.description Search CCM projects and list them
# @raycast.author admin

API_TOKEN="${CCM_API_TOKEN:-}"
API_PORT="${CCM_API_PORT:-23890}"
BASE_URL="http://127.0.0.1:${API_PORT}"

if [ -z "$API_TOKEN" ]; then
  echo "Error: CCM_API_TOKEN not set."
  echo ""
  echo "Set it in Raycast Script Command settings or export it:"
  echo "  export CCM_API_TOKEN=your-token-here"
  exit 1
fi

QUERY="${1:-}"
URL="${BASE_URL}/api/projects"
if [ -n "$QUERY" ]; then
  ENCODED=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$QUERY'))")
  URL="${URL}?q=${ENCODED}"
fi

RESPONSE=$(curl -s -w "\n%{http_code}" -H "Authorization: Bearer ${API_TOKEN}" "$URL" 2>/dev/null)
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" != "200" ]; then
  echo "Error: CCM API returned HTTP ${HTTP_CODE}"
  echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "$BODY"
  exit 1
fi

echo "$BODY" | python3 -c "
import json, sys
data = json.load(sys.stdin)
if not data.get('ok'):
    print('Error:', data.get('error', 'Unknown'))
    sys.exit(1)
projects = data.get('data', [])
if not projects:
    print('No projects found.')
    sys.exit(0)
for p in projects:
    pin = '📌 ' if p.get('pinned') else ''
    lang = f\" [{p['language']}]\" if p.get('language') else ''
    launches = p.get('launch_count', 0)
    print(f\"{pin}{p['name']}{lang} — {p['path']}  ({launches} launches)\")
print(f\"\n{len(projects)} project(s) found.\")
"
