#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../../" && pwd)"
STATE_DIR="${ROOT_DIR}/.codex/state"
CONV_FILE="${STATE_DIR}/walkietalkie-conversations.json"
mkdir -p "${STATE_DIR}"

ANON_KEY="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImJ0cXF1bWxncG9odHFqYXJ6a3VhIiwicm9sZSI6ImFub24iLCJpYXQiOjE3Njc2OTUxMDksImV4cCI6MjA4MzI3MTEwOX0.x-8QnxdKPVJoyvmAKg-N83t5AIADgQDlicDnzzGeo0I"
SUPABASE_URL="https://btqqumlgpohtqjarzkua.supabase.co"
BASE_URL="${SUPABASE_URL}/rest/v1"

CALLER="lpush"
MESSAGE=""
FOLLOW_UP=0

while [ $# -gt 0 ]; do
  case "$1" in
    --caller)
      CALLER="${2:?caller required}"
      shift 2
      ;;
    --follow-up)
      FOLLOW_UP=1
      shift
      ;;
    --message)
      MESSAGE="${2:?message required}"
      shift 2
      ;;
    *)
      echo "Usage: $0 [--caller <caller>] [--follow-up] --message <message>" >&2
      exit 1
      ;;
  esac
done

[ -n "${MESSAGE}" ] || { echo "message is required" >&2; exit 1; }

auth_headers=(
  -H "apikey: ${ANON_KEY}"
  -H "Authorization: Bearer ${ANON_KEY}"
  -H "Accept-Profile: rightaim"
)

urlencode() {
  jq -nr --arg value "$1" '$value|@uri'
}

json_array_from_lines() {
  jq -Rsc 'split("\n") | map(select(length > 0))'
}

strip_markdown_code() {
  perl -0pe '
    s/```.*?```//gs;
    s/`[^`\n]*`//g;
  '
}

extract_recipients() {
  printf '%s' "${MESSAGE}" \
    | strip_markdown_code \
    | perl -0ne 'while(/@([A-Za-z0-9_-]+(?:#[A-Za-z0-9_-]+(?:#[0-9]+)?)?)/g){print "$1\n"}' \
    | awk '!seen[$0]++'
}

extract_tags() {
  printf '%s' "${MESSAGE}" \
    | strip_markdown_code \
    | perl -0ne 'while(/(?<!@)#([A-Za-z0-9_-]+)/g){print "$1\n"}' \
    | awk '!seen[$0]++'
}

repo_name=""
branch_name=""
issue_id=""
if git -C "${ROOT_DIR}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  repo_name="$(basename "$(git -C "${ROOT_DIR}" remote get-url origin 2>/dev/null)" .git 2>/dev/null || true)"
  [ -n "${repo_name}" ] || repo_name="$(basename "${ROOT_DIR}")"
  branch_name="$(git -C "${ROOT_DIR}" branch --show-current 2>/dev/null || true)"
  issue_id="$(printf '%s' "${branch_name}" | grep -oE '[0-9]+' | head -1 || true)"
fi

recipient_lines="$(extract_recipients || true)"
tag_lines="$(extract_tags || true)"

enriched_content="${MESSAGE}"
enriched_recipient_lines=""

while IFS= read -r recipient; do
  [ -n "${recipient}" ] || continue
  enriched="${recipient}"
  if [ -n "${issue_id}" ] && [ -n "${repo_name}" ] && [[ "${recipient}" != *"#"* ]]; then
    enriched="${recipient}#${repo_name}#${issue_id}"
  fi
  enriched_recipient_lines+="${enriched}"$'\n'
  if [ "${enriched}" != "${recipient}" ]; then
    escaped_name="$(printf '%s' "${recipient}" | sed 's/[][(){}.^$*+?|/]/\\&/g')"
    escaped_replacement="$(printf '%s' "@${enriched}" | sed 's/[\/&]/\\&/g')"
    enriched_content="$(printf '%s' "${enriched_content}" | perl -0pe "s/@${escaped_name}(?![#[:alnum:]_-])/${escaped_replacement}/g")"
  fi
done <<< "${recipient_lines}"

recipients_json="$(printf '%s' "${enriched_recipient_lines}" | json_array_from_lines)"
tags_json="$(printf '%s' "${tag_lines}" | json_array_from_lines)"

session_id="$("${ROOT_DIR}/.codex/register-local-session.sh" --actor "${CALLER}" --cwd "${PWD}")"

primary_target="$(printf '%s' "${enriched_recipient_lines}" | awk 'NF {print; exit}' | cut -d'#' -f1)"
conversation_id=""
if [ "${FOLLOW_UP}" -eq 1 ] && [ -n "${primary_target}" ] && [ -f "${CONV_FILE}" ]; then
  conversation_id="$(jq -r --arg target "${primary_target}" '.[$target] // empty' "${CONV_FILE}" 2>/dev/null || true)"
fi

metadata="$(jq -n \
  --arg source "codex" \
  --arg caller "${CALLER}" \
  --arg client "codex" \
  --arg workdir "${PWD}" \
  --arg repo "${repo_name:-unknown}" \
  --arg session_id "${session_id}" \
  '{source: $source, caller: $caller, client: $client, workdir: $workdir, repo: $repo, session_id: $session_id}')"

send_ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
payload="$(jq -n \
  --arg author "${CALLER}" \
  --arg content "${enriched_content}" \
  --argjson recipients "${recipients_json}" \
  --argjson tags "${tags_json}" \
  --arg conversation_id "${conversation_id}" \
  --argjson metadata "${metadata}" \
  '{
    p_author: $author,
    p_content: $content,
    p_recipients: $recipients,
    p_tags: $tags,
    p_conversation_id: (if $conversation_id == "" then null else $conversation_id end),
    p_metadata: $metadata
  }')"

result="$(curl -fsS -X POST "${BASE_URL}/rpc/post_team_message" \
  "${auth_headers[@]}" \
  -H "Content-Type: application/json" \
  -d "${payload}")"

message_id="$(printf '%s' "${result}" | jq -r 'if type == "object" then (.message_id // .id // empty) elif type == "string" then . else empty end' 2>/dev/null || true)"
server_conversation_id="$(printf '%s' "${result}" | jq -r 'if type == "object" then (.conversation_id // empty) else empty end' 2>/dev/null || true)"

if [ -z "${message_id}" ]; then
  printf 'CONV_ID=\n'
  printf 'TARGET=%s\n' "${primary_target}"
  printf 'STATUS=error\n'
  printf '%s\n' '---'
  printf '[error] Send failed: %s\n' "${result}"
  exit 0
fi

if [ -z "${server_conversation_id}" ]; then
  server_conversation_id="$(curl -fsS \
    "${BASE_URL}/team_messages?id=eq.$(urlencode "${message_id}")&select=conversation_id" \
    "${auth_headers[@]}" \
    | jq -r '.[0].conversation_id // empty')"
fi

if [ -n "${primary_target}" ] && [ -n "${server_conversation_id}" ]; then
  tmp_file="$(mktemp)"
  if [ -f "${CONV_FILE}" ]; then
    jq --arg target "${primary_target}" --arg conv "${server_conversation_id}" '.[$target] = $conv' "${CONV_FILE}" > "${tmp_file}"
  else
    jq -n --arg target "${primary_target}" --arg conv "${server_conversation_id}" '{($target): $conv}' > "${tmp_file}"
  fi
  mv "${tmp_file}" "${CONV_FILE}"
fi

if [ -z "${primary_target}" ]; then
  joined_tags="$(printf '%s' "${tag_lines}" | paste -sd, -)"
  printf 'CONV_ID=%s\n' "${server_conversation_id}"
  printf 'TARGET=\n'
  printf 'STATUS=posted\n'
  printf '%s\n' '---'
  if [ -n "${joined_tags}" ]; then
    printf 'Posted to #%s.\n' "${joined_tags//,/, #}"
  else
    printf '%s\n' 'Message posted.'
  fi
  exit 0
fi

last_status=""
for _ in $(seq 1 60); do
  delivery_json="$(curl -fsS \
    "${BASE_URL}/team_message_deliveries?message_id=eq.$(urlencode "${message_id}")&recipient=eq.$(urlencode "${primary_target}")&select=status,claimed_by,target_session_id&limit=1" \
    "${auth_headers[@]}" || echo '[]')"
  reaction_json="$(curl -fsS \
    "${BASE_URL}/team_message_reactions?message_id=eq.$(urlencode "${message_id}")&author=eq.$(urlencode "${primary_target}")&select=type,note&order=created_at.desc&limit=1" \
    "${auth_headers[@]}" || echo '[]')"
  content="$(curl -fsS \
    "${BASE_URL}/team_messages?conversation_id=eq.$(urlencode "${server_conversation_id}")&author=eq.$(urlencode "${primary_target}")&order=created_at.desc&limit=1&created_at=gt.${send_ts}" \
    "${auth_headers[@]}" \
    | jq -r '.[0].content // empty' 2>/dev/null || true)"

  if [ -n "${content}" ]; then
    printf 'CONV_ID=%s\n' "${server_conversation_id}"
    printf 'TARGET=%s\n' "${primary_target}"
    printf 'STATUS=reply\n'
    printf '%s\n' '---'
    printf '%s\n' "${content}"
    exit 0
  fi

  delivery_status="$(printf '%s' "${delivery_json}" | jq -r '.[0].status // empty' 2>/dev/null || true)"
  delivery_claimed_by="$(printf '%s' "${delivery_json}" | jq -r '.[0].claimed_by // empty' 2>/dev/null || true)"
  delivery_target="$(printf '%s' "${delivery_json}" | jq -r '.[0].target_session_id // empty' 2>/dev/null || true)"
  reaction_type="$(printf '%s' "${reaction_json}" | jq -r '.[0].type // empty' 2>/dev/null || true)"
  reaction_note="$(printf '%s' "${reaction_json}" | jq -r '.[0].note // empty' 2>/dev/null || true)"

  new_status=""
  if [ -n "${reaction_type}" ]; then
    case "${reaction_type}" in
      read)
        new_status="processing"
        ;;
      working)
        new_status="working"
        ;;
      blocked)
        if [ -n "${reaction_note}" ]; then
          new_status="blocked: ${reaction_note}"
        else
          new_status="blocked"
        fi
        ;;
      awaiting)
        new_status="awaiting input"
        ;;
      done)
        new_status="done (waiting for reply text)"
        ;;
      complete)
        new_status="complete"
        ;;
      *)
        new_status="${reaction_type}"
        ;;
    esac
  elif [ -n "${delivery_target}" ]; then
    new_status="assigned to worker"
  elif [ "${delivery_status}" = "claimed" ] && [ -n "${delivery_claimed_by}" ]; then
    new_status="accepted by router"
  elif [ "${delivery_status}" = "pending" ]; then
    new_status="pending"
  elif [ "${delivery_status}" = "resolved" ]; then
    new_status="resolved (no reply yet)"
  elif [ "${delivery_status}" = "failed" ]; then
    printf 'CONV_ID=%s\n' "${server_conversation_id}"
    printf 'TARGET=%s\n' "${primary_target}"
    printf 'STATUS=failed\n'
    printf '%s\n' '---'
    printf '[failed] Delivery to %s failed: %s\n' "${primary_target}" "${delivery_claimed_by}"
    exit 0
  fi

  if [ -n "${new_status}" ] && [ "${new_status}" != "${last_status}" ]; then
    echo "[status] ${new_status}" >&2
    last_status="${new_status}"
  fi

  sleep 3
done

printf 'CONV_ID=%s\n' "${server_conversation_id}"
printf 'TARGET=%s\n' "${primary_target}"
printf 'STATUS=timeout\n'
printf '%s\n' '---'
printf '[timeout] %s has not responded after 180s. Last status: %s.\n' "${primary_target}" "${last_status:-unknown}"
exit 0
