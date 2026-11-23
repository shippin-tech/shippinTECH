#!/usr/bin/env bash
# Installs one shippin.tech mail node on this Linux host with Docker.
#
#   curl -fsSL https://raw.githubusercontent.com/shippin-tech/shippinTECH/main/install.sh \
#     | sudo bash -s -- --hostname mail.example.com
#
# The node runs with its own PostgreSQL, object storage and NATS (all persistent,
# all on this host). Installation asks you to choose the permanent root password.
# Then sign in at https://<hostname>/login as root@system.local.
#
# Options:
#   --hostname NAME    the mail host name, e.g. mail.example.com (required)
#   --dir PATH         instance directory (default /srv/shippin-tech/mail)
#   --ref REF          git tag or branch to take the deploy files from (default main)
#   --image IMAGE      exact candidate image (required for PostgreSQL)
#   --storage BACKEND  postgres (default) or foundationdb (scale installation)
#   --from DIR         copy the deploy files from a local checkout's deploy/ instead
#   --admin-password-stdin  read the permanent administrator password from stdin
#   --no-start         write the files and stop, so you can review server.env first
#   --auto-update      also install the timer that follows the release channel
#   --no-systemd       do not install the boot unit
#   --no-backup        do not install the nightly backup timer
set +x
set -euo pipefail

REPO=https://raw.githubusercontent.com/shippin-tech/shippinTECH
IMAGE_BASE=ghcr.io/shippin-tech/shippintech-scale
ref=main dir=/srv/shippin-tech/mail host='' image='' from='' start=true auto=false systemd=true backup=true
backend=postgres
claim_stdin=false

die() { echo "install: $*" >&2; exit 1; }
while [[ $# -gt 0 ]]; do
  case "$1" in
    --hostname) host=${2:-}; shift 2 ;;
    --dir) dir=${2:-}; shift 2 ;;
    --ref) ref=${2:-}; shift 2 ;;
    --image) image=${2:-}; shift 2 ;;
    --storage) backend=${2:-}; shift 2 ;;
    --from) from=${2:-}; shift 2 ;;
    --admin-password-stdin) claim_stdin=true; shift ;;
    --no-start) start=false; shift ;;
    --auto-update) auto=true; shift ;;
    --no-systemd) systemd=false; shift ;;
    --no-backup) backup=false; shift ;;
    -h|--help) awk 'NR>1 && /^#/ { sub(/^# ?/, ""); print; next } NR>1 { exit }' "$0"; exit 0 ;;
    *) die "unknown option $1 (try --help)" ;;
  esac
done
[[ "$backend" == postgres || "$backend" == foundationdb ]] || die '--storage must be postgres or foundationdb'
# Older release images omit the PostgreSQL driver. Require a reviewed candidate.
[[ "$backend" != postgres || -n "$image" ]] || die 'PostgreSQL needs --image with a candidate digest or sha-commit built with PostgreSQL; no older release image is selected automatically.'
if [[ -n "$image" && ! "$image" =~ ^ghcr\.io/[a-z0-9._/-]+(@sha256:[a-f0-9]{64}|:sha-[a-f0-9]{40})$ ]]; then
  die '--image must be an immutable GHCR digest or sha-<full-commit> tag'
fi

[[ "$(uname -s)" == Linux || -n "$from" ]] || die 'this installer is for Linux hosts'
[[ "$host" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$ ]] \
  || die 'pass --hostname with the mail host name, for example --hostname mail.example.com'
command -v docker >/dev/null || die 'Docker is not installed. Install Docker Engine first: https://docs.docker.com/engine/install/'
docker compose version >/dev/null 2>&1 || die 'the Docker Compose plugin (docker compose) is missing'
docker info >/dev/null 2>&1 || die 'cannot talk to Docker; run as root or as a member of the docker group'
command -v flock >/dev/null || die 'flock is missing; install util-linux'
[[ -e "$dir/server.env" ]] && die "$dir already holds an installation; upgrade it with: cd $dir && bash update.sh <image>"

if [[ -n "$from" ]]; then
  from=$(cd -- "$from" && pwd) || die "--from $from is not a directory"
fi
mkdir -p "$dir"
cd "$dir"
files=(compose.yaml backend.sh claim.sh update.sh auto-update.sh verify-install.sh backup.sh host.sh relay.sh relay.py config.json config.postgres.json server.env.example relay.env.example README.md PRODUCTION.md)
for f in "${files[@]}"; do
  if [[ -n "$from" ]]; then
    cp -- "$from/$f" "$f"
  else
    curl -fsSL "$REPO/$ref/deploy/$f" -o "$f" || die "could not download deploy/$f for $ref"
  fi
done
if [[ "$backend" == postgres ]]; then cp config.postgres.json config.json; fi
chmod +x claim.sh update.sh auto-update.sh verify-install.sh backup.sh host.sh relay.sh relay.py

if [[ -z "$image" ]]; then
  docker pull -q "$IMAGE_BASE:release" >/dev/null \
    || die "could not pull $IMAGE_BASE:release; if the image is private, run: docker login ghcr.io"
  image=$(docker image inspect --format '{{range .RepoDigests}}{{println .}}{{end}}' "$IMAGE_BASE:release" \
    | grep -m1 -F "$IMAGE_BASE@sha256:") || die 'could not resolve the release image to a digest'
fi

secret() { head -c 512 /dev/urandom | LC_ALL=C tr -dc 'A-Za-z0-9' | cut -c1-"$1"; }
project="shippin-$(printf '%s' "${host%%.*}" | tr -c 'a-z0-9-' '-')"
umask 077
cat > server.env <<EOF
# Written by install.sh on $(date -u +%F). Change values here, then: bash update.sh "\$SHIPPIN_IMAGE"
COMPOSE_PROJECT_NAME=$project
COMPOSE_PROFILES=bundled,$backend
SHIPPIN_STORAGE=$backend
SHIPPIN_IMAGE=$image
MAIL_HOSTNAME=$host
# Optional break-glass administrator, separate from the owner created at install.
RECOVERY_ADMIN=
# Hours internal installer authority lasts before an unfinished claim needs a restart.
SETUP_KEY_HOURS=24
# Bundled object storage and NATS on this host.
S3_ENDPOINT=http://minio:9000
S3_ACCESS_KEY=shippin
S3_SECRET_KEY=$(secret 40)
S3_BUCKET=mail
S3_REGION=us-east-1
NATS_ADDRESS=nats:4222
# Mail and HTTPS listen on every interface; the plain HTTP management port stays local.
HTTP_BIND=127.0.0.1
HTTPS_BIND=0.0.0.0
MAIL_BIND=0.0.0.0
HTTP_HOST_PORT=8080
SMTP_HOST_PORT=25
SMTPS_HOST_PORT=465
SUBMISSION_HOST_PORT=587
IMAPS_HOST_PORT=993
MANAGESIEVE_HOST_PORT=4190
HTTPS_HOST_PORT=443
# Free plan (see README.md, "Free plan"). Closed until you open it.
PANEL_SIGNUP=closed
PANEL_MAX_DOMAINS=100
PANEL_MAILBOXES=5
PANEL_ALIASES=20
PANEL_MAILBOX_MB=1024
PANEL_SIGNUPS_PER_HOUR=10
PANEL_SENDS_PER_DAY=200
EOF
if [[ "$backend" == postgres ]]; then printf 'POSTGRES_PASSWORD=%s\n' "$(secret 40)" >> server.env; fi
umask 022
cp -- relay.env.example relay.env
mkdir -p secrets/relay
chmod 755 secrets/relay

if ! $start; then
  echo "Files written to $dir. Review server.env, then run: cd $dir && bash update.sh $image && bash claim.sh"
  exit 0
fi

bash update.sh "$image"
if $claim_stdin; then bash claim.sh --password-stdin; else bash claim.sh; fi
if $systemd && command -v systemctl >/dev/null && [[ -d /run/systemd/system ]]; then
  if $backup; then
    bash host.sh install
  else
    BACKUP_TIMER=skip bash host.sh install
  fi
fi
if $auto; then
  bash auto-update.sh install
fi

cat <<EOF

Installed in $dir. Before people can use it:
  1. DNS: an A record (and AAAA if you use IPv6) for $host pointing at this server,
     and reverse DNS for the server's IP set to $host at your hosting provider.
  2. Firewall: allow TCP 25, 465, 587, 993, 4190 and 443 in. Outbound TCP 25 must be open.
  3. Certificate: until one is issued, clients see the server's own certificate.
     See README.md, "TLS certificate".
  4. Check the install at any time: cd $dir && sudo bash verify-install.sh
EOF
