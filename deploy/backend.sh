#!/usr/bin/env bash
# Shared deployment selection. Source from an instance directory with server.env.
backend_value() { awk -F= -v k="$1" '$1 == k { sub(/^[^=]*=/, ""); print; exit }' server.env; }
backend=${SHIPPIN_STORAGE:-$(backend_value SHIPPIN_STORAGE)}
# An existing installation with no selector uses its original FoundationDB store.
backend=${backend:-foundationdb}
case "$backend" in
  postgres) store_type=PostgreSql ;;
  foundationdb) store_type=FoundationDb ;;
  *) echo 'SHIPPIN_STORAGE must be postgres or foundationdb.' >&2; return 1 ;;
esac
if [[ -f .storage-backend && "$(cat .storage-backend)" != "$backend" ]]; then
  echo 'Storage selection differs from this installation. Changing it is not a data migration; use a separate instance.' >&2
  return 1
fi
profiles=$(backend_value COMPOSE_PROFILES)
bundled=false
[[ ",$profiles," == *,bundled,* ]] && bundled=true
for candidate in postgres foundationdb; do
  if [[ ",$profiles," == *,$candidate,* && ( "$candidate" != "$backend" || "$bundled" != true ) ]]; then
    echo 'Database profile conflicts with SHIPPIN_STORAGE or bundled services.' >&2
    return 1
  fi
done
if $bundled && [[ ",$profiles," != *,$backend,* ]]; then
  profiles="${profiles:+$profiles,}$backend"
fi
export COMPOSE_PROFILES="$profiles"
if [[ "$backend" == postgres && -z "$(backend_value POSTGRES_PASSWORD)" ]]; then
  echo 'Set POSTGRES_PASSWORD in server.env before starting PostgreSQL.' >&2
  return 1
fi
