# Windows Agent R Script Launch Repair

Status: active; ISSUE-2-AGENT-LAUNCH-1 authorized 2026-08-10; launcher and
aisdk-readiness repairs implemented; exact updated-main automated verification
and owner desktop Agent acceptance passed; ISSUE-2-AGENT-LAUNCH-R1 fresh
readiness admission authorized by the project owner's 2026-08-10 repair
instruction and implemented; exact `0.4.0-dev.29` verification and PR update
passes; upstream integration completed in merge `f05315c`; exact installed
Windows acceptance remains pending

Date: 2026-08-10
Authorization: user requested a fix and pull request for upstream GitHub Issue
#2 on 2026-08-10
Change class: D1 narrow defect correction
Risk: R2 desktop-to-Agent-R process-launch boundary
Work package: ISSUE-2-AGENT-LAUNCH-1
Owning contracts:
`implemented-windows-startup-diagnostics-and-recovery-design.md` and
`implemented-agent-llm-configuration-design.md`

## Problem And Evidence

On Windows, Model settings can complete a real `aisdk::generate_text()` probe,
while every desktop Agent turn fails before authentication. The failed child
process exits with Windows status `0xc0000005`, empty stdout and stderr, and no
`[rho-agent-startup] script_started` trace. Rho then reports the secondary
authentication timeout as a generic connection failure.

The desktop Agent launcher passes its complete multi-line R program as one
native command-line argument using `Rscript -e`. Passing the same program
through a short R wrapper reaches all pre-connection startup traces, which
isolates the failure to Windows command-line transport rather than R syntax,
the selected model, credentials, or provider network access.

Owner testing of the launcher repair removed `0xc0000005` and reached all four
pre-connection startup traces. It then exposed a second admission defect:
Rho's optional Agent probe accepted installed `aisdk` 1.4.12 because the
package loaded, although Rho's pinned 1.5.0 contract requires exported
`normalize_capability_model_routes()` and `set_run_trace_sink()`. A basic
`generate_text()` connection test can therefore succeed while the complete
ChatSession runtime is incompatible.

Regression invariant: desktop Agent R code is transported in a UTF-8 temporary
`.R` file, never as a multi-line `-e` argument, and that file remains alive
until the Agent R child exits.

Readiness invariant: Rho marks Agent R available only when the installed aisdk
meets the pinned minimum version and exports the APIs used by a complete turn;
an older loadable package remains an actionable runtime setup failure.

## Contract

- Write `desktop_agent_turn_script()` verbatim to a uniquely named temporary
  file with an `.R` suffix and flush it before spawning Rscript.
- Invoke `Rscript <script-file> <port> <agent-package> <mode>`; do not include
  `-e` or the script body in native process arguments.
- Keep the temporary-file owner in `run_agent_turn()` scope until the spawned
  child has exited or been killed and reaped.
- Preserve stdin transport for the bootstrap token, redacted runtime profile,
  and complete model prompt.
- Preserve the existing child environment, authentication timeout, turn
  timeout, cancellation, broker identity, approval, and result semantics.
- A temporary-file create/write/flush failure is a visible pre-spawn Agent
  failure. The file is removed automatically after the turn scope ends.
- No provider, credential, model routing, schema, persistence, project,
  approval, frontend, or public protocol behavior changes.
- Improving the generic frontend classification of process crashes is useful
  follow-up work but is outside this repair.
- The optional Agent runtime probe requires `aisdk >= 1.5.0` and verifies the
  two Agent-only exports before declaring the runtime available.
- An incompatible probe retains the detected aisdk version and names the
  required version or missing API. Workspace R remains available.
- Model connection tests remain provider/model connectivity checks. They do
  not override the separate complete-Agent runtime readiness gate.
- Rho does not automatically install or modify the user's R package library.

## Cross-Review

- The implemented Windows startup specification already requires desktop-owned
  multi-line R probes to use UTF-8 temporary `.R` files. This repair closes the
  remaining Agent-turn launcher deviation without changing R discovery or
  startup capability policy.
- The Agent LLM configuration contract retains provider admission, credential
  injection, runtime-profile routing, and Connection/Model/Capability tests.
  This repair occurs before those Agent-turn semantics execute.
- The Agent conversation-concurrency contract retains exact conversation/turn
  identity, cancellation, scheduling, and persistence. The temporary script is
  process-local and contains no conversation content or credential value.
- No ownership, schema, security-policy, persistence, or sequencing conflict
  remains.

## ISSUE-2-AGENT-LAUNCH-R1 Fresh Readiness Admission

The general R/Ark runtime cache is not authoritative for Agent compatibility.
An `aisdk` installation can be upgraded, downgraded, or replaced independently
of the cached Rscript, Ark, profile, and environment file signatures. Reusing a
previous positive `agent_runtime` value would therefore create a startup race
in which one Agent turn could be admitted before the asynchronous fresh probe
rejects the current package.

The accepted repair contract is:

- remove `agent_runtime` from the persisted `RuntimeCacheFile`; an older
  cache carrying that JSON member remains readable only for its R/Ark metadata,
  and the member is ignored;
- both cache-hit and cache-miss startup paths expose one shared deferred,
  unavailable Agent status until `agent_runtime_retry` completes a fresh
  probe against the current R library;
- no Agent turn may observe a cached positive readiness value; Workspace R may
  still use the existing validated cache and start without waiting for aisdk;
- fresh-probe success and failure continue to update only the current in-memory
  startup/runtime views. Manual Retry remains the recovery path and no R
  library mutation is introduced;
- add a backward-compatibility regression that injects a legacy positive
  `agent_runtime` member, proves the cache still loads its R/Ark facts, proves
  newly serialized cache state omits that member, and proves deferred status is
  unavailable with no version claim;
- retain the exact aisdk version/export regressions, Windows script-file
  launcher tests, prompt/credential stdin isolation, and complete affected
  validation.

This changes no database schema, project state, credential source, Provider
request, model route, approval, Agent turn identity, or R package contract. The
cache is machine-local performance state, not product persistence authority.
The System Credential specification retains model-readiness ownership; the
Windows startup specification retains R/Ark cache ownership; this repair only
removes an invalid overlap between them.

## Acceptance And Verification

- A deterministic Rust regression test proves the generated script file has an
  `.R` suffix, contains the exact UTF-8 Agent program, and process arguments
  contain only its path plus the existing port/package/mode values.
- The existing large-prompt transport test proves prompt and provider-secret
  names remain absent from process arguments and prompt content remains on
  stdin.
- Runtime-probe regressions prove loadable aisdk 1.4.12 is unavailable with its
  version retained, while compatible 1.5.0 is admitted only after the required
  exports are checked.
- A legacy-cache regression proves a persisted positive Agent status cannot
  enter current startup admission before the fresh probe.
- Run focused `rho-server` tests, formatting, the affected workspace tests,
  both R package suites, JavaScript syntax, and `git diff --check`.
- Build and launch the updated Windows desktop application. The owner manually
  verifies a real Agent turn with a Model-settings-tested profile before any
  push or pull request is created.

## Version, NEWS, And Stop Point

The project owner's instruction to fix the existing upstream pull request
authorizes this user-visible source correction as the single-use
`0.4.0-dev.29` integration identity. Application metadata, workflow defaults,
release fixtures, cache identity, and `NEWS.md` are synchronized. The R package
versions, database schema, and public protocol are unchanged. This does not
authorize candidate construction, installation, merge, publication, or update
site mutation.

Stop after this repair, exact-source regression evidence, contract review,
scoped commit, and update of PR #24. Exact installed `dev.29` Windows acceptance
remains an explicit release gate. Do not expand into provider-specific behavior
or broader Agent diagnostics.

## Implementation Evidence

The implementation writes and flushes the complete desktop Agent program to a
uniquely named temporary `.R` file, passes only that path plus the existing
port/package/mode arguments to Rscript, and retains the file owner throughout
the child lifecycle. The existing token, runtime profile, and prompt remain on
stdin.

The optional Agent probe now requires `aisdk >= 1.5.0`, checks the two exported
APIs used only by complete Agent turns, and retains a detected incompatible
version in its unavailable status. No R library is installed or mutated.

The first owner run of this readiness gate exposed and rejected an incorrectly
qualified `utils::package_version()` call before any Agent turn. The corrected
probe uses base R's `base::package_version()` explicitly, and its regression
contract rejects reintroducing the invalid namespace qualification.

On updated fork `main` commit `a0c23a3`, the launcher, large-prompt, and
aisdk-readiness focused regressions passed. The complete Rust workspace passed
from an isolated Windows MSVC target directory, as did Rust formatting,
JavaScript syntax, and diff checks. The repository's documented Windows GNU
target remains unavailable locally because Rtools45 is not installed. The two
R package suites remain unrun because this R library does not contain
`testthat`; neither R package source changed.

Post-verification review found no change to provider requests, credentials,
model routing, Agent stdin, broker authentication, conversation identity,
cancellation, persistence, approval, schema, or frontend protocol. The branch
remains active until pull-request review and integration. At that initial
checkpoint no version or release decision changed; the later authorized R1
slice below allocated and synchronized the single-use `dev.29` source identity.

ISSUE-2-AGENT-LAUNCH-R1 removes `agent_runtime` from the serialized general
R/Ark cache and uses the same unavailable deferred status on cache hits and
misses. The background and manual retry paths remain the only fresh readiness
writers, and they update current in-memory startup/runtime state. A legacy-cache
regression injects an incompatible positive aisdk 1.4.12 status, proves Rscript,
Ark, R version, and R library facts still load, proves current serialization
omits Agent readiness, and proves startup remains unavailable until a fresh
probe.

On the synchronized `0.4.0-dev.29` source, Rust formatting and complete
workspace check/tests pass: desktop 176 passed with one opt-in native Keychain
smoke ignored, server 59 passed, and store 108 passed. Both R package suites,
JavaScript syntax, all 52 frontend/release-contract scripts, version/release
fixtures, and `git diff --check` pass. Focused legacy-cache, pinned-aisdk, and
desktop script-file transport regressions pass. Post-verification contract
review confirms that legacy extra JSON fields remain readable, cache hit and
miss share the same fail-closed admission, and the temporary Agent script owner
remains live through child completion.

Owner acceptance used the refreshed `0.4.0-dev.27` debug application with the
exact pinned aisdk 1.5.0 commit installed in the selected R library. The Agent
runtime probe completed and a configured model returned an Agent response. The
earlier Windows access violation and the later incompatible-aisdk startup
failure did not recur. Separately observed panel-wide focus movement is outside
this work package and neither blocks nor expands Issue #2. This evidence remains
valid for the original launcher/readiness correction but is not exact installed
`dev.29` acceptance; that release gate remains open.

Upstream source integration completed on 2026-08-11 through PR #24 at merge
commit `f05315cde735f011a76c1400bdadd28d21acc57e`. Issue #2 closed from the
PR's source-fix relationship. This establishes only the reviewed `dev.29`
default-branch source identity; no candidate, artifact, installed acceptance,
MAC5, publication, or update-site fact was created or inferred.

## 2026-08-14 extension: rho-server CLI probes close the same transport gap

The desktop interactive paths (Agent turn, connection test, Agent runtime
probe, model probes) already transported Agent R code in flushed UTF-8
temporary `.R` files. Two rho-server CLI probe subcommands still passed their
multi-line Agent R programs through `Rscript -e`, the exact pattern this
repair eliminated (PR #24 converted only the turn launcher). On 2026-08-14,
while investigating a Linux DeepSeek reachability report, both remaining
launches were converted to the same invariant:

- `crates/rho-server/src/coordinator.rs`: `probe_coordinator` now launches
  `coordinator_probe_script()` from a `write_coordinator_probe_script()`
  temporary `.R` file via `coordinator_probe_args(...)` — no `-e`;
- `crates/rho-server/src/main.rs`: `probe agent-r` now launches
  `agent_r_probe_script()` the same way — no `-e`;
- regression tests assert a flushed UTF-8 `.R` file is used and no `-e` or
  script body appears in the native process arguments (mirroring the
  `desktop_agent_script_uses_a_flushed_utf8_r_file_instead_of_inline_e`
  test);
- the temporary-file owner stays in scope until the spawned child is reaped
  in both functions.

Verification on 2026-08-14: `cargo build -p rho-server --locked` and
`cargo test -p rho-server --locked` pass (60 lib + 1 bin tests, including the
two new regressions); `cargo fmt --all -- --check` clean. No provider,
credential, model-routing, schema, persistence, project, approval, frontend,
or public-protocol behavior changed; no version or NEWS change (CLI
diagnostics only, no user-visible application behavior).

## 2026-08-14 resolution of the Linux DeepSeek reachability report

The reported "cannot reach the service" failure was reproduced with the
app's exact R-side connection-test path and traced to an environment gap, not
an application defect: the reporting R library had `aisdk` 1.5.0 but was
missing the pinned `aisdk.providers` 0.1.0 (exact commit
`5cf315e5eedad7d83b224c96595da346e1192a85`), which the active credential
spec and README require for DeepSeek and the other registered providers.
`rho_test_model_profile` then failed at the provider-admission gate with the
actionable message "The selected provider requires aisdk.providers 0.1.0 or
later..." and `error_class` `provider`, before any network call. The Rust
model-discovery endpoint and the R-side call both reach
`api.deepseek.com` (401 with a dummy key) once the package is present.
Remediation: `remotes::install_github("YuLab-SMU/aisdk.providers@5cf315e5eedad7d83b224c96595da346e1192a85")`
into the R user library (also installable from Rho's environment panel). No
source change was required; no version or NEWS impact.
