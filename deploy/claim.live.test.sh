#!/usr/bin/env bash
# Real packaged-image check on a disposable Linux CI runner, never an existing host.
set +x
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true ]] || { echo 'Use a disposable GitHub Actions runner.' >&2; exit 2; }
image=${1:?pass the immutable candidate image}
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
work=$(mktemp -d)
instance="$work/instance"
export FIRST_RUN_URL=http://127.0.0.1:8080
export FIRST_RUN_PASSWORD='Packaged claim regression fixture password'
cleanup() {
  if [[ -f "$instance/server.env" ]]; then
    (cd "$instance" && docker compose --env-file server.env -f compose.yaml down --remove-orphans) >/dev/null 2>&1 || true
  fi
  rm -rf -- "$work"
}
trap cleanup EXIT
printf '%s\n' "$FIRST_RUN_PASSWORD" | bash "$root/install.sh" --from "$root/deploy" \
  --dir "$instance" --hostname claim.example.test --image "$image" \
  --admin-password-stdin --no-systemd > "$work/install-output" 2>&1 || {
    cat "$work/install-output"; exit 1;
  }
grep -qx SHIPPIN_STORAGE=postgres "$instance/server.env"
! grep -Fq "$FIRST_RUN_PASSWORD" "$work/install-output"
echo 'ok packaged PostgreSQL installation claims root without printing its password'
cd "$instance"
compose=(docker compose --env-file server.env -f compose.yaml)
"${compose[@]}" exec -T mail test ! -e /var/lib/shippin.tech/setup-key
python3 - <<'PY'
import base64, json, os, urllib.request
base = os.environ['FIRST_RUN_URL']
assert json.load(urllib.request.urlopen(base+'/api/v1/setup'))['pending'] is False
auth = base64.b64encode(('root@system.local:'+os.environ['FIRST_RUN_PASSWORD']).encode()).decode()
request = urllib.request.Request(base+'/api/v1/account', headers={'Authorization':'Basic '+auth})
assert urllib.request.urlopen(request).status == 200
print('ok normal root password signs in to packaged API')
PY
# No secret is needed or consumed on a repeated claim.
bash claim.sh --password-stdin </dev/null
bash verify-install.sh
"${compose[@]}" exec -T mail test ! -e /var/lib/shippin.tech/setup-key
echo 'ok ownership and retired installer authority survive restart'
cd "$root/ui"
npx playwright test -c playwright.live.config.ts first-run
