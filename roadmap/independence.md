# Independence ledger

The source and dependency record for A2, A3 and A5 of [ROADMAP.md](../ROADMAP.md),
updated 2026-11-21. Reuse the existing shippinTECH implementations before specifying
new replacements. Each actual replacement records files, baseline behavior, tests
and the removal commit. Missing test evidence means unverified, not automatically
unwritten: the earlier squashed-baseline classification cannot establish authorship.

Measured inventory: [PROVENANCE.md](../PROVENANCE.md). Rules: [REGRESSION.md](../REGRESSION.md). A replacement
is written from the published specification and the product's own requirements. The original is the baseline to
compare against, not text to be paraphrased; it stays until the oracle passes and goes in its own commit.

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
