#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="${ROOT_DIR}/.codex/state"
STATE_FILE="${STATE_DIR}/local-session.json"
mkdir -p "${STATE_DIR}"

ANON_KEY="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImJ0cXF1bWxncG9odHFqYXJ6a3VhIiwicm9sZSI6ImFub24iLCJpYXQiOjE3Njc2OTUxMDksImV4cCI6MjA4MzI3MTEwOX0.x-8QnxdKPVJoyvmAKg-N83t5AIADgQDlicDnzzGeo0I"
SUPABASE_URL="https://btqqumlgpohtqjarzkua.supabase.co"

ACTOR="lpush"
WORKDIR="${PWD}"

while [ $# -gt 0 ]; do
  case "$1" in
    --actor)
      ACTOR="${2:?actor required}"
      shift 2
      ;;
    --cwd)
      WORKDIR="${2:?cwd required}"
      shift 2
      ;;
    *)
      echo "Usage: $0 [--actor <actor>] [--cwd <workdir>]" >&2
      exit 1
      ;;
  esac
done

session_id=""
if [ -f "${STATE_FILE}" ]; then
  session_id="$(jq -r --arg actor "${ACTOR}" '.sessions[$actor].id // empty' "${STATE_FILE}" 2>/dev/null || true)"
fi

if [ -z "${session_id}" ]; then
  short_id="$(uuidgen | tr '[:upper:]' '[:lower:]' | tr -d '-' | cut -c1-12)"
  session_id="${ACTOR}.local.${short_id}"
fi

REPO="unknown"
RELATIVE="${WORKDIR#/Users/madsnissen/}"
if [[ "${RELATIVE}" =~ ^(team|ops|builds|crafts|oss)/([^/]+) ]]; then
  REPO="madshn/${BASH_REMATCH[2]}"
fi

payload="$(jq -n \
  --arg id "${session_id}" \
  --arg actor "${ACTOR}" \
  --arg repo "${REPO}" \
  --arg workdir "${WORKDIR}" \
  '{p_id: $id, p_actor: $actor, p_repo: $repo, p_workdir: $workdir}')"

curl -fsS -X POST "${SUPABASE_URL}/rest/v1/rpc/register_local_session" \
  -H "apikey: ${ANON_KEY}" \
  -H "Authorization: Bearer ${ANON_KEY}" \
  -H "Content-Type: application/json" \
  -d "${payload}" \
  >/dev/null 2>&1 || true

tmp_file="$(mktemp)"
if [ -f "${STATE_FILE}" ]; then
  jq \
    --arg actor "${ACTOR}" \
    --arg id "${session_id}" \
    --arg workdir "${WORKDIR}" \
    '.sessions = (.sessions // {}) | .sessions[$actor] = {id: $id, workdir: $workdir}' \
    "${STATE_FILE}" > "${tmp_file}"
else
  jq -n \
    --arg actor "${ACTOR}" \
    --arg id "${session_id}" \
    --arg workdir "${WORKDIR}" \
    '{sessions: {($actor): {id: $id, workdir: $workdir}}}' > "${tmp_file}"
fi
mv "${tmp_file}" "${STATE_FILE}"

printf '%s\n' "${session_id}"
