# Existing-work reconciliation and absorption plan

Audited 2026-11-21 against `7976eae9a96bb3034972dd49a7aea77221f67f32`.
Scope: this repository, its available Git objects, refs, reflogs, historical PR
heads and current uncommitted work. No other project's history is included.
This is the prerequisite to [ROADMAP.md](../../ROADMAP.md), not a new product plan.
Product implementation remains stopped at the owner's instruction.

## Current refresh ? 2026-11-22

The original audit below remains historical evidence. At `19c4409`, all 53 recorded
source heads plus current main cover **257/257 stored commits**, with zero uncovered.
All 40 refs remain, including 32 checkpoints; no active stash, one worktree. The
retained remote branch is now 14 commits behind and has zero unique commits.
The two dropped stashes remain represented by the original source ledger; no stash
or branch was applied or deleted in this pass.

[current-audit.json](current-audit.json) records the exact refs and stored commit IDs.
[current-review.csv](current-review.csv) preserves every original review row with the
current blob, exact-content destination when found, and an execution-card assignment.
There are **3,193 open reviews**, including 37 exact blob matches; no behavioral
absorption is inferred from matching bytes. Routing is mechanical by path/packet;
reviewers must confirm the target card and record semantic evidence before closure.

Five later commits are now accounted for: `65a5805` history planning, `7553600`
People/API, `e952af3` PostgreSQL install/recovery, `1b39550` install defects and
`19c4409` system-domain/root. R8's product checkpoint is committed; its old
uncommitted wording below is historical. Broader R2?R7 absorption remains open.
The execution handoff is [../execution.csv](../execution.csv), not a second roadmap.

## What the evidence establishes

**The existing base was carried into the public baseline.** The last recovered
pre-reset development tip `6c4e00fc20304c5dca20366e359352228ec1b4f9` and the
parentless public root `6693b60be553b880ffd9c6ca9b1dd5640f41d036` have the same tree:
`c07e94e3edb239747fcc5068fb4be2a72d36240b`. This proves byte-for-byte tracked-tree
continuity across that history reset. It does not prove the source was independently
authored, or that features removed before that snapshot were preserved.

Compared with audited main, that recovered tip has 3,724 identical path/mode/blob
entries, one blob present elsewhere, 310 changed paths and **zero absent paths**.
Do not commission a fresh rewrite of all that existing work. Reconcile changes and
finish the specific remaining connections in the [dependency map](../independence.md).

| Evidence | Result | Meaning for the plan |
| --- | --- | --- |
| Live remote branches | `main` at `7976eae`; `cursor/production-deploy-parity-c403` at `b09dbc3` | Verified with `git ls-remote --heads origin`. The feature branch has 0 unique commits and is 9 behind main; no merge needed |
| Local worktrees / active stash | One worktree; no active stash entries | Does not exclude dropped stash objects |
| Current refs | 40: 32 agent checkpoints, 1 local branch, 3 remote refs including HEAD, 4 tags | Every ref has its own source and disposition row |
| Stored commits | 252; 101 unreachable from current refs | Every object is listed, including work hidden by the history reset |
| Source heads | 53 source records, including PR heads/merges, stash parents and recovered history | Their combined ancestry covers all 252 stored commits; zero uncovered commits |
| Intermediate versions | 21,352 first-parent file-change rows across all 252 commits | Captures earlier additions and removals, not just final branch tips |
| Non-identical history | 3,193 distinct path/blob/comparison review items | Assigned to packets below; these are review units, **not 3,193 required rewrites** |
| Historical absent paths | 152 paths: 82 have recorded moves to existing destinations, 7 have candidate destinations, 6 concern removed deployment planning, 57 lack a direct candidate | Explicitly queued; a historical move proves continuity at that commit, not current behavior parity |
| Current uncommitted product work | Five tracked UI changes and `ui/test/team-api.test.ts` | Preserved separately; not merged, committed or declared complete by this audit |

## Every branch, checkpoint and stash has a disposition

[source-disposition.csv](source-disposition.csv) is the per-source action ledger.
[sources.csv](sources.csv) supplies full object IDs and source kinds;
[source-summary.csv](source-summary.csv) supplies exact comparison counts.
The following groups are organizational only: no member is omitted from those ledgers.

| Source group | Exact coverage | Action |
| --- | --- | --- |
| Main / origin main / origin HEAD | Three refs at `7976eae` | Audited destination; retain pending work separately |
| Retained deployment branch / PR 3 | `b09dbc3` | Already in main ancestry. No redundant merge or restore |
| PR 1 | Head `9247013`, merge `6ef46d1` | Review retained sequencing requirements against current ROADMAP; older planning does not override current owner direction |
| PR 2 | Head `b2d0f7b`, direct merge `3737b14` | Closed PR status is not absence: direct merge history exists. Compare scaffolding requirements and current implementation via R2-R7 |
| Renamed temporary baseline branch | `temp-public-baseline` became main; root `6693b60` | Exact tree matches recovered development tip. Do not merge disconnected history into main just to repair ancestry |
| Pre-reset development history | Tip `6c4e00f`, including 95 previously uncovered ancestors | Now fully represented in source ancestry and commit-change inventory. Reuse retained implementations; review earlier removals |
| Checkpoints `996cab63-4262-40d1-a100-e2dbadd38e39` | Turns 0, 1, 2, 3; four individual refs | Earlier portal/control-plan work: R3, R5, R7. Compare each recorded version, not only turn 3 |
| Checkpoints `bb184991-7aa9-4c37-8965-b745bce9a3e9` | Every turn 0 through 24; 25 individual refs | Platform/portal/schema work: R2-R7. Preserve designs and tests removed before final baseline |
| Checkpoints `c279fa4c-aaa1-47e9-b41a-00203f1437ed` | Turns 0, 1, 2; three individual refs | Later feature/UI changes: R2-R7. Do not assume synthetic checkpoint commits were merged |
| Tags | v0.1.0 `92c363b`, v0.1.1 `e7a783c`, v0.1.2 `6b49bc1`, v0.1.3 `3dbf1e5` (peeled commits) | Historical capability evidence; compare differences, do not retag or deploy |
| Dropped stash 1 | `404d276`, index `6a65cc3`, untracked `d4a58fa` | R1 content accounted for below; do not apply |
| Dropped stash 2 | `3da0551`, index `b0abef7`, untracked `6183274` | Same content as stash 1; do not duplicate work |
| Both stash bases | `98a159c`, a child of public root `6693b60` | Already in main ancestry. This is a **post-reset** base |

### R1: recovered stashes — content accounted for

Both worktree trees are identical. Both index trees equal their base, so neither
contains an additional staged change. Their tracked edits fix the SMTP package
version in Cargo.lock and the malformed license string in crates/smtp/Cargo.toml;
both files match audited main exactly. There is nothing to apply from those edits.

Each untracked parent contains the same five A2UI files. A2UINode.svelte and
A2UISurface.svelte match main exactly. model.ts, types.ts and a2ui.test.ts are
present with formatting/import-order changes; their diffs were inspected.
The original types.ts blob also exists at platform/web/src/lib/a2ui/a2ui-types.ts.
These stashes contain no identified missing implementation. Runtime behavior was
not retested as part of this documentation audit.

### Existing feature work to retain and finish

The recovered history identifies real feature additions before the branding edits.
The following commits are concrete entry points for R2/R3, not assertions that
commit subjects alone prove independent authorship or complete runtime parity.
Paths below are current reuse targets under crates/common/src/shippin/ unless noted.

| Existing work | Historical implementation entry point | Reuse target / remaining cut |
| --- | --- | --- |
| Statistics/dashboard | `01e9504` | stats.rs and Svelte dashboard; D1/D3 |
| Live tracing and retained traces | `550792a`, `fb80191` | trace_history.rs, HTTP trace.rs and admin tracing; D3 |
| Alerts | `f2bcb86` | HTTP shippin/alerts.rs and services/alerts.rs; D3 |
| Deleted mail/account recovery | `d19efb9`, `835cab0` | recover.rs and email/message/recover.rs; D5 |
| Masked addresses | `cdce0e8`, `835cab0` | masks.rs and HTTP masked.rs; D4 |
| Organisations, quotas, panels | `bf19d86`, `30abd0f`, `833b91b` | orgs.rs/resources.rs and HTTP org/panel; D2/D9 |
| Domain branding | `f7b6953`, `a8fc0e3` | brand.rs and Svelte organisation branding; D6 |
| Domain identity-provider sign-in | `41755c5` | sso.rs and HTTP SSO; D2 |
| Calendar email templates | `3dd21d6` | calendar_templates.rs; D6 |
| SCIM provisioning | `0360927` | provisioning.rs and HTTP provisioning; D8 full-operation comparison |
| AI settings/assistant | `f01fc31` | ai.rs and HTTP assistant.rs; D7 spam/script caller integration |
| Service keys and MCP | `444721b`, `a9cd45b`, `4a36f08` | keys.rs and HTTP keys/mcp; preserve security and approval behavior through D10 |
| Shared-domain delivery | `899812d` | split.rs and delivery callers; retain protocol proof |
| People and teams | `4bd18d4` | Svelte shippin/people plus pending API diff; D2/D10 |
| Webmail, Drive, Docs, calendar, contacts and Google | `1bf18d5`, `ba8108e`, `a59877a`, `321d2b7`, `0bef929`, `c7a4f6d`, `23c5813`, `0a5e3e9` | Existing adapters/screens and Rust operations; retain the full workflows in A4 |

`2b2c852` moved the existing server modules into shippin folders; `9e0f5fe`
moved the admin/dashboard UI. [recorded-renames.csv](recorded-renames.csv) records
101 rename detections from those two commits, including destination existence and
similarity. This establishes 82 of the absent historical paths' moves. Those files
are not presumed lost and should not be rewritten merely because their old paths
disappeared.

## Absorption work packets — no new product work before reconciliation

The detailed inputs are [review-items.csv](review-items.csv) and
[missing-paths.csv](missing-paths.csv). Each historical version has a packet,
example commit and blob; [commit-changes.csv](commit-changes.csv) retains every
commit association. Review the latest retained form first, then resolve earlier
distinct versions. Do not review repeated snapshots as separate features.

Each open row must finish with: retained destination or required recovery,
behavior/requirement preserved, evidence, affected capability IDs, and its required
verification. A superseded version needs a concrete equivalent or documented reason;
“newer main” alone is insufficient. Necessary recovery is planned before new feature
work. No blanket checkout, merge or stash application is the integration strategy.

| Order / packet | Inputs and responsibility | Required disposition / acceptance |
| --- | --- | --- |
| R0: evidence and preservation | All 40 refs, 53 sources, 252 commits, reflog and six pending product files | Inventory complete. Before any later cleanup, preserve unreachable development/stash history under archival refs or a verified local recovery bundle; no pruning meanwhile. This audit does not create archival refs or mutate Git history |
| R1: stash reconciliation | Two stash worktree/index/untracked sets | Content comparison complete as above; retain current implementations, no stash application |
| R2: engine and feature history | 1,561 review units under crates/ and tests/; recovered own-feature commits and renamed/edited EE code | Map actual own implementations and their callers into D1-D9, identify remaining inherited implementations per file, retain protocol/security/storage behavior and original test assertions. Header/name-only changes remain unresolved authorship evidence |
| R3: Svelte and workflows | 1,164 review units; portal/admin moves, dashboard, organisations, provisioning, people, webmail, A2UI, removed control-plan screens | Compare old and current workflows, permissions and failure paths. Attach every removed behavior to a retained equivalent or recovery task. Current Svelte source is the base; no new UI rewrite |
| R4: already imported platform code | One non-identical review unit under platform/; retained import ledger and current UI | Reconcile the recorded difference and distinguish source retained in this repo from working integration. No neighboring-repository audit; preserve any missing integration as explicit work |
| R5: build, storage and installation | 136 review units; Cargo, Docker, deploy and scripts | Reconcile configuration/build differences against PostgreSQL-first, FoundationDB-at-scale and retained storage versatility. Preserve valid existing install/recovery work. No installer implementation before source reconciliation |
| R6: schema and contract | 17 review units; API/schema sources | Map old contract operations to the owned /api/v1 contract and generation inputs. Preserve capabilities without keeping upstream private endpoints. Feed concrete differences into D10 |
| R7: plans, provenance and assets | 314 review units; roadmap records, historical requirements, source notices, assets and removed documents | Carry valid requirements into the single roadmap; explicitly supersede conflicting directions. Record origins and retained notices; removed labels are not replacement implementations. Do not restore old conversation records as active instructions |
| R8: pending consumer checkpoint | Six product files listed in product-freeze.csv and the People/API plan | Review against the reconciled requirements; retain the current diff. Only after reconciliation and resumption may it receive its own bounded implementation commit. Existing web results do not certify live parity |

### Concrete earlier removals that must not disappear from the plan

- `35db53c` removed DeploymentPlanner.svelte, WorkspaceDeployPanel.svelte,
  control-plan.ts, its tests and story, and simplified PlatformControl.svelte.
  The old model described managed/self-host modes, lanes, targets and tier limits.
  Current PlatformControl is a read-only porting ledger, **not a deployment
  equivalent**. R3/R5 must preserve usable requirements/design and specify the real
  operations needed; do not restore simulated controls and call hosting complete.
- `01e9504` replaced the old dashboard/metrics proof scripts with own statistics
  and Svelte dashboard work. R2/R3 must reconcile the old measured surfaces and
  checks with current stats/metrics; deletion of a script is not proof that its
  assertions are still covered.
- `23c5813` removed adapter/groupware.ts during contacts work. R3 must reconcile
  its operations against current contacts/calendar adapters and protocol tests.
- Many old catalog/portal and common/http module paths now have shippin paths.
  All 152 absent historical paths have individual candidate/review rows. Names
  suggest destinations; behavior and source history decide whether they are retained.
- `953c5de` mixes license-string edits with actual TLS UI additions. Preserve the
  TLS work; do not classify the whole commit as either a rewrite or a mere rename.
  `6c4e00f` removes license markers from, for example, common/config and composite
  storage while retaining the implementation. Those edits do not demonstrate a port.

## Smallest implementation sequence after this gate closes

1. Resolve R2-R7 decisions and recovery requirements. Retain the own code already
   present and finish the historical reconciliation before resuming product edits.
2. Checkpoint R8/A0 separately. Complete D1 with D2/D3/D9: owned feature startup,
   directories/quotas, telemetry and storage configuration. PostgreSQL is the first
   test profile, FoundationDB the scale profile; no edition-triggered downgrade.
3. Finish D4-D8 by connecting the existing masks, recovery, calendar/brand, AI and
   provisioning implementations and supplying only their identified behavior gaps.
4. Finish D10: owned schema/generator and direct Rust application operations for
   Svelte/MCP. A REST wrapper around private JMAP is an intermediate implementation.
5. Close D11 and confirmed application-source replacement rows alongside those
   changes. Source history must justify each replacement; no arbitrary percentage
   of changed code and no whole-engine rewrite inferred from a squashed baseline.
6. Run the complete capability/protocol/storage proofs; then package PostgreSQL-first
   self-hosting and the FoundationDB scale option as the same product. Webmail stays
   in the baseline. Campaign services/listmonk and later upstream-inspired fixes
   follow separation, using the owned API and existing Svelte design system.

## Evidence files and limits

- [refs-raw.txt](refs-raw.txt), [reflog.txt](reflog.txt): available reference evidence.
- [commits.csv](commits.csv), [commit-ids.txt](commit-ids.txt): all stored commits,
  parents, subjects, main ancestry and source-inventory coverage.
- [content-differences.csv](content-differences.csv): every non-identical entry from
  the 53 source trees; equality compares path, mode and blob.
- [commit-changes.csv](commit-changes.csv): every commit's first-parent changes
  (root trees included), including deletions and intermediate versions.
- [source-disposition.csv](source-disposition.csv), [review-items.csv](review-items.csv),
  [missing-paths.csv](missing-paths.csv): source actions and detailed review queue.
- [recorded-renames.csv](recorded-renames.csv): observed server/UI moves with commit,
  similarity and current destination evidence.
- [product-freeze.csv](product-freeze.csv), [starting-status.txt](starting-status.txt):
  starting uncommitted state. Product hashes must remain unchanged during planning.

Inventory coverage is complete **for the objects available at this audit**.
Unavailable/pruned objects or deleted remote branches with no retained object cannot
be reconstructed from this evidence. Metadata CSVs are not backups of Git objects.
Content continuity is established; complete behavioral absorption and independent
source authorship remain open. No tests, deployments, merges or product edits are
performed by generating these reports. No release readiness is claimed.
