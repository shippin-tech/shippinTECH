# Zero-regression requirement

The fork must preserve existing behavior while becoming an independent shippinTECH
product. New integrations and a different implementation are additive until their
replacements prove equivalent behavior. A smaller demo is useful development evidence;
it is not permission to remove existing capabilities.

This is an acceptance requirement, not a claim that zero regressions have been proven.
The repository remains pre-release under [RELEASE.md](RELEASE.md).

Owner clarification, 2026-11-21: follow [ROADMAP.md](ROADMAP.md). The final product
has one owned shippinTECH API. Fresh pre-release installs and breaking private
upstream API/data formats are authorized; legacy compatibility and old-layout
migration are not required. Every existing capability and permission boundary
remains required. Standard protocol interoperability, new-product data/settings
durability, backup/recovery and the PostgreSQL-to-FoundationDB scaling path still
need proof. PostgreSQL is the default install; retain FoundationDB and other
existing storage capabilities. A path rename alone is not independence.

## Baselines and evidence

The local comparison anchor for this requirement is commit
`533ca73524f0fa0ec82b98ced1a8e4cf03318c14`. It already contains known test failures and
incomplete features; those are work to resolve, not an acceptable permanent floor.
This anchor does not establish parity with the original upstream fork point. Use
the current capability inventory, existing tests and observable requirements to
verify each port now. Record the exact upstream revision when available; an unknown
historical revision does not block PostgreSQL setup or porting known behavior, but
does limit any claim of complete equivalence to that upstream revision.

For each replacement, record the baseline and candidate revisions, capability,
input fixtures, expected observable results, test command, environment and results.
Run equivalent cases against both implementations with synthetic data. Document
intentional bug fixes separately with the expected behavior and regression test.
Do not treat silently dropped fields, reduced protocols or changed permissions as fixes.

## Capability floor

Use [api/README.md](api/README.md), [RELEASE.md](RELEASE.md), the API specifications,
and `resources/product/api.json` as the starting inventory. The table below is a
minimum set of areas to cover, not a completed equivalence assessment.

| Area | Required evidence before replacing existing behavior |
| --- | --- |
| Mail and clients | SMTP delivery/submission, queue retries and failure reports; JMAP/IMAP/POP3 behavior; folders, flags, search, compose/reply/forward, drafts, attachments and byte preservation through independent clients |
| Calendar, contacts, files and filtering | Existing JMAP/DAV and ManageSieve operations, round trips, sharing/ACLs, exports and notifications; a greeting or discovery response alone is insufficient |
| Identity and administration | Setup, login, app passwords, two-factor, SSO/SCIM, account changes, roles, organisations, quotas and cross-account isolation; administrative CRUD including stale-write rejection |
| Integrations and AI | Existing Google flows plus Microsoft connections; consent, expiry, cancellation, account isolation, disconnect, download/copy bytes and error recovery; existing AI and MCP permission/approval behavior |
| Persistence and operations | New shippinTECH settings/data survive restart, crash recovery and supported product upgrades; backup restoration and PostgreSQL-to-FoundationDB transfer verified with content. Historical upstream formats need not be imported |
| User experience and performance | Existing navigation, accessible keyboard/mobile flows, clear errors; representative latency, throughput and memory measured against baseline with explicit acceptance thresholds before replacement |

No existing capability becomes optional merely because it is scheduled for a later
rewrite stage. New functionality can be prioritized by adoption evidence. Existing
limitations remain explicitly documented until fixed; do not advertise unimplemented
features as baseline behavior.

## Verification gates

1. Run every applicable check from `.github/workflows/check.yml`: brand/API/inventory
   and MCPL checks, registry generation, formatting/lint, typecheck, **all** unit tests,
   static build and **all** mocked browser tests. Do not narrow the final run to the
   new feature. Do not hide failures with skips, broad retries or weaker assertions.
2. Run the server build and affected protocol/contract suites for runtime changes.
   Use `deploy/local/` for the disposable Docker stack; follow its documented setup.
   Run `ui`'s live browser and contract suites and relevant `deploy/local/verify*.py`
   checks against the exact candidate image. Record image identity and results.
3. Prove replacement-specific behavior against the baseline and candidate using
   disposable fixtures. Prove new-product recovery and storage transfer where
   affected; upstream data-format migration is not required. Keep comparison
   evidence until affected capabilities pass. Performance acceptance needs measured
   fixtures and thresholds, not a subjective claim that the rewrite is faster.
4. Verify real Google/Microsoft consent with designated test accounts before claiming
   live interoperability. Mock success is adapter evidence only. A user enters their
   keys in the documented fields; never collect secrets in chat or test logs.
5. Update the implementation plan with observed results and unresolved coverage.
   An unavailable Docker engine, test account or environment leaves the relevant
   check unverified. It does not turn that check green or permit replacement.

These gates supplement the release gate; they do not authorize a release, real
customer mail or a rollout. Superseded implementations can be removed after their
capabilities pass on the replacement; permanent compatibility is not required.
