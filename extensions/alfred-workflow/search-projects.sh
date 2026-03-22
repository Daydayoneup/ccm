#!/bin/bash
# CCM - Search Projects (Alfred Script Filter)
# Searches CCM projects and returns Alfred-compatible JSON

QUERY="${1:-}"
API_TOKEN="${CCM_API_TOKEN:-}"
API_PORT="${CCM_API_PORT:-23890}"
BASE_URL="http://127.0.0.1:${API_PORT}"

# No token configured
if [ -z "$API_TOKEN" ]; then
  cat <<'EOF'
{"items":[{"title":"CCM API Token not configured","subtitle":"Set CCM_API_TOKEN in Workflow Environment Variables","valid":false,"icon":{"path":"icon.png"}}]}
EOF
  exit 0
fi

# Build URL
URL="${BASE_URL}/api/projects"
if [ -n "$QUERY" ]; then
  ENCODED=$(python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))" "$QUERY")
  URL="${URL}?q=${ENCODED}"
fi

# Call API
RESPONSE=$(curl -s --connect-timeout 2 -H "Authorization: Bearer ${API_TOKEN}" "$URL" 2>/dev/null)

# Connection failed
if [ $? -ne 0 ] || [ -z "$RESPONSE" ]; then
  cat <<'EOF'
{"items":[{"title":"Cannot connect to CCM","subtitle":"Make sure CCM is running with HTTP API enabled","valid":false,"icon":{"path":"icon.png"}}]}
EOF
  exit 0
fi

# Parse and format for Alfred
python3 <<PYEOF
import json, sys

try:
    data = json.loads('''$RESPONSE''')
except:
    print(json.dumps({"items": [{"title": "Failed to parse CCM response", "valid": False}]}))
    sys.exit(0)

if not data.get("ok"):
    error = data.get("error", "Unknown error")
    print(json.dumps({"items": [{"title": f"CCM Error: {error}", "valid": False}]}))
    sys.exit(0)

projects = data.get("data", [])

if not projects:
    result = {"items": [{"title": "No projects found", "subtitle": "Try a different search term", "valid": False}]}
    print(json.dumps(result))
    sys.exit(0)

items = []
for p in projects:
    pinned = p.get("pinned", False)
    lang = p.get("language") or ""
    launches = p.get("launch_count", 0)

    subtitle_parts = [p["path"]]
    if lang:
        subtitle_parts.append(f"[{lang}]")
    subtitle_parts.append(f"{launches} launches")
    subtitle = "  |  ".join(subtitle_parts)

    item = {
        "uid": p["id"],
        "title": ("📌 " if pinned else "") + p["name"],
        "subtitle": subtitle,
        "arg": p["id"],
        "autocomplete": p["name"],
        "mods": {
            "cmd": {
                "arg": p["path"],
                "subtitle": "Copy project path"
            },
            "alt": {
                "arg": p["path"],
                "subtitle": f"Open in Finder: {p['path']}"
            }
        },
        "text": {
            "copy": p["path"],
            "largetype": p["name"]
        }
    }
    items.append(item)

print(json.dumps({"items": items}))
PYEOF
