#!/usr/bin/env bash
# host.sh writes a unit for this directory and drops units whose paths are missing.
# Run: bash deploy/host.test.sh
set -euo pipefail
src=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
cp "$src/host.sh" "$src/backup.sh" "$src/backend.sh" "$src/compose.yaml" "$work/"
cd "$work"
mkdir -p bin units real secrets
printf 'COMPOSE_PROJECT_NAME=shippin-mail\nCOMPOSE_PROFILES=bundled\n' > server.env
touch compose.yaml
mkdir -p real
printf 'COMPOSE_PROJECT_NAME=shippin-mail\nCOMPOSE_PROFILES=bundled\n' > real/server.env
cp compose.yaml real/compose.yaml
cat > bin/systemctl <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$FAKE/systemctl.log"
exit 0
EOF
chmod +x bin/systemctl
export FAKE="$work/units" SHIPPIN_UNIT_DIR="$work/units" SHIPPIN_SYSTEMCTL="$work/bin/systemctl"
cat > units/shippin-tech-mail.service <<EOF
[Service]
WorkingDirectory=$work/real
ExecStart=/usr/bin/docker compose --env-file $work/real/server.env -f $work/real/compose.yaml up -d
EOF
cat > units/shippin-tech-infra.service <<'EOF'
[Service]
WorkingDirectory=/etc/shippin-tech
ExecStart=/usr/bin/docker compose -f /etc/shippin-tech/infra.compose.yaml up -d
EOF

out=$(bash host.sh install 2>&1)
echo "$out" | grep -q 'removed shippin-tech-infra.service' && echo "ok   the infra unit pointed at a missing file" || { echo "FAIL infra unit"; echo "$out"; exit 1; }
[[ -f units/shippin-tech-mail.service ]] && echo "ok   a unit whose paths exist is kept" || { echo "FAIL kept unit was removed"; exit 1; }
[[ ! -f units/shippin-tech-infra.service ]] && echo "ok   the missing infra unit is gone" || { echo "FAIL infra unit remains"; exit 1; }
unit=$(ls units/shippin-stack-shippin-mail.service)
grep -q "WorkingDirectory=$work" "$unit" && echo "ok   the stack unit uses this directory" || { echo FAIL; cat "$unit"; exit 1; }
grep -q 'compose.yaml up -d' "$unit" && echo "ok   the stack unit starts this compose file" || { echo FAIL; exit 1; }
! grep -q 'node-a' "$unit" && echo "ok   the stack unit does not use node-a" || { echo FAIL; exit 1; }
! grep -q 'infra.compose.yaml' "$unit" && echo "ok   the stack unit does not use the missing infra file" || { echo FAIL; exit 1; }
timer=$(ls units/shippin-backup-shippin-mail.timer)
grep -F -q 'OnCalendar=*-*-* 03:15:00' "$timer" && echo "ok   backup timer is 03:15" || { echo FAIL; cat "$timer"; exit 1; }
grep -q "WorkingDirectory=$work" units/shippin-backup-shippin-mail.service && echo "ok   backup timer runs backup.sh here" || { echo FAIL; exit 1; }
grep -q 'fdb-backup-agent' compose.yaml && echo "ok   compose still defines the backup agent" || { echo FAIL; exit 1; }
grep -q 'secrets/relay' compose.yaml && echo "ok   compose mounts the relay secret directory" || { echo FAIL; exit 1; }
grep -q 'SETUP_KEY_HOURS' compose.yaml && echo "ok   compose still passes the setup-key lifetime" || { echo FAIL; exit 1; }
bash "$src/../install.sh" --help | grep -q -- '--no-backup' && echo "ok   install.sh documents --no-backup" || { echo FAIL; exit 1; }

echo "host.sh: all checks passed"
