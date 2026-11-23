#!/usr/bin/env bash
# Finish ownership on a new installation. The password never enters argv or a file.
# Interactive: bash claim.sh. Automation: feed one password line to --password-stdin.
set +x
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
[[ $# == 0 || ( $# == 1 && "$1" == --password-stdin ) ]] || {
  echo 'Usage: bash claim.sh [--password-stdin]' >&2; exit 2;
}
[[ -f server.env && -f compose.yaml ]] || { echo 'Run claim.sh in the instance directory.' >&2; exit 1; }
compose=(docker compose --env-file server.env -f compose.yaml)
status=0
"${compose[@]}" exec -T mail /usr/local/bin/shippin.tech --claim-admin-status </dev/null || status=$?
case "$status" in
  0) ;;
  3) exit 0 ;;
  *) echo 'Cannot read ownership state. Start the mail container and retry bash claim.sh.' >&2; exit 1 ;;
esac
if [[ "${1:-}" == --password-stdin ]]; then
  "${compose[@]}" exec -T mail /usr/local/bin/shippin.tech --claim-admin
else
  if ! { exec 3<>/dev/tty; } 2>/dev/null; then
    echo 'A terminal is required. For automation, supply the password on stdin with --password-stdin.' >&2
    exit 1
  fi
  password='' repeat=''
  trap 'unset password repeat' EXIT
  printf 'Choose the password for root@system.local: ' >&3
  IFS= read -r -s password <&3 || exit 1
  printf '\nRepeat password: ' >&3
  IFS= read -r -s repeat <&3 || exit 1
  printf '\n' >&3
  [[ "$password" == "$repeat" ]] || { echo 'Passwords differ; retry bash claim.sh.' >&2; exit 1; }
  printf '%s\n' "$password" | "${compose[@]}" exec -T mail /usr/local/bin/shippin.tech --claim-admin
  unset password repeat
fi
host=$(awk -F= '$1 == "MAIL_HOSTNAME" { sub(/^[^=]*=/, ""); print; exit }' server.env)
printf 'Open https://%s/login and sign in as root@system.local.\n' "$host"
