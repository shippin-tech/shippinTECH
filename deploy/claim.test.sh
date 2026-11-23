#!/usr/bin/env bash
# Exercise the shipped helper: secret on stdin, never argv/output.
set -euo pipefail
src=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir "$work/bin"
cp "$src/claim.sh" "$work/claim.sh"
printf 'MAIL_HOSTNAME=mail.example.test\n' > "$work/server.env"
touch "$work/compose.yaml"
export CLAIM_TEST_STATE="$work" PATH="$work/bin:$PATH"
cat > "$work/bin/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$CLAIM_TEST_STATE/argv"
case "${*: -1}" in
  --claim-admin-status) cat >/dev/null; exit "$(cat "$CLAIM_TEST_STATE/status")" ;;
  --claim-admin)
    IFS= read -r password
    [[ "$password" == 'Synthetic installer password fixture' ]] || exit 1
    echo called >> "$CLAIM_TEST_STATE/claimed"
    echo 'Administrator root@system.local created.' ;;
  *) exit 99 ;;
esac
EOF
chmod +x "$work/bin/docker"
echo 0 > "$work/status"
output=$(printf '%s\n' 'Synthetic installer password fixture' | bash -x "$work/claim.sh" --password-stdin 2>&1)
[[ "$output" == *'https://mail.example.test/login'* ]]
[[ "$output" != *'Synthetic installer password fixture'* ]]
! grep -q 'Synthetic installer password fixture' "$work/argv"
[[ $(wc -l < "$work/claimed") == 1 ]]
echo 'ok   stdin claim succeeds without exposing password in arguments or trace output'
echo 3 > "$work/status"
bash "$work/claim.sh" --password-stdin </dev/null
[[ $(wc -l < "$work/claimed") == 1 ]]
echo 'ok   already claimed does not consume input or change administrator'
echo 1 > "$work/status"
if bash "$work/claim.sh" --password-stdin </dev/null >/dev/null 2>&1; then exit 1; fi
[[ $(wc -l < "$work/claimed") == 1 ]]
echo 'ok   server failure refuses claim'
echo 0 > "$work/status"
if printf 'weak\n' | bash "$work/claim.sh" --password-stdin >/dev/null 2>&1; then exit 1; fi
printf '%s\n' 'Synthetic installer password fixture' | bash "$work/claim.sh" --password-stdin >/dev/null
[[ $(wc -l < "$work/claimed") == 2 ]]
echo 'ok   failed claim is retryable'

if [[ "$(uname -s)" == Linux ]]; then
  python3 - <<'PY'
import errno, os, pty, select, time
from pathlib import Path
work = Path(os.environ['CLAIM_TEST_STATE'])
def prompt_claim(repeat):
    pid, fd = pty.fork()
    if pid == 0:
        os.execlp('bash', 'bash', str(work / 'claim.sh'))
    output = b''
    sent = 0
    deadline = time.monotonic() + 15
    try:
        while time.monotonic() < deadline:
            if not select.select([fd], [], [], 0.2)[0]:
                continue
            try:
                chunk = os.read(fd, 4096)
            except OSError as error:
                if error.errno == errno.EIO: break
                raise
            if not chunk: break
            output += chunk
            if sent == 0 and b'Choose the password' in output:
                os.write(fd, b'Synthetic installer password fixture\n'); sent = 1
            elif sent == 1 and b'Repeat password:' in output:
                os.write(fd, repeat + b'\n'); sent = 2
        else:
            os.kill(pid, 9)
            raise AssertionError('claim prompt timed out')
    finally:
        os.close(fd)
    _, status = os.waitpid(pid, 0)
    assert b'Synthetic installer password fixture' not in output, 'password echoed'
    return os.waitstatus_to_exitcode(status)
assert prompt_claim(b'different synthetic fixture') != 0
assert len((work/'claimed').read_text().splitlines()) == 2
assert prompt_claim(b'Synthetic installer password fixture') == 0
assert len((work/'claimed').read_text().splitlines()) == 3
print('ok   interactive confirmation rejects mismatch and never echoes password')
PY
fi
