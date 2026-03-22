#!/bin/bash
# CCM - Launch Claude Code in project (Alfred Run Script)
# Receives project ID as argument, calls CCM API to launch

PROJECT_ID="${1}"
API_TOKEN="${CCM_API_TOKEN:-}"
API_PORT="${CCM_API_PORT:-23890}"
BASE_URL="http://127.0.0.1:${API_PORT}"

if [ -z "$API_TOKEN" ]; then
  echo "CCM_API_TOKEN not configured"
  exit 1
fi

if [ -z "$PROJECT_ID" ]; then
  echo "No project ID provided"
  exit 1
fi

RESPONSE=$(curl -s --connect-timeout 2 \
  -X POST \
  -H "Authorization: Bearer ${API_TOKEN}" \
  "${BASE_URL}/api/projects/${PROJECT_ID}/launch" 2>/dev/null)

if [ $? -ne 0 ]; then
  echo "Cannot connect to CCM"
  exit 1
fi

OK=$(echo "$RESPONSE" | python3 -c "import json,sys; print(json.load(sys.stdin).get('ok',False))" 2>/dev/null)

if [ "$OK" = "True" ]; then
  echo "Launched"
else
  ERROR=$(echo "$RESPONSE" | python3 -c "import json,sys; print(json.load(sys.stdin).get('error','Unknown error'))" 2>/dev/null)
  echo "Failed: $ERROR"
  exit 1
fi
