#!/usr/bin/env bash
# Exercises backup.sh against a fake docker: a backup holds the database snapshot, the
# objects, the settings file and server.env, and never the key-wrapping key (KEYS.md),
# which `backup.sh key` prints on request so it can be kept apart.
# Run: bash deploy/backup.test.sh
set -euo pipefail
src=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
cp "$src/backup.sh" "$src/backend.sh" "$work/"
cd "$work"
KEY='d3JhcHBpbmcta2V5LWZvci10aGUtdGVzdC1vbmx5ISE='

mkdir -p bin state
# Fake docker. Every call is logged in $FAKE/calls. The snapshot step makes the folder a
# real fdbbackup would write into.
cat > bin/docker <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$FAKE/calls"
all="$*"
if [[ "$all" == *'stop mail'* ]]; then touch "$FAKE/mailstopped"; exit 0; fi
if [[ "$all" == *'up -d --wait'* ]]; then rm -f "$FAKE/mailstopped"; exit 0; fi
if [[ "$all" == *'exec -T mail '* && -f "$FAKE/mailstopped" ]]; then exit 1; fi
if [[ "$all" == *'ps --status running -q mail'* ]]; then
  [[ -f "$FAKE/mailstopped" ]] || echo test-mail
  exit 0
fi
if [[ "$all" == *'pg_dump '* ]]; then
  [[ ! -f "$FAKE/dumpfail" ]] || exit 1
  for target in backups/*.partial; do mkdir -p "$target/objects"; done
  echo synthetic-postgres-archive
  exit 0
fi
if [[ "$all" == *'pg_restore '* ]]; then
  cat >/dev/null
  if [[ "$all" == *'--list'* ]]; then [[ ! -f "$FAKE/invalidarchive" ]]; exit; fi
  [[ ! -f "$FAKE/restorefail" ]]; exit
fi
if [[ "$all" == *"fdbbackup start"* ]]; then
  dir=$(sed -E 's#.*file:///backups/([^/ ]+)/fdb.*#\1#' <<< "$all")
  mkdir -p "backups/$dir/fdb" "backups/$dir/objects"
  echo snapshot > "backups/$dir/fdb/snapshot"
  exit 0
fi
if [[ "$all" == *"cat /var/lib/shippin.tech/config.json"* ]]; then
  if grep -q SHIPPIN_STORAGE=postgres server.env; then echo '{"@type":"PostgreSql"}'; else echo '{"@type":"FoundationDb"}'; fi
  exit 0
fi
if [[ "$all" == *"cat /var/lib/shippin.tech/key-wrapping.key"* ]]; then cat "$FAKE/key"; exit 0; fi
exit 0
EOF
chmod +x bin/docker
printf '#!/usr/bin/env bash\nexit 0\n' > bin/flock
chmod +x bin/flock
export PATH="$work/bin:$PATH" FAKE="$work/state"
echo "$KEY" > state/key
printf 'COMPOSE_PROJECT_NAME=node-a\nCOMPOSE_PROFILES=bundled\nS3_ACCESS_KEY=a\nS3_SECRET_KEY=b\n' > server.env
touch compose.yaml
mkdir -p secrets/relay
printf 'super-secret-value\n' > secrets/relay/relay.key

expect() { if [[ "$2" == *"$1"* ]]; then echo "ok   $3"; else echo "FAIL $3"; echo "---"; echo "$2"; echo "---"; exit 1; fi; }
refute() { if [[ "$2" != *"$1"* ]]; then echo "ok   $3"; else echo "FAIL $3"; echo "---"; echo "$2"; echo "---"; exit 1; fi; }

out=$(bash backup.sh 2>&1)
expect "Backup written to backups/" "$out" "a backup is written"
expect "Not in it, on purpose: the key-wrapping key" "$out" "the backup says the key-wrapping key is left out"
made=$(ls -d backups/*Z)
expect "FoundationDb" "$(cat "$made/config.json")" "the settings file is in the backup"
[[ -f "$made/server.env" ]] && echo "ok   server.env is in the backup" || { echo "FAIL server.env is in the backup"; exit 1; }
refute "$KEY" "$(cat -- $(find "$made" -type f))" "no file in the backup holds the key-wrapping key"
refute "super-secret-value" "$(cat -- $(find "$made" -type f))" "the smarthost key is not in the backup"
refute "key-wrapping.key" "$(cat state/calls)" "a backup never reads the key-wrapping key"

expect "$KEY" "$(bash backup.sh key 2>&1)" "backup.sh key prints the key-wrapping key on request"
expect "backup.sh key" "$(bash backup.sh help 2>&1 || true)" "the usage names the key command"
echo "backup.sh: all checks passed"

rm -rf backups
mkdir backups
printf 'SHIPPIN_STORAGE=postgres\nPOSTGRES_PASSWORD=synthetic-test\n' >> server.env
: > state/calls
out=$(bash backup.sh 2>&1)
made=$(ls -d backups/*Z)
[[ -s "$made/postgres.dump" && "$(cat "$made/backend")" == postgres ]] || { echo 'FAIL PostgreSQL archive'; exit 1; }
expect 'stop mail' "$(cat state/calls)" 'mail quiesced for snapshot'
expect 'up -d --wait' "$(cat state/calls)" 'mail resumed after snapshot'
refute fdbbackup "$(cat state/calls)" 'PostgreSQL does not call FoundationDB'
bash backup.sh restore "$made" --yes >/dev/null
expect 'pg_restore' "$(cat state/calls)" 'PostgreSQL restore executed'
expect 'single-transaction' "$(cat state/calls)" 'restore is transactional'
touch state/invalidarchive
: > state/calls
if bash backup.sh restore "$made" --yes >/dev/null 2>&1; then echo 'FAIL invalid archive accepted'; exit 1; fi
refute 'stop mail' "$(cat state/calls)" 'invalid archive refusal precedes stopping mail'
rm state/invalidarchive
touch state/restorefail
: > state/calls
if bash backup.sh restore "$made" --yes >/dev/null 2>&1; then echo 'FAIL failed restore accepted'; exit 1; fi
refute 'up -d --wait' "$(cat state/calls)" 'failed restore leaves mail stopped'
[[ -f state/mailstopped ]]
rm state/restorefail state/mailstopped
echo foundationdb > "$made/backend"
: > state/calls
if bash backup.sh restore "$made" --yes >/dev/null 2>&1; then echo 'FAIL cross-backend restore accepted'; exit 1; fi
refute 'stop mail' "$(cat state/calls)" 'cross-backend refusal precedes stopping mail'
rm -rf backups
mkdir backups
touch state/dumpfail
: > state/calls
if bash backup.sh >/dev/null 2>&1; then echo 'FAIL failed dump reported success'; exit 1; fi
expect 'up -d --wait' "$(cat state/calls)" 'mail resumes after failed backup'
[[ -z "$(find backups -maxdepth 1 -name '*Z' -print)" ]] || { echo 'FAIL partial backup published'; exit 1; }
rm -rf backups
mkdir backups
rm state/dumpfail
touch state/mailstopped
: > state/calls
bash backup.sh >/dev/null
refute 'up -d --wait' "$(cat state/calls)" 'backup preserves an intentionally stopped node'
echo 'PostgreSQL backup and refusal checks passed'
