# shippinTECH alpha execution plan

Owner direction, 2026-11-21. **This is the single execution order.** It supersedes
older sequencing in `PLAN.md`, porting work packages, dated plans and Cursor rules.
[REGRESSION.md](REGRESSION.md) defines acceptance; [RELEASE.md](RELEASE.md) controls
tags. This is pre-release engineering on disposable installs.

## 1. The result being built

One shippinTECH product: the existing Svelte UI and style tokens drive owned Rust
application services through `/api/v1`. PostgreSQL is the initial database.
FoundationDB remains the scale option behind the same application/storage boundary.
Other existing storage capabilities remain available. The choice of database must
not change which mail, admin, security or collaboration features are available.

All existing capabilities remain in scope, including capabilities previously sold
as Enterprise. There is no permanent upstream private API, compatibility shim or
old UI requirement. Fresh pre-release installs are allowed; preserving upstream
private API shapes and historical data layouts is not an acceptance requirement.
Standard SMTP, IMAP, POP3, JMAP, DAV, ManageSieve, OAuth and SCIM interoperability is
part of the product. New shippinTECH data still needs durability, backup and recovery.
PostgreSQL-to-FoundationDB transfer is a product scaling capability, not upstream
compatibility work.

Reuse the Svelte and shippinTECH Rust implementations already written. Port the
missing behavior and reconnect callers; do not restart those features from zero.
Do not reduce capabilities by compiling out Enterprise, substitute successful
stubs, or count renaming as a completed port. Permitted third-party libraries can
remain dependencies with their notices; ownership of the product does not require
rewriting every library authored by someone who also worked upstream.

```mermaid
flowchart TD
  UI[Svelte screens and existing style tokens] --> API[shippinTECH API /api/v1]
  Agent[shippin.ai and approved MCP actions] --> API
  API --> Rust[Rust application operations and authorization]
  Protocols[Mail and groupware protocol adapters] --> Rust
  Rust --> Store[Shared persistence and durable jobs]
  Store --> PG[PostgreSQL: initial install]
  Store --> FDB[FoundationDB: scale profile]
  Store --> Other[Existing blob, search, cache and coordinator options]
  Rust --> Services[Service adapters: campaigns and future services]
```

## 2. Verified starting point

This table records observations from this session, not a claim of alpha completion.

| Item | Observed state |
| --- | --- |
| Main | Local and remote `main` both at `a99915375d94081b17484ce4a652b0e04dff9194` before this planning change. |
| Other remote branch | `cursor/production-deploy-parity-c403` at `b09dbc3` is **8 commits behind main, 0 unique commits** (`git rev-list --left-right --count`). Its commits are already in main; no merge is required. No branch was deleted. |
| Pending work | Five tracked UI files plus new `ui/test/team-api.test.ts` and its plan record. This work is not committed or deployed. No Rust/storage edits are pending from this session. |
| UI checks | `npm test`: **41 files, 289 tests passed** after the People/API boundary changes. Svelte check passed with 0 errors/warnings before the final boundary guard; rerun it in A0. |
| Main CI | [Check for a999153](https://github.com/shippin-tech/shippinTECH/actions/runs/44843695571) passed. That does not verify the uncommitted candidate. |
| API | Actions, account security and bootstrap use `/api/v1/registry` on main. Pending People/teams work moves its consumer too and rejects private JMAP fallback. `crates/http/src/api/registry_rest.rs` still constructs private JMAP requests internally. |
| Existing own Rust | `crates/common/src/shippin/` contains AI, branding, calendar templates, Google, keys, masks, organisations, provisioning, recovery, resources, split delivery, SSO, statistics and trace history. Own alert paths also exist in `crates/http/src/shippin/alerts.rs` and `crates/services/src/alerts.rs`. |
| Remaining EE plumbing | `enterprise/license.rs` always refuses validation/renewal. `enterprise/config.rs`, account-creation checks and the scheduler still consume it. Some old paths can therefore be unavailable; the exact affected feature callers need the A2 mapping. |
| Storage | PostgreSQL and FoundationDB backends exist. Cargo defaults are `rocks,enterprise`; `Dockerfile.fdb` builds `scale,enterprise` without PostgreSQL. Local Compose requires FoundationDB. PostgreSQL-first is not implemented. |
| Live verification | Current candidate has not passed live browser/contract, protocol or storage parity. The temporary parity containers are stopped. Historical ledger results are not current-image results. |
| Source accounting | `PROVENANCE.md` compares a squashed baseline. Its roughly 96% Rust classification is **not proof of upstream authorship or a reliable estimate of remaining rewrite work**. Preserve already rewritten work while resolving actual origins. |

## 3. Execution order and completion gates

Work on `main`, with bounded commits and their evidence. No PRs, release tags or
unrelated refactors. Each package ends with a reviewable result and an updated
capability report. Do not move an unverified row to Done.

### A0 — Finish and commit the pending Svelte API consumer

**Files:** `ui/src/lib/shippin/people/team.ts`, `ui/src/lib/contract/registry.ts`,
`ui/src/lib/adapter/{registry-facade,jmap,action-call,bootstrap-call}.ts`, associated
`ui/test/`, `ui/test-live/`, `ui/scripts/prove-portal-{actions,bootstrap}.ts`.

1. Review the pending diff: paged people/domain reads, id-only account creation,
   group membership, deletion, stale-write errors and credential field patches.
2. Remove obsolete private-call builders and update their proof scripts. Keep
   standard protocol adapters. Unknown admin objects must fail without fallback.
3. Run the full web gate, not just the new tests. Commit the validated pending API
   work separately from this plan. Its live verification stays open until A1.

**Done:** all applicable web checks pass; active Svelte admin operations use the
owned API; no private transport fallback; commit ID and remaining live checks recorded.

### A1 — Make PostgreSQL the actual runnable baseline

**Files:** `crates/main/Cargo.toml`, `Dockerfile`, `Dockerfile.fdb`,
`deploy/local/{compose.yaml,start.ps1,bootstrap.py,refresh.ps1,README.md}`,
`install.sh`, `deploy/{compose.yaml,server.env.example,README.md}`;
existing `crates/store/src/{build,registry,backend/postgres}` and
`crates/common/src/manager/{boot,defaults}.rs` only where boot wiring requires it.

1. Add PostgreSQL to the common shipped build; retain FoundationDB and existing
   optional stores. Set PostgreSQL as the new-install default with explicit scale
   selection. Do not create a second engine or a different feature edition.
2. Generate a valid PostgreSQL store configuration at install time, resolve secrets
   through existing mechanisms, and use the existing first-run administrator flow.
   Startup must select the requested store explicitly and fail clearly if unavailable.
3. Local default starts PostgreSQL without requiring FoundationDB. Keep an explicit
   FoundationDB scale profile with the same UI, API, mail behavior and service options.
   Update install docs and scripts together; a compiled driver alone is insufficient.
4. Run first-run, admin/People/password, SMTP-to-webmail and restart tests on the exact
   candidate. Capture commit, image ID, backend, fixtures and results. Record a usable
   baseline for subsequent Rust ports; failures are fixes to make here, not waivers.

**Done:** a fresh disposable PostgreSQL install reaches every existing feature surface;
its live suites run; restart retains synthetic mail/settings. FoundationDB support
still builds. No FDB repair detour is required to establish the PostgreSQL baseline.

### A2 — Finish the existing shippinTECH feature ports

**Files:** `crates/common/src/shippin/`, `crates/http/src/shippin/`,
`crates/services/src/alerts.rs`, their call sites in `email`, `groupware`, `directory`,
`store` and `http`; old `crates/common/src/enterprise/`,
`crates/services/src/task_manager/scheduler.rs`,
`crates/jmap/src/registry/mapping/principal.rs`,
`crates/common/src/cache/directory.rs`, `crates/main/src/main.rs`.

For each row: identify active callers, route them to the existing own implementation,
port missing behavior, run the stated real workflow, then remove the superseded path.
Remove dead license validation, renewal scheduling and licensed-account caps as part
of that cutover. Preserve real role, tenant, abuse and quota limits. Merely making
`is_enterprise_edition()` return true is not completion.

| Feature | Existing implementation to extend | Behavior that closes the row |
| --- | --- | --- |
| Metrics, traces, alerts | `shippin/stats.rs`, `shippin/trace_history.rs`, own alert modules | Live and stored data, retention, filters, email/webhook delivery; one collector and scheduling path, no duplicate alerts |
| Recovery and masked addresses | `shippin/recover.rs`, `shippin/masks.rs` | Delete/restore with bytes and permissions; mask creation, delivery, disable and all previously supported uses |
| Tenancy and identity | `shippin/orgs.rs`, `shippin/sso.rs`, `shippin/provisioning.rs`, own SCIM route | Tenant walls, delegated administration, quota enforcement, domain directories, provisioning and refusal behavior; current OIDC/SCIM subset must not hide missing baseline operations |
| Branding and calendar templates | `shippin/brand.rs`, `shippin/calendar_templates.rs` | Domain branding, reminder/invitation content and RSVP behavior through the actual mail/groupware callers |
| AI, spam and Sieve prompts | `shippin/ai.rs`, `spam-filter`, `common/src/scripts` | Existing provider configuration drives classification and Sieve prompts; timeout/failure/disabled behavior tested; assistant success alone does not close this |
| Storage features | `store/src/backend/{postgres,foundationdb,composite}`, `coordinator` | SQL replicas, blob/cache sharding and cluster behavior remain available with correct writes, reads, quotas and recovery |

**Done:** every remaining old feature consumer has an owned replacement and passing
behavior evidence. No license stub, renewal task or upstream entitlement is needed
to exercise the supported features. Reuse historical F0–F11 tests, rerun against A1.

### A3 — Own the Rust contract and remove private JMAP dispatch

**Files:** `crates/http/src/api/{v1,registry_rest}.rs`, `crates/jmap/src/registry/`,
`crates/registry/src/{schema,types}`, `resources/schema/schema.json.gz`,
`api/{DESIGN.md,v1/openapi.yml}`, `ui/scripts/emit-registry.ts`,
`ui/src/lib/contract/`, `mcpl/`.

1. Use the existing `/api/v1` operations as the starting contract. Define checked-in
   shippinTECH schema sources and reproducible generators. Preserve capabilities,
   required fields, patch semantics, errors and concurrency; private wire shapes may change.
2. Implement the application operations directly in Rust: validation, authorization,
   transactions, service calls and auditing. `/api/v1/registry` must stop constructing
   `x:*` JMAP requests. Standard JMAP and other protocols call the same domain operations.
3. Generate the Svelte client, Rust models, OpenAPI and MCP action declarations from
   owned sources; move every product caller in the same slice. Do not maintain two APIs.
4. Remove private extension routes, unused schemas and bundles after their callers
   move. Test refusal/absence of retired routes and ordinary protocol interoperability.

**Done:** a clean checkout regenerates contracts; UI and MCP invoke the same authorized
operations; admin execution does not dispatch through the old private JMAP layer;
full CRUD/actions/security tests pass, including partial credentials and stale edits.

### A4 — Close full Svelte and protocol capability parity

**Files:** `ui/src/lib/shippin/`, `ui/src/lib/portal/`, `ui/src/lib/token/`,
`ui/src/routes/`, `ui/porting/`, `ui/e2e-live/`, `ui/test-live/`,
`deploy/local/verify*.py`, `tests/src/{smtp,imap,jmap,webdav,scim,system,store,cluster}`.

Use the coverage table below. Finish actual missing workflows: Sieve editor and
activation, typed settings where generic CRUD loses behavior, webmail search,
independent IMAP/POP3/DAV/ManageSieve/OAuth checks, accessibility and mobile flows.
Retain webmail compose/reply/forward, drafts, attachments, calendars, contacts, files,
Docs, administration and existing integrations. Reuse style tokens; this is not a
visual redesign. Inventory entries need workflow proof, not just a reachable route.

**Done:** each existing capability has candidate-image evidence. Implement or explicitly
leave open unfinished promises such as the Microsoft server adapter; never silently
call a mock a real integration or remove the screen to make a test pass.

### A5 — Close the remaining source and dependency separation

**Files:** `provenance/`, `scripts/provenance.py`, `Cargo.toml`, `Cargo.lock`,
crate manifests, `crates/{smtp,imap,pop3,jmap,dav,scim,managesieve,email,groupware,
common,directory,store,coordinator,services,registry,types,trc,utils,nlp}` and protocol
crates; exact replacement files are selected by the source decisions below.

Produce a per-file/per-component decision: **existing shippinTECH work: keep;
permitted dependency: retain with notices; confirmed remaining inherited application
implementation: replace; uncertain origin: unresolved**. Validate earlier rewrites
before adding them to the replacement queue. A squashed commit or path name alone
cannot decide this. This accounting runs alongside A2–A4; it must not block A1.

The executable replacement order is: owned feature wiring (A2), private application
contracts/schema (A3), filtering runtime where still required, then remaining mail,
groupware, persistence and infrastructure components actually identified as inherited.
Each replacement gets its exact files, observable specification and affected suites
before coding. Replace only the identified component; retain the shared storage and
protocol behavior. `roadmap/independence.md` records those decisions and removal commits.

`sieve-rs` is a concrete remaining dependency boundary with user and MTA-stage
integration; retain the full extension/function/plugin inventory when replacing it.
Do not replace MIME, auth, parser or utility packages solely because of their author.
Any such dependency retained is attributed as a dependency, not claimed as our code.

**Done:** every shipped application source/generated artifact has a resolved origin;
all required replacements have passing behavior evidence and removal commits;
no forbidden upstream source, binary, runtime service or build download remains.
The present inventory does not yet establish how many files this requires. Removing
EE gates, translating to Svelte or changing headers cannot by itself close this gate.

### A6 — Prove a complete alpha and the PostgreSQL-to-scale path

**Files:** `scripts/{regression.py,engine-suite.sh}`, `.github/workflows/check.yml`,
`tests/`, `deploy/`, `roadmap/capabilities.csv` and its generator, `RELEASE.md`.

1. Make the regression runner execute the actual required matrix: PostgreSQL baseline,
   FoundationDB scale, existing supported-backend checks, both default and scale builds.
   Current runner only selects RocksDb/Sqlite and needs correction. Supply Docker/service
   prerequisites and `STORE`/`COORDINATOR`; timeouts, ignored and missing tests stay visible.
2. Run full UI, live contracts/browser, independent protocol and engine suites on a
   named candidate. Fix failures. Record historical results separately from this run.
3. Prove new-product backup/restore and PostgreSQL-to-FoundationDB transfer with synthetic
   messages, attachments, accounts, ACLs, keys and jobs. Pause writes for the first
   supported transfer if needed; verify counts/content and restart the same application.
   Do not promise automatic online scale migration before it is implemented and tested.
4. Exercise unavailable storage, exhausted disk, interrupted jobs and restart. Requests
   must have bounded failure, useful errors and health reporting, followed by recovery.
5. Record throughput, latency and memory fixtures and acceptance thresholds before
   evaluating the candidate. Build/run the server on Linux amd64 and arm64. The earlier
   desktop-commander platform request was withdrawn and is not a new client project.

**Done:** A0–A5 closed; all capability tests pass on the candidate or a specifically
identified unfinished feature remains a blocker; fresh install and scale transfer
work; release rows are reconciled with evidence. No global zero-regression or
independence claim is made from a green build alone. No `v*` tag before RELEASE closes.

### Commands and evidence locations

These are execution commands, not a claim that they have all passed. Rust scale
checks need the FoundationDB client/build prerequisites. Live commands require the
candidate stack and synthetic accounts; credentials stay in ignored local files.

| Check | Command / evidence |
| --- | --- |
| Whole web gate | From root: `python scripts/regression.py --only web`; on Windows use Git Bash for POSIX deploy checks, and report any POSIX-only check not run |
| Focused Svelte diagnostics | From `ui`: `npm run check`, `npm test`, `npm run build`, `npm run test:browser`; the full web gate also checks contracts, branding, inventory and MCPL |
| PostgreSQL build | `cargo check -p shippin-tech --locked --no-default-features --features "postgres enterprise"` |
| Shared scale build | `cargo check -p shippin-tech --locked --no-default-features --features "postgres scale enterprise"`; retain other supported feature checks |
| Live owned contract and browser flows | From `ui`: `npm run test:contract-live`, `npm run test:live`; root `python scripts/regression.py --only live` also runs the existing local verifier scripts |
| Engine matrix | A6 first fixes `scripts/regression.py` and `scripts/engine-suite.sh` to select PostgreSQL/FoundationDB and their Cargo features/services; then `python scripts/regression.py --engine`. The current command alone does not cover that matrix |
| Results | `REGRESSION-RESULTS.md`, `roadmap/regression-results.json`, per-run logs, and capability-specific fixtures; record commit/image/backend, passed/failed/ignored/not-run counts and failure causes |
| Source/contract closure | Regeneration checks plus a clean isolated build; inspect dependency graph and active routes for the specifically retired sources/methods. Do not use a branding string search as the ownership test |

## 4. Every capability has a destination

IDs refer to [the existing ledger](roadmap/capabilities.csv). Its `verified` statuses
are historical; A6 attaches current commit/image/backend results. All rows remain
in the floor. This table assigns work; it does not upgrade their status.

| Capability IDs | Required coverage | Packages |
| --- | --- | --- |
| M1, M2, M3, M4 | Delivery, submission, relay refusal, queue retry/failure reports, DKIM/SPF/DMARC/ARC, TLS policy | A1, A4, A5, A6 |
| M5, M6, I9 | IMAP flags/search/move/IDLE, POP3 retrieve/delete, OAuth mail-client login | A4, A5, A6 |
| M7, M8, M9 | Webmail read/thread/flags, compose/reply/forward/drafts, byte-exact attachments, live search | A1, A4, A6 |
| M10, C7 | Spam learning/classification, Sieve editor, runtime extensions and ManageSieve effects | A2, A4, A5, A6 |
| M11, M12, M13 | Masks, undelete and split-domain relay | A2, A4, A6 |
| C1, C2, C3, C4 | Calendar/repeats/invitations/templates, contacts, independent DAV round trips and ACLs | A2, A4, A5, A6 |
| C5, C6 | File operations, sharing and isolation, Docs editing/autosave | A3, A4, A6 |
| I1, I2, I3, I4, I5, I6, I7, I8 | Setup, credentials/MFA, directories/SSO, SCIM, tenants/roles/quotas, CRUD, signup/DNS, keys | A0, A1, A2, A3, A4, A6 |
| G1, G2 | Existing Google workflows and unfinished Microsoft connection; consent, bytes, expiry, revocation and isolation | A4, A6 |
| G3, G4 | AI/MCP approvals, declared operations and learning progress | A2, A3, A4, A6 |
| O1, O2, O3 | Metrics, tracing and alerts | A2, A6 |
| O4, O5, O6, O8, O9, O10 | Install, backup/restore, TLS, monitoring, product data durability, bounded storage failures; old-layout migration is not mandatory | A1, A6 |
| O7 | PostgreSQL first, FoundationDB scale, replicas/shards, blob/cache/coordination options | A1, A2, A6 |
| U1, U2, U3 | Full Svelte workflows, accessibility/mobile, performance evidence | A0, A3, A4, A6 |

## 5. After separation: improvements and additional services

Webmail is already part of alpha and is not deferred. Once the separation gate
closes, review upstream fixes as references: record the bug, affected behavior and
our reproducer, then implement and test the improvement in our architecture.
No upstream merge is the default integration mechanism.

Campaigns such as listmonk integrate behind a shippinTECH Rust service adapter and
Svelte screens, using the same identity, permissions, secrets, jobs and audit model.
Start with list/subscriber management, consent/unsubscribe/suppression, campaign
submission and delivery events. Keep campaign load isolated from transactional mail.
Evaluate the service's own dependency terms separately; integration does not make
its source shippinTECH-authored. Do not make campaigns a prerequisite for completing
the mail/API separation. shippin.ai uses the same declared operations, not a second
control plane.

## 6. Immediate next task and reporting rule

**Next implementation task: A0.** Finish the pending consumer changes, remove the
obsolete private-call builders, run the full web gate and commit that bounded result
to main. Then execute A1: PostgreSQL default install and a real candidate baseline.
No architecture selection or waiver answer is needed from the owner to start.

Each completion report contains: commit, files/behavior changed, commands/results,
current image/backend when applicable, and exact remaining blockers. Claims about
latest upstream superiority wait for a dated feature/benchmark comparison. The alpha
is measured against full capability coverage, not a reduced free-plan demonstration.
