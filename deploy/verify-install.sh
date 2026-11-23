#!/usr/bin/env bash
# Checks a mail node is installed the way it should stay: settings kept in the data
# volume, a known administrator state, and both surviving a restart.
# Run as root in the instance directory (the one holding update.sh and server.env):
#   sudo bash verify-install.sh
# To also prove a sign-in works, pass a login (RECOVERY_ADMIN in server.env is used
# when set):
#   sudo VERIFY_LOGIN='you@example.com:your password' bash verify-install.sh
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

pass=0
fail() { echo "FAIL $*" >&2; exit 1; }
ok() { pass=$((pass + 1)); echo "ok   $*"; }
note() { echo "note $*"; }

[[ -f server.env ]] || fail "no server.env here; run this in the instance directory"
value() { awk -F= -v k="$1" '$1 == k { sub(/^[^=]*=/, ""); print; exit }' server.env; }
login=${VERIFY_LOGIN:-$(value RECOVERY_ADMIN)}
port=$(value HTTP_HOST_PORT); port=${port:-8080}
bind=$(value HTTP_BIND); bind=${bind:-127.0.0.1}
[[ "$bind" == 0.0.0.0 ]] && bind=127.0.0.1
base="http://$bind:$port"
compose=(docker compose --env-file server.env -f compose.yaml)
source ./backend.sh
if [[ "$backend" == postgres ]] && $bundled; then
  "${compose[@]}" exec -T postgres sh -c 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "SELECT 1"' >/dev/null \
    || fail 'PostgreSQL does not answer a query'
  ok 'PostgreSQL answers a query'
fi

check_node() {
  local when=$1
  "${compose[@]}" exec -T mail test -f /var/lib/shippin.tech/config.json \
    || fail "$when: /var/lib/shippin.tech/config.json is missing, so the server is in setup mode. Run: bash update.sh \"\$(grep ^SHIPPIN_IMAGE= server.env | cut -d= -f2-)\""
  ok "$when: settings file is in the data volume"
  local pending
  pending=$(curl -s "$base/api/v1/setup" | grep -o '"pending":[a-z]*' | cut -d: -f2)
  case "$pending" in
    false) ok "$when: ownership is claimed" ;;
    true) fail "$when: installation is unclaimed; run bash claim.sh to choose the root password" ;;
    *) fail "$when: the server did not answer $base/api/v1/setup" ;;
  esac
  if [[ "$login" == *:* ]]; then
    local status
    status=$(curl -s -o /dev/null -w '%{http_code}' -u "$login" "$base/api/v1/account")
    [[ "$status" == 200 ]] || fail "$when: signing in as ${login%%:*} returned HTTP $status"
    ok "$when: ${login%%:*} signs in"
  fi
}

check_node "before restart"
"${compose[@]}" restart mail >/dev/null
for _ in $(seq 1 90); do
  [[ "$(curl -s -o /dev/null -w '%{http_code}' "$base/healthz/live")" == 200 ]] && break
  sleep 2
done
check_node "after restart"
if "${compose[@]}" logs --since 10m mail 2>&1 | grep -q 'bootstrap mode - temporary administrator'; then
  fail "the logs show the server started in setup mode with a temporary administrator"
fi
ok "no temporary setup-mode administrator in the logs"
[[ "$login" == *:* ]] || note "no login given, so sign-in was not tested; see the top of this script"
echo "Install check passed: $pass checks."
