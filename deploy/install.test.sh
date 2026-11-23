#!/usr/bin/env bash
# Exercise real installer file generation with isolated fake host tools.
set -euo pipefail
src=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir "$work/bin"
printf '#!/usr/bin/env bash\nexit 0\n' > "$work/bin/docker"
printf '#!/usr/bin/env bash\nexit 0\n' > "$work/bin/flock"
chmod +x "$work/bin/"*
export PATH="$work/bin:$PATH"
image=ghcr.io/shippin-tech/shippin-tech-scale:sha-1111111111111111111111111111111111111111
for backend in postgres foundationdb; do
  dir="$work/$backend"
  args=()
  [[ "$backend" == postgres ]] || args=(--storage foundationdb)
  bash "$src/../install.sh" --from "$src" --dir "$dir" --hostname mail.example.test --image "$image" --no-start "${args[@]}" > "$work/output"
  grep -qx "SHIPPIN_STORAGE=$backend" "$dir/server.env"
  grep -qx "COMPOSE_PROFILES=bundled,$backend" "$dir/server.env"
  [[ -f "$dir/backend.sh" && -f "$dir/claim.sh" ]]
  grep -qx 'MANAGESIEVE_HOST_PORT=4190' "$dir/server.env"
  grep -Fq '${MAIL_BIND:-127.0.0.1}:${MANAGESIEVE_HOST_PORT:-4190}:4190' "$dir/compose.yaml"
  grep -Fq 'node-logs:/var/log/shippin.tech' "$dir/compose.yaml"
  if [[ "$backend" == postgres ]]; then
    grep -q '"@type": "PostgreSql"' "$dir/config.json"
    grep -q '"variableName": "SHIPPIN_POSTGRES_PASSWORD"' "$dir/config.json"
    password=$(sed -n 's/^POSTGRES_PASSWORD=//p' "$dir/server.env")
    [[ ${#password} -eq 40 ]]
    ! grep -Fq "$password" "$dir/config.json"
    ! grep -Fq "$password" "$work/output"
  else
    grep -q '"@type": "FoundationDb"' "$dir/config.json"
  fi
  echo "ok $backend installer generation"
done
if bash "$src/../install.sh" --from "$src" --dir "$work/invalid" --hostname mail.example.test --image "$image" --storage invalid --no-start >/dev/null 2>&1; then
  echo 'FAIL invalid storage accepted'; exit 1
fi
[[ ! -e "$work/invalid" ]]
if bash "$src/../install.sh" --from "$src" --dir "$work/no-image" --hostname mail.example.test --no-start >/dev/null 2>&1; then
  echo 'FAIL PostgreSQL selected an older release implicitly'; exit 1
fi
[[ ! -e "$work/no-image" ]]
before=$(cat "$work/postgres/server.env")
if bash "$src/../install.sh" --from "$src" --dir "$work/postgres" --hostname mail.example.test --image "$image" --no-start >/dev/null 2>&1; then
  echo 'FAIL existing install overwritten'; exit 1
fi
[[ "$(cat "$work/postgres/server.env")" == "$before" ]]
echo 'Installer refusal and secret handling checks passed'

# A normal installation must actually invoke claim after startup, not just print instructions.
mkdir "$work/source"
cp "$src/"* "$work/source/" 2>/dev/null || true
printf '#!/usr/bin/env bash\ncat >/dev/null\necho started > started\n' > "$work/source/update.sh"
cat > "$work/source/claim.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ -f started && "$1" == --password-stdin ]]
IFS= read -r password
[[ "$password" == 'Synthetic install stdin fixture' ]]
touch claimed
EOF
printf '%s\n' 'Synthetic install stdin fixture' | bash "$src/../install.sh" --from "$work/source" --dir "$work/started" --hostname mail.example.test --image "$image" --admin-password-stdin --no-systemd > "$work/started-output"
[[ -f "$work/started/claimed" ]]
! grep -q 'Synthetic install stdin fixture' "$work/started-output"
echo 'ok installation starts services then claims with password on stdin'
