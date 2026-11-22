# Independence ledger

The source and dependency record for A2, A3 and A5 of [ROADMAP.md](../ROADMAP.md),
updated 2026-11-22. Reuse the existing shippinTECH implementations before specifying
new replacements. Each actual replacement records files, baseline behavior, tests
and the removal commit. Missing test evidence means unverified, not automatically
unwritten: the earlier squashed-baseline classification cannot establish authorship.

**History gate first:** [the reconciliation plan](reconciliation/README.md) covers
all available branches, checkpoint refs, dropped stashes and stored commits. The
pre-reset development tip and public baseline have identical trees. Existing own
implementations must be retained. Historical name/header changes are separately
identified, and changed or removed behavior has a review queue. D1-D11 start only
after that reconciliation and the owner's implementation freeze have been resolved.

Measured inventory: [PROVENANCE.md](../PROVENANCE.md). Rules: [REGRESSION.md](../REGRESSION.md). A replacement
is written from the published specification and the product's own requirements. The original is the baseline to
compare against, not text to be paraphrased; it stays until the oracle passes and goes in its own commit.

## Current executable work ? 2026-11-22

[execution.csv](execution.csv) expands the D1?D11 map into 17 bounded cards,
including existing-work absorption, owner claim, external feeds, source closure,
Svelte parity and final qualification. [current-review.csv](reconciliation/current-review.csv)
assigns all 3,193 historical review inputs without claiming they have been resolved.
The inputs/reuse/delta/proof/completion columns are the implementation handoff.

**First feature cut:** D1/D2/D3/D9 are one configuration-and-consumer retirement
boundary. Ordinary packaged builds must exercise it without `test_mode`, a synthetic
license, or a recovery-admin override. Current integration tests can receive a test
license and therefore cannot establish removal of the runtime coupling.

**D0 claim contract:** local installer authority claims `root@system.local` with
hidden password entry/confirmation (or an explicit unattended secret input), reusing
current validation and root creation. Define durable unclaimed/claimed state separately
from temporary key presence and recovery credentials. The claim is one-time and
concurrency-safe; an interrupted attempt is resumable without replacing an existing
administrator. No secret in process arguments, logs or URLs. Root manages the server;
real domains/mailboxes remain independent. Final success means normal sign-in works.
Test weak input, failure/retry, simultaneous claims, restart, repeat claim and optional
recovery configuration. Do not expose an anonymous first-visitor-wins public takeover.
The internal transport/authority implementation is authored in D0; no new public
compatibility endpoint is mandated by this planning document.

**D12 external inputs:** the current spam-filter URL is the working upstream feed;
fixing its typo did not make rule distribution independent. Inventory actual build
and runtime downloads, author permitted/owned replacements and prove rule behavior.
Classify inert legacy strings separately from active dependencies.

**Source completion:** retained own source, permitted attributed dependencies and
independent replacements are different outcomes. Rename/translation/edit volume is
not a release criterion: modifying covered source does not itself remove its license
conditions ([GNU FAQ](https://www.gnu.org/licenses/gpl-faq.html)). No waiver is assumed
or requested. Unresolved origins stay explicit; preserve notices until resolved.
This plan supplies an engineering separation process, not a claim that the current
repository is license-free.

## Remaining runtime dependency map — inspected 2026-11-21

This is the immediate engineering cut, before installer expansion. Paths below are
active source references inspected in the current tree. The existence of an own
implementation does not prove that every old caller uses it or that it has full
behavior parity. Each row states the connection still required. All rows are open
until the listed workflow passes on the candidate.

Paths in the old-caller column are relative to `crates/`. The replacement column
names existing implementation, or explicitly says when the shared replacement is
not present. These are work boundaries, not instructions to copy old implementations.

| Cut | Old caller / dependency still present | Existing shippinTECH implementation and remaining work | Required proof before removal |
| --- | --- | --- | --- |
| D1: startup and feature configuration | `common/src/config/mod.rs`, `common/src/enterprise/{config,mod,license}.rs`, `common/src/lib.rs` | Add owned feature configuration that initializes the existing shippin modules independently. Currently a failed license parse downgrades data/blob/memory stores, sets metrics/tracing to None and clears directories. Replace that coupling; do not simply return true from the edition check | Boot and reload configured PostgreSQL plus replica/composite configurations with directories, metrics and tracing intact; unsupported/malformed configuration still fails clearly. Repeat affected behavior in the FDB profile |
| D2: account creation, tenant permissions and quotas | `common/src/cache/directory.rs`, `common/src/auth/{authentication,permissions}.rs`, `common/src/storage/quota.rs`, `jmap/src/registry/mapping/principal.rs` | Reuse `common/src/shippin/{orgs,sso,provisioning}.rs` and own HTTP org/provisioning routes. Connect domain-directory lookup and provisioning; preserve tenant permissions and byte/account quotas while removing licensed-account limits. Existing organisation management alone is not proof of domain-directory/tenant parity | `org.spec.ts`, `people.spec.ts`, `sso.spec.ts`, `verify_v1.py`, directory/system suites; cross-tenant reads/writes denied, quotas enforced, both internal and configured external directory account paths exercised |
| D3: telemetry and alerts | `common/src/enterprise/alerts.rs`, `common/src/manager/boot.rs`, `common/src/cache/reload.rs`, `common/src/telemetry/metrics/prometheus.rs`, `services/src/task_manager/{scheduler,maintenance,index}.rs`, `http/src/api/{mod,telemetry}.rs` | Reuse `common/src/shippin/{stats,trace_history}.rs`, `http/src/shippin/{stats,trace,alerts}.rs`, `services/src/alerts.rs`. Unify collector/retention/maintenance callers and permission checks; remove the old renewal task only after feature scheduling is independent. Preserve Prometheus counters, gauges and histograms | `dashboard.spec.ts`, `tracing.spec.ts`, `alerts.spec.ts` plus metrics export, retention and reload checks. One event produces the intended history/alert, not duplicates; all supported metrics remain available |
| D4: masked-address routing | `common/src/{cache/principals,network/mta}.rs`, `common/src/enterprise/masked.rs`, `jmap/src/registry/mapping/masked_email.rs` | Reuse `common/src/shippin/masks.rs` and `http/src/shippin/masked.rs`. Reconcile recipient resolution and MTA routing with the own mask store. Existing own HTTP CRUD is not enough to remove the old address parser/resolver | `masked.spec.ts`, SMTP recipient/delivery tests; create, resolve, deliver, disable, delete and limits under the same own address model; any baseline sending/reply behavior gets an explicit test |
| D5: deletion and recovery | `services/src/task_manager/index.rs`, `jmap/src/registry/mapping/task.rs`, enterprise retention fields | Reuse `common/src/shippin/recover.rs`, `email/src/message/recover.rs`, `http/src/shippin/recover.rs`. Connect archive-on-delete, account deletion/cancellation, retention and purge jobs. Retain the full supported recoverable object set | `recover.spec.ts` plus delete/purge/restart tests; recovered content, identities and ACLs match fixtures; deletion cannot silently bypass retention |
| D6: calendar templates, RSVP and branding | `services/src/task_manager/{alarm,imip}.rs`, `http/src/request.rs`, `common/src/enterprise/mod.rs` | Own calendar template overrides are already called by alarm/imip; remove old enterprise fallbacks while preserving default templates. Reuse `common/src/shippin/{calendar_templates,brand}.rs` and own HTTP routes; map RSVP and logo callers to the own product surfaces | `calendar-templates.spec.ts`, `calendar.spec.ts`, `brand.spec.ts`; custom and default reminder/invitation bodies, RSVP, domain branding and permission refusals |
| D7: AI classification and script prompts | `spam-filter/src/analysis/llm.rs`, `common/src/scripts/plugins/llm_prompt.rs`, `common/src/enterprise/llm.rs` | Reuse `common/src/shippin/ai.rs` and service keys. The old callers still read `core.enterprise` for models/configuration. Implement equivalent named-model selection, prompt parameters, response interpretation and permissions through the own provider layer | `assistant.spec.ts` is only part of the proof. Add actual spam classification and Sieve prompt workflows, permission denial, provider error/timeout and disabled-provider cases; no echo-test-only completion |
| D8: SCIM and product capability reporting | `scim/src/auth.rs`, `http/src/request.rs`, `http/src/auth/permissions.rs`, `jmap/src/registry/mod.rs` | Own `http/src/shippin/provisioning.rs` exists. Reconcile the old SCIM handler and own endpoint by behavior; preserve all supported filters/operations before retiring either path. Replace edition labels/object gates with actual permissions and feature availability | `scim.spec.ts`, SCIM/system suites, UI permission cases; admin/user/tenant boundaries and required provisioning operations; the known eq-only own endpoint is not automatically full protocol parity |
| D9: storage composition | `common/src/config/mod.rs`, `store/src/lib.rs`, `store/src/backend/composite/`, backend feature manifests | Keep PostgreSQL, FoundationDB and existing backend options. Own the application selection/configuration and remove edition-triggered downgrades. Source provenance of composite implementations is a separate replacement decision, not resolved by enabling them | Primary/replica and shard reads/writes, error/recovery paths, tenant quotas and backend feature matrix. No capability difference between self-hosted and managed-hosted builds |
| D10: admin API/schema | `http/src/api/registry_rest.rs`, `jmap/src/registry/{get,query,set}.rs`, `registry/src/schema/`, `resources/schema/schema.json.gz` | The Svelte client exists; the server still constructs private JMAP requests. Own schema inputs and generators, implement direct Rust application operations and move UI/MCP consumers. Replace generated artifacts from owned sources; editing generated output alone is not closure | Contract generation from clean checkout, CRUD/action/security/patch/ETag tests, active private method callers removed, old private routes unavailable, standard protocols still pass |
| D11: filter language/runtime | `sieve-rs` in `common`, `email`, `jmap`, `managesieve`, `smtp` and `tests` manifests | Independent runtime is not present in this tree. Implement the specification below, covering user filtering AND MTA-stage scripts/functions/plugins. Do not remove filtering or swap in a reduced editor/runtime | Full S1 oracle below: compile errors/positions, all supported extensions, action effects, delivery, ManageSieve/JMAP, fuzz and performance fixtures; then remove the dependency |

The first implementation boundary is **D1 together with its D2/D3/D9 callers**:
own feature initialization must preserve the configured capability graph. This is
not permission to disable the remaining D4–D8 paths. Port those into the existing
implementations in the table, with their tests, before removing the old module.
D10 owns the application contract; D11 closes a distinct third-party runtime
dependency. None of these tasks is fulfilled by changing product names or headers.

### What is already established and what is not

- Existing Svelte and own Rust implementations are real source in the repository;
  they are reuse targets, not a reason to commission their rewrites again.
- People/API consumer changes are committed as `7553600`; the current anchor is
  `19c4409`. Its full web gate has 289 unit tests and 13 mocked browser tests.
  Packaged live behavior remains a separate gate; do not redo this committed work.
- The failing license stub is a concrete runtime coupling. Removing that coupling
  is required, but does not establish authorship of the rest of the Rust engine.
- The squashed provenance inventory cannot tell how much pre-baseline code was
  already independently rewritten. The exact remaining application-source rewrite
  count is unresolved. No percentage threshold or "just rename it" completion is
  part of this plan.

### Application-source closure record required before an independence claim

For every shipped application file/generated artifact, the existing provenance
ledger must record: path, origin evidence, retain/replace decision, replacement
path when required, capability IDs, tests/results and removal commit. Classify
confirmed existing own code as retained; do not infer origin from a folder name.
Use repository history, original source comparison and concrete authorship records.
Unresolved rows stay unresolved rather than turning into a blanket whole-crate
rewrite. An exact historical fork revision helps that comparison but does not block
the known D1–D11 work. No waiver answer is requested for these engineering tasks.

**Separation completion requires both:** D1–D11 closed with no feature loss, and
every shipped application-source row resolved. The former is operational removal
of known dependencies; it does not substitute for the latter.

## Minimal replacement decisions

The manifest inventory in `provenance/dependencies.csv` records S2–S6 as
`Apache-2.0 OR MIT`; author identity alone is not a reason to replace them. Retain
these as attributed third-party dependencies, verifying the exact resolved package
and notices before distribution. This is not a claim that they are our source.
S1 is recorded as `AGPL-3.0-only` and remains a replacement boundary for the requested
independent product. No waiver discussion blocks the engineering work.

| Slice | Package (version) | Lines | Decision | Status |
|---|---|---:|---|---|
| S1 | `sieve-rs` 0.7.3 | 19,233 | Replace | Specified below; baseline not yet run |
| S2 | `smtp-proto` 0.2.5, `mail-builder` 1.0.0 and 0.5.0, `hashify` 0.2.9, `encodify` 1.0.2 | not measured | Retain as attributed dependencies; verify resolved versions/notices | No replacement scheduled solely for upstream authorship |
| S3 | `mail-parser` 0.11.9 | not measured | Retain as an attributed dependency; verify resolved version/notices | No replacement scheduled solely for upstream authorship |
| S4 | `mail-auth` 0.13.3 | not measured | Retain as an attributed dependency; verify resolved version/notices | No replacement scheduled solely for upstream authorship |
| S5 | `calcard` 0.3.14, `jmap-tools` 0.1.8 | not measured | Retain as attributed dependencies; verify resolved versions/notices | No replacement scheduled solely for upstream authorship |
| S6 | `dns-update` 0.6.0, `jmap-client` 0.4.3 (tests only) | not measured | Retain as attributed dependencies; verify resolved versions/notices | No replacement scheduled solely for upstream authorship |
| S7 | `registry` schema generator (82k generated lines, no generator in tree) | 81,831 | Own the schema and the generator | Not started |

Line counts of the other packages are measured when their slice is specified, from the published source, not
guessed.

## Application source decisions

These are separate from the package table. No row is closed merely by changing a
name, license header or feature flag. Exact file decisions and their evidence are
the output of ROADMAP A5, alongside the ports; unresolved origin is not a blanket
instruction to rewrite the whole crate.

| Area | Execution package | Decision and completion requirement |
| --- | --- | --- |
| Existing Svelte product and style tokens | A0, A4 | Keep; finish missing workflows and prove the full capability inventory |
| `common/src/shippin`, own HTTP/services modules | A2 | Keep and extend; map each remaining old caller to these implementations, port missing behavior, then remove obsolete paths |
| Private registry API and generated schemas | A3 / S7 | Own application operations, schema sources and generation; remove private JMAP dispatch after all consumers move |
| Remaining mail/groupware application code | A5 | Identify actual unresolved/inherited implementations per file; replace required slices with mail/protocol behavior proof |
| Storage, coordination and infrastructure application code | A1, A2, A5, A6 | Preserve PostgreSQL, FoundationDB and other existing capabilities; replace only identified source while retaining backend semantics |
| Third-party packages and assets | A5 | Record resolved source/license/notices; retain permitted dependencies and replace those incompatible with the chosen product boundary |

## S1: the Sieve runtime

### What it really is

`sieve-rs` is not only the filter language users write. In this engine it runs two kinds of script and
bridges to one native subsystem, found by searching the tree for `sieve::` (49 files):

1. **User mail filtering**: ManageSieve (`crates/managesieve`), the JMAP `SieveScript` object
   (`crates/jmap/src/sieve`), and delivery (`crates/email/src/sieve/ingest.rs`).
2. **MTA stage scripts**: administrator-defined scripts run at connect, EHLO, MAIL, RCPT and DATA
   (`crates/common/src/network/mta.rs`, `crates/smtp`), with the engine's own functions and plugins
   (`crates/common/src/scripts/functions/*`, `scripts/plugins/*`: DNS, HTTP, exec, lookup, query, headers,
   text, LLM prompt).
3. **The `spamtest` extensions** read the native spam filter's verdict (`SpamStatus`). The spam filter itself
   (`crates/spam-filter`, 8.7k lines of Rust) does not use `sieve-rs` and is not part of this slice.

A replacement must therefore run the same scripts to the same results in both places and answer `spamtest`
from the same verdict.

### Surface to reproduce

The names the rest of the engine imports, so call sites change by one dependency line:
`Compiler`, `Runtime`, `Sieve`, `Context`, `FunctionMap`, `Input`, `Event`, `Envelope`, `Mailbox`, `Recipient`,
`SpamStatus`, `runtime::Variable`, `runtime::RuntimeError`, `compiler::ErrorType`, `compiler::Number`,
`compiler::ReceivedPart`, `compiler::grammar::Capability`, plus the `validate_sieve_script` path used by
ManageSieve and JMAP. The in-tree functions and plugins already live in this repository and are not part of the
replacement.

Extensions to implement, taken from the `Capability` list of the baseline: envelope (and DSN and deliver-by),
fileinto, encoded-character, body, convert, copy, relational, date, index, duplicate, variables, editheader,
foreverypart, mime, replace, enclose, extracttext, enotify, redirect (DSN, deliver-by), environment, reject,
ereject, extlists, subaddress, vacation (and seconds), fcc, mailbox, mailboxid, mboxmetadata, servermetadata,
specialuse, imap4flags, ihave, imapsieve, include, regex, spamtest, spamtestplus, virustest, and the engine's
expression, while and address-part extensions. Each is implemented from its RFC (RFC 5228 for the base
language; the extension RFCs by number in the slice's test plan), not from the baseline's source.

### Oracle

In order of strength. Every one runs in CI once the slice lands.

1. The inherited engine suite sections that exercise Sieve: delivery with Sieve, MTA stage scripts,
   ManageSieve, JMAP SieveScript (`tests/src/smtp`, `tests/src/jmap`, `tests/src/imap`). Baseline recorded by
   ROADMAP R1 first.
2. **Differential run.** The baseline package ships about 300 Sieve fixtures (compile errors, warnings,
   recovery, comparators, and one directory per extension). Both implementations compile and run each and must
   agree on: accepted or rejected, error type and position, actions produced, variables set.
3. The RFC examples, extracted into fixtures with the RFC section cited.
4. Differential fuzzing of the compiler and the runtime on generated scripts and messages. A disagreement is a
   bug in one of them and gets a fixture.
5. The engine suite's own Sieve fixtures (18 `.sieve` files under `tests/resources/jmap/sieve` and
   `tests/resources/smtp/sieve`: include, redirect, fcc, mailbox, stage scripts, awl, spamtest) run on both
   implementations with identical results.

### Done when

- `crates/sieve` builds with the same call-site surface, behind a feature that selects it over `sieve-rs`.
- Oracle 1 to 5 pass with the new crate, and results are recorded in the table below.
- Performance: compile and run time for the fixture set and a representative stage script within the
  threshold recorded by ROADMAP R5.
- The `sieve-rs` dependency is removed in its own commit and `PROVENANCE.md` regenerated.

## Slice log

| Slice | Baseline revision | Oracle result | Own implementation lands | Original removed |
|---|---|---|---|---|
| S1 | | | | |
