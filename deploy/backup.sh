#!/usr/bin/env bash
# Backs up or restores a node that runs the bundled services (COMPOSE_PROFILES=bundled).
#
#   bash backup.sh                         # new backup in ./backups/<UTC time>, keeps the newest $KEEP (7)
#   bash backup.sh list                    # what is there
#   bash backup.sh key                     # prints the key-wrapping key, to keep apart from backups
#   bash backup.sh restore backups/<time> --yes   # replaces ALL mail data with that backup
#   sudo bash backup.sh install            # systemd timer: 03:15 local time, every day
#   sudo bash backup.sh uninstall          # remove that timer
#
# A backup holds a PostgreSQL dump or FoundationDB snapshot (accounts, settings, mail
# index), a mirror of the object storage (message bodies and attachments) taken
# right after it, the server's settings file and server.env. It does not copy
# secrets/ (the smarthost key) or the key-wrapping key. Copy ./backups off
# the host as well; a backup on the same disk does not survive losing the disk.
# Nodes on external databases and S3 back those up with their own tools.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
here=$(pwd -P)
unit_dir=${SHIPPIN_UNIT_DIR:-/etc/systemd/system}
systemctl=(${SHIPPIN_SYSTEMCTL:-systemctl})

value() { awk -F= -v k="$1" '$1 == k { sub(/^[^=]*=/, ""); print; exit }' server.env; }
[[ -f server.env ]] || { echo 'Run this in the instance directory.' >&2; exit 1; }
[[ ",$(value COMPOSE_PROFILES)," == *,bundled,* ]] || {
  echo 'backup.sh covers bundled services only. Back up external databases and S3 with their tools.' >&2
  exit 1
}
export COMPOSE_PROFILES="$(value COMPOSE_PROFILES)"
source ./backend.sh
compose=(docker compose --env-file server.env -f compose.yaml)
tool=(docker compose --env-file server.env -f compose.yaml --profile bundled --profile tools run --rm -T objects-tool sh -c)
mc_setup='export HOME=/tmp; cp /tools/mc /tmp/mc && chmod +x /tmp/mc && /tmp/mc alias set s3 http://minio:9000 "$S3_ACCESS_KEY" "$S3_SECRET_KEY" >/dev/null'
mkdir -p backups

backup_unit() {
  local project safe
  project=$(value COMPOSE_PROJECT_NAME)
  project=${project:-shippin-mail}
  safe=$(printf '%s' "$project" | tr -c 'A-Za-z0-9._-' '-')
  printf 'shippin-backup-%s\n' "$safe"
}

require_root_unit() {
  if [[ "$unit_dir" == /etc/systemd/system && ${EUID:-$(id -u)} -ne 0 ]]; then
    echo 'backup: run as root (sudo bash backup.sh install)' >&2
    exit 1
  fi
}

case "${1:-backup}" in
  install)
    require_root_unit
    command -v "${systemctl[0]}" >/dev/null || { echo 'backup: systemd is required for the timer' >&2; exit 1; }
    unit=$(backup_unit)
    mkdir -p -- "$unit_dir"
    cat > "$unit_dir/${unit}.service" <<EOF
[Unit]
Description=shippin.tech mail backup (${here})
After=docker.service
[Service]
Type=oneshot
WorkingDirectory=${here}
ExecStart=/usr/bin/env bash ${here}/backup.sh
EOF
    cat > "$unit_dir/${unit}.timer" <<EOF
[Unit]
Description=Nightly shippin.tech mail backup (${here})
[Timer]
OnCalendar=*-*-* 03:15:00
Persistent=true
RandomizedDelaySec=15min
Unit=${unit}.service
[Install]
WantedBy=timers.target
EOF
    "${systemctl[@]}" daemon-reload
    "${systemctl[@]}" enable --now "${unit}.timer"
    echo "installed ${unit}.timer (03:15 in the host timezone). Copy backups/ off this machine; see README.md."
    ;;
  uninstall)
    require_root_unit
    unit=$(backup_unit)
    "${systemctl[@]}" disable --now "${unit}.timer" >/dev/null 2>&1 || true
    rm -f -- "$unit_dir/${unit}.timer" "$unit_dir/${unit}.service"
    "${systemctl[@]}" daemon-reload || true
    echo "removed ${unit}.timer"
    ;;
  backup)
    exec 9>.update.lock
    flock -n 9 || { echo 'An update or backup is running.' >&2; exit 1; }
    umask 077
    stamp=$(date -u +%Y%m%dT%H%M%SZ)
    # Written under a .partial name and renamed only when every part succeeded,
    # so a listed backup is always a complete one.
    work="$stamp.partial"
    mkdir "backups/$work"
    # Stop this node's writers while taking BOTH database and object snapshots.
    # Preserve a deliberately stopped node; resume a running node even on failure.
    was_running=$("${compose[@]}" ps --status running -q mail)
    resume_mail() {
      rc=$?
      trap - EXIT
      if [[ -n "$was_running" ]]; then
        "${compose[@]}" up -d --wait --wait-timeout 300 mail || rc=1
      fi
      exit "$rc"
    }
    trap resume_mail EXIT
    if [[ -n "$was_running" ]]; then "${compose[@]}" stop mail; fi
    if [[ "$backend" == postgres ]]; then
      echo 'PostgreSQL snapshot...'
      "${compose[@]}" exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --format=custom --no-owner --no-acl' > "backups/$work/postgres.dump"
    else
      "${compose[@]}" up -d fdb-backup-agent >/dev/null 2>&1
      echo 'FoundationDB snapshot...'
      "${compose[@]}" exec -T fdb-backup-agent fdbbackup start -C /var/fdb/fdb.cluster -d "file:///backups/$work/fdb" -w >/dev/null
    fi
    echo "Object storage..."
    "${tool[@]}" "$mc_setup && /tmp/mc mirror --quiet \"s3/\$S3_BUCKET\" /backups/$work/objects" >/dev/null
    "${compose[@]}" run --rm --no-deps -T --entrypoint sh mail -c \
      'cat /var/lib/shippin.tech/config.json' > "backups/$work/config.json"
    cp server.env "backups/$work/server.env"
    printf '%s\n' "$backend" > "backups/$work/backend"
    mv -- "backups/$work" "backups/$stamp"
    chmod -R go-rwx "backups/$stamp"
    echo "Backup written to backups/$stamp"
    echo "Not in it, on purpose: the key-wrapping key. Keep one copy apart (bash backup.sh key); a new server needs it to read saved AI keys, sign-in secrets and Google links."
    keep=${KEEP:-7}
    ls -1d backups/*Z/ 2>/dev/null | sort | head -n "-$keep" | while read -r old; do
      "${tool[@]}" "rm -rf -- '/backups/$(basename "$old")'" >/dev/null 2>&1 || true
      echo "Removed old backup $old"
    done
    ;;
  key)
    # The key-wrapping key, to keep apart from backups (KEYS.md). Printed only on request.
    "${compose[@]}" exec -T mail cat /var/lib/shippin.tech/key-wrapping.key
    ;;
  list)
    ls -1d backups/*Z/ 2>/dev/null || echo 'No backups yet.'
    ls -1d backups/*.partial/ 2>/dev/null | sed 's/^/incomplete, safe to delete: /' || true
    ;;
  restore)
    dir=${2:-}
    [[ "$dir" =~ ^backups/[0-9]{8}T[0-9]{6}Z$ && -d "$dir/objects" && -s "$dir/config.json" ]] || { echo "Usage: bash backup.sh restore backups/<time> --yes" >&2; exit 2; }
    [[ "${3:-}" == --yes ]] || { echo "This replaces every account, setting and message with the backup in $dir. Add --yes to go ahead." >&2; exit 2; }
    saved_backend=foundationdb
    [[ ! -f "$dir/backend" ]] || saved_backend=$(cat "$dir/backend")
    [[ "$saved_backend" == "$backend" ]] || { echo 'Backup database differs from this instance; restore is not a database migration.' >&2; exit 1; }
    exec 9>.update.lock
    flock -n 9 || { echo 'An update or backup is running.' >&2; exit 1; }
    stamp=$(basename "$dir")
    if [[ "$backend" == postgres ]]; then
      [[ -s "$dir/postgres.dump" ]] || { echo 'PostgreSQL archive is missing.' >&2; exit 1; }
      grep -Eq '"@type"[[:space:]]*:[[:space:]]*"PostgreSql"' "$dir/config.json" || { echo 'Backup settings do not select PostgreSQL.' >&2; exit 1; }
      "${compose[@]}" exec -T postgres pg_restore --list < "$dir/postgres.dump" >/dev/null
    else
      [[ -d "$dir/fdb" ]] || { echo 'FoundationDB snapshot is missing.' >&2; exit 1; }
      "${compose[@]}" up -d fdb-backup-agent >/dev/null 2>&1
    fi
    echo "Stopping the mail server..."
    "${compose[@]}" stop mail
    if [[ "$backend" == postgres ]]; then
      echo 'Restoring PostgreSQL...'
      "${compose[@]}" exec -T postgres sh -c 'pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner --no-acl --single-transaction --exit-on-error' < "$dir/postgres.dump"
    else
      echo 'Restoring FoundationDB...'
      "${compose[@]}" exec -T fdb-backup-agent fdbcli -C /var/fdb/fdb.cluster --exec 'writemode on; clearrange "" \xff' >/dev/null
      snapshot=$("${compose[@]}" exec -T fdb-backup-agent sh -c "ls -1 /backups/$stamp/fdb | head -1" | tr -d '\r')
      "${compose[@]}" exec -T fdb-backup-agent fdbrestore start -r "file:///backups/$stamp/fdb/$snapshot" --dest-cluster-file /var/fdb/fdb.cluster -w >/dev/null
    fi
    echo "Restoring object storage..."
    "${tool[@]}" "$mc_setup && /tmp/mc mirror --quiet --overwrite --remove /backups/$stamp/objects \"s3/\$S3_BUCKET\""
    "${compose[@]}" run --rm --no-deps -T --user 0 --entrypoint sh mail -c \
      'cat > /var/lib/shippin.tech/config.json && chown 2000:2000 /var/lib/shippin.tech/config.json' < "$dir/config.json"
    echo "Starting the mail server..."
    "${compose[@]}" up -d --wait --wait-timeout 300 mail
    echo "Restored from $dir"
    ;;
  *)
    awk 'NR>1 && /^#/ { sub(/^# ?/, ""); print; next } NR>1 { exit }' "$0" >&2
    exit 2
    ;;
esac
