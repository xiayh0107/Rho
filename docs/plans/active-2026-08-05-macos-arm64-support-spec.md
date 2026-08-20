# macOS Apple Silicon Support

Status: active; full direction authorized by the project owner on 2026-08-05;
MAC1 implementation, automated verification, and independent contract review
complete on 2026-08-05; MAC2 implementation, automated verification, arm64
debug-app runtime acceptance, and separate contract review complete on
2026-08-05; MAC3 implementation, affected automated verification, isolated
native-Keychain smoke, unsigned arm64 development-app workflow acceptance, and
separate contract review complete on 2026-08-05; the MAC3 mandatory stop is
reached; MAC4 entry review complete and explicitly authorized on 2026-08-05;
MAC4 implementation, locally available verification, version/NEWS review, and
separate contract review complete on 2026-08-05; authoritative main-repository
candidate assets and draft creation remain NOT RUN; the MAC4 mandatory stop is
reached; MAC4-R fork rehearsal entry review complete and explicitly authorized
on 2026-08-05; its implementation, local automated verification, separate
contract review, and exact-commit credentialed hosted rehearsal completed on
2026-08-06; installed-app testing then rejected that `0.4.0-dev.1` rehearsal
artifact because hardened-runtime library validation prevented bundled Ark
from loading the official CRAN R shared library; the bounded `0.4.0-dev.2`
repair was explicitly authorized, implemented, and locally verified on
2026-08-06; its exact-commit replacement hosted rehearsal also passed that
day; upstream `main` was integrated without history rewriting on 2026-08-07
and its independent development line through `0.4.0-dev.15` requires the
combined candidate to advance to `0.4.0-dev.16`; MAC4-R3 asynchronous
notarization orchestration is explicitly authorized, locally implemented,
verified, and contract-reviewed on 2026-08-07; its first two exact-commit
hosted rehearsals failed closed at the developer-log host allowlist and then
the fresh-finalizer R dependency gate; both bounded repairs were regression-
covered; replacement run `31163017077` at exact commit
`8de3dcc1dafc9e8562d239a6051a9113b778f1c3` passed the full review-only
rehearsal, reduced total macOS-runner use to 13 minutes 51 seconds, and
produced an independently verified seven-file artifact; the MAC4-R3 mandatory
stop was reached for that source; upstream then advanced through `b5800ae`;
the new commits were integrated by ordinary merge `9d3086e`, their complete
affected local validation passed, and replacement run `31165265090` at exact
post-merge commit `c4661bbe25dcc326737c51b385c65865a795edb9` passed the
full independently verified review-only rehearsal with 12 minutes 10 seconds
of macOS-runner use; the refreshed MAC4-R3 mandatory stop is reached; Issue #4
CRED-UX2 subsequently advanced the live development identity to
`0.4.0-dev.17`; CRED-UX3/CRED-UX4A advanced it to the installed-and-rejected
`0.4.0-dev.18`; CRED-UX4A-R1 reached the now-historical replacement identity
`0.4.0-dev.19`; Issue #6 produced installed-and-rejected identity
`0.4.0-dev.20`; PROBLEMS-AGENT-REPAIR-3, CRED-UX4A-R2, and WS3-Q1-R1 advance
the corrective development identity to now-rejected `0.4.0-dev.21`; owner
workflow review then authorizes PROBLEMS-AGENT-REPAIR-4 and advances the live
replacement identity to now-rejected `0.4.0-dev.22`; its owner workflow
acceptance exposes the remaining file parse-token selection gap, and the
explicitly authorized PROBLEMS-AGENT-REPAIR-5 produced now-historical
replacement identity `0.4.0-dev.23`; its source validation passed but no
artifact was built before Issue #9 advanced the live application identity to
`0.4.0-dev.24`, while every earlier run remains exact-source historical
evidence and cannot validate the combined corrected behavior;
the owner accepted the local `dev.24` development-app experience and authorized
one exact-default-branch review-only `Rho_for_mac` rehearsal on 2026-08-09;
run `31294667960` passed with independently verified exact-source artifacts;
the owner then integrated the same source into upstream `main` and requested
the start of publication, authorizing authoritative candidate/draft creation;
MAC5 installed acceptance and final publication remain open and unauthorized

Change class: D3 shared platform architecture, runtime distribution,
credential boundary, update protocol, and release automation. The exact
candidate and publication work in MAC4-MAC5 is additionally D4.

Risk: R3 for MAC1-MAC3 and R4 for MAC4-MAC5.

Owning documents: the active development roadmap retains milestone order and
the complete M3 acceptance gate. This specification owns the Apple Silicon
macOS 14+ implementation stream, its platform adapters, bundled Ark runtime,
R discovery, Apple Keychain integration, additive update artifact, and signed
DMG handoff. The accepted About/update specification retains channel,
allowlist, manifest-fetch, SemVer, and user-initiated-install policy. Existing
project, Workspace R, Agent R, approval, persistence, and scientific evidence
contracts remain authoritative.

Authorization evidence: after reviewing a decision-complete implementation
plan, the project owner requested "Implement the plan" on 2026-08-05. This
activated MAC1. After the MAC1 implementation checkpoint and evidence were
reported, the project owner requested "激活后续步骤" on 2026-08-05. Per the
one-package governance rule, this activated MAC2 only. After the MAC2
checkpoint was committed and its evidence reported, the project owner
requested "激活 MAC3" on 2026-08-05. This activates MAC3 only after the entry
review recorded below. The project owner then confirmed "允许开始 MAC3 实现和验收"
on 2026-08-05. MAC4 and MAC5 still require their own entry-condition review and
recorded authorization amendment before product-code work starts. After the
MAC3 checkpoint and evidence were committed, the project owner requested
"开始完成MAC4" on 2026-08-05. This activates MAC4 only under the entry review
and mandatory stop recorded below; MAC5 remains unauthorized. After the MAC4
implementation was pushed for upstream review, the project owner identified
that the main repository requires pre-merge hosted build evidence and then
approved the bounded fork rehearsal proposal with "行，那就这样做吧" on
2026-08-05. This activates MAC4-R only; it does not authorize a fork Release,
an authoritative candidate, MAC5 acceptance, or publication.
After the installed `0.4.0-dev.1` DMG reproduced a different-Team-ID library
validation failure, the project owner requested "修复这个问题，并push" on
2026-08-06. This activates only the bounded MAC4-R2 entitlement, version,
diagnostic-gate, and replacement-rehearsal repair recorded below. It does not
authorize an authoritative candidate, MAC5, a tag, a Release, or publication.
After the passing replacement run showed that Apple queue time kept the macOS
runner idle for more than three hours, the project owner approved the bounded
orchestration optimization with "我同意这个优化" and requested
"顺便同步一下上游仓库的最新更改" on 2026-08-07. This activates MAC4-R3 and a
non-history-rewriting merge of `YuLab-SMU/Rho` `main`; it does not activate
MAC5 or any publication mutation.

Mandatory stop: implement MAC4's additive update schema, synchronized
`0.4.0-dev.1` candidate identity, deterministic evidence tooling, parallel
Windows/macOS candidate build, signed/notarized macOS packaging, immutable
draft-release assembly, and separately gated publish workflow; run all locally
available validation and record every hosted secret/signing/draft gate that was
not run; reconcile NEWS and contracts; then stop before MAC5 exact-candidate
installed acceptance, acceptance-evidence upload, release publication, or live
Pages acceptance.

MAC4-R mandatory stop: add a repository-bound, artifact-only rehearsal lane;
prove its positive and negative admission contract locally; push it to the fork
default branch; run one exact-commit Windows/macOS hosted rehearsal using the
fork repository secrets; record the run, artifact hashes, and any failure
truthfully; then stop. Do not create a Git tag or Release, do not promote fork
artifacts into the main candidate, and do not activate MAC5.

MAC4-R2 mandatory stop: advance the distributed development identity to
`0.4.0-dev.2`; grant only the Apple library-validation exception required for
Ark to load a separately signed arm64 R; prove the signed Ark and app contain
the exact entitlement and no other exception; reproduce a successful Workspace
smoke against official CRAN arm64 R; run one new exact-commit fork rehearsal;
record its immutable evidence; then stop before MAC5 or publication.

MAC4-R3 mandatory stop: integrate the latest upstream `main`, select a new
single-use candidate identity above both development lines, replace synchronous
macOS-runner waiting with one immutable submit/wait/finalize chain, prove its
negative and recovery paths locally, run one exact-commit fork rehearsal, and
record macOS-runner duration plus immutable evidence. Do not create or mutate a
tag, Release, draft, Pages state, acceptance record, or MAC5 evidence.

## Goal And Success Criteria

Deliver Rho as a public, Developer-ID-signed and Apple-notarized direct-download
DMG for Apple Silicon Macs while preserving Windows x64 behavior and the same
Workspace/Agent ownership boundaries.

The first rehearsal identity was `0.4.0-dev.1`; installed-app evidence rejected
it before any authoritative candidate or draft existed. The repaired fork-only
`0.4.0-dev.2` rehearsal passed. Because synchronized upstream source already
advanced independently through `0.4.0-dev.15`, the integrated source first
became `0.4.0-dev.16`; Issue #4 work advanced the development candidate
through `0.4.0-dev.17`, the rejected `0.4.0-dev.18`, the historical
Provider-first recovery identity `0.4.0-dev.19`, the rejected Issue #6
identity `0.4.0-dev.20`, and the current corrective identity
`0.4.0-dev.21`, followed by the current Console-entry replacement identity
`0.4.0-dev.22`, parser-token identity `0.4.0-dev.23`, and the current Task Rail
semantics replacement identity `0.4.0-dev.24`. It supports macOS 14 or later,
arm64 R 4.4 or later, and an arm64 Ark 0.1.252 sidecar. It is developed in
`YuLab-SMU/Rho_for_mac`, reviewed into `YuLab-SMU/Rho`, and released only by the
main repository through GitHub Releases and the Rho website.

Success requires all of the following facts to be recorded separately:

- platform-neutral desktop code and configurations build on Windows and macOS;
- the installed app runs its bundled arm64 Ark with a selected arm64 R;
- macOS integration, Keychain, keyboard, files, Git, Quarto, and recovery
  behavior pass the affected workflow matrix;
- the exact DMG passes signature, notarization, staple, Gatekeeper, clean
  install, quarantine, upgrade/reinstall, and uninstall acceptance;
- the same published candidate retains the supported Windows x64 artifact;
- a D4 checklist records an explicit GO before the draft release and update
  site become public.

## Decisions And Non-goals

- Architecture: Apple Silicon only for this candidate. Intel x64 and Universal
  Binary remain open M3 work.
- Minimum OS: macOS 14.0.
- Distribution: direct DMG from GitHub Releases and the Rho website. No Mac App
  Store and no App Sandbox.
- Signing: Developer ID Application, hardened runtime, minimal entitlements,
  App Store Connect API-key notarization, and stapling are mandatory for public
  release. No identity is hard-coded in the repository.
- R support: retain the existing R 4.4 minimum and test the implementation-time
  current stable R as well. The first candidate accepts arm64 R only.
- Updates: this MAC4/MAC5 package owns discovery and redirect only. No
  background installer download, delta update, or silent restart is permitted
  here. A later separately cross-reviewed native-updater contract may add an
  explicit-user signed install/restart path only after it preserves Apple
  signing, notarization, stapling, and exact-candidate acceptance.
- No new public Workspace protocol, persistence schema, approval lane, shell
  execution authority, remote execution, or scientific behavior is introduced.
- Linux, Intel macOS, App Store packaging, automatic update installation
  **within this MAC package**, and executing user shell startup files are out
  of scope. This does not authorize a native updater; the separately active
  `2026-08-15-tauri-native-updater` contract must be activated first.

## Compatibility And Ownership

### Desktop configuration

The base Tauri configuration contains only cross-platform identity, frontend,
window, security, and shared bundle metadata. Tauri's platform files own bundle
targets, platform resources, and platform icons:

- Windows retains NSIS, WebView2 loader, current Ark resources, and `.ico`/PNG
  icons in `tauri.windows.conf.json`.
- macOS uses app and DMG targets, an `.icns` icon, minimum system version 14.0,
  hardened runtime, and a minimal entitlement file in
  `tauri.macos.conf.json`.

MAC1 does not bundle Ark or produce a distributable candidate. It creates a
buildable platform boundary and uses generated RGBA icon inputs so the Tauri
macro and packaging configuration can be validated on arm64 macOS.

### Runtime and environment

MAC2 adds `macos-arm64` to the project-owned Ark runtime manifest. Ark 0.1.252
is downloaded from its pinned release, verified against SHA-256
`aa1186f6e1ad271abaf246fd76e0aa9039cdeeff2cb52147e8887060afd5fb07`, checked
for arm64 Mach-O architecture, and staged as
`binaries/ark-aarch64-apple-darwin`. Generated runtime binaries remain ignored;
the manifest, bootstrap script, license handling, and configuration are
tracked.

The installed app locates Ark beside the macOS app executable and retains the
existing Windows resource lookup. The existing Rust/Jet kernel launch path
remains authoritative; no Tauri shell plugin is added.

Rscript discovery order is:

1. a valid persisted user selection;
2. `RHO_RSCRIPT`;
3. `/Library/Frameworks/R.framework/Resources/bin/Rscript`;
4. `/Library/Frameworks/R.framework/Versions/Current/Resources/bin/Rscript`;
5. a deterministic search of the application child-process PATH.

The probe returns version, architecture, R home/bin, path separator, libraries,
and effective startup-file paths. R below 4.4 is rejected. Non-arm64 R is
rejected with stable recovery code `R_ARCH_MISMATCH`, without modifying the R
installation or user files.

The child-process PATH merges, in order, inherited entries, selected R bin,
`/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/bin`, `/usr/sbin`,
`/sbin`, and an existing `~/.local/bin`. Native path APIs deduplicate entries.
No shell or shell startup file is executed. Ark kernelspec, supervised Git,
Agent R, and Quarto discovery consume this environment.

Jet's existing Unix process group, signal interrupt, watchdog, and shutdown
semantics remain authoritative. MAC2 may add only a narrow
`ArkSession::terminate_process_group()` recovery wrapper for the existing
desktop `Arc::try_unwrap` failure path: SIGTERM, bounded wait, then SIGKILL.

### macOS integration and credentials

MAC3 maps safe external navigation and log-directory opening to macOS `open`,
uses `HOME` only as a platform fallback before canonical project validation,
and removes Windows executable names from macOS-facing picker and recovery
copy. It does not change canonical project identity or containment rules.

The existing keyring abstraction uses Keyring 4.1.6's `v1` Apple native
Keychain store on macOS. Stable provider IDs, precedence, size bounds,
redaction, Agent-only injection, idempotent delete, and `.Renviron`
compatibility remain unchanged. No Keychain entitlement or sharing group is
added for the non-sandboxed direct-download app. Default automated tests use
an injected credential store; a native Keychain smoke is explicit, isolated,
and cleanup-bound so routine tests never alter a developer credential.

The basic editor recognizes Command+Enter on macOS. Monaco uses Command for
command-or-control actions and Command-click definition navigation. Windows
Ctrl behavior remains unchanged. Browser/mock mode gains an explicit,
deterministic macOS fixture; it never infers platform behavior from a developer
machine.

### Update manifest

MAC4 keeps `schema_version: 1` and adds optional
`artifacts.macos_aarch64` with the same `url`, lowercase 64-character
`sha256`, and positive byte `size` fields as the Windows artifact. Existing
Windows clients may ignore the additive field. New clients select the current
platform artifact; a valid feed without one returns recoverable
`UPDATE_PLATFORM_UNAVAILABLE` and never blocks startup.

All existing response limits, timeouts, SemVer/channel rules, text rendering,
and Rho/GitHub URL allowlists remain unchanged. Notarization is release
evidence, not a manifest boolean. A published asset is never silently replaced
under the same version.

## Work Packages

### MAC1: Platform-neutral build shell — authorized

- split shared, Windows, and macOS Tauri configuration ownership;
- generate RGBA application icons and an `.icns` without changing product
  branding;
- configure macOS app/DMG metadata, macOS 14 minimum, hardened runtime, and
  minimal entitlements;
- add the smallest platform helpers needed for safe URL/directory opening and
  platform-appropriate default project fallback, with unit tests;
- remove macOS build-time references to Windows-only dependencies while
  preserving the existing Windows target configuration;
- prove macOS desktop check and configuration merge, plus Windows
  cross-configuration validation where the local toolchain permits it.

Exit gate: MAC1 diff contains no Ark bootstrap, R discovery, Keychain, update
schema, signing workflow, version bump, NEWS claim, or release publication.

### MAC2: Bundled Ark and arm64 R vertical slice — authorized 2026-08-05

Entry review:

- MAC1 is committed as `e4a3196`; its macOS configuration, desktop/workspace
  tests, arm64 debug build, independent review, version decision, and NO-GO
  release status are recorded above.
- The worktree was clean at activation and contained no unrelated user changes.
- Ark 0.1.252 remains the project-pinned runtime. The macOS arm64 archive and
  exact SHA-256 named in this contract are the only authorized new runtime
  input. The pinned GitHub HTTPS URL may follow HTTPS-only release-asset
  redirects, but the result is accepted only after the exact hash and arm64
  Mach-O checks pass; alternate versions, universal/x64 archives, manifest
  overrides, and unverified local binaries are rejected.
- Existing Jet process groups/watchdog, broker project identity, Workspace R
  authority, Agent R separation, R 4.4 minimum, and user startup-file policy
  are unchanged entry constraints.
- MAC2 may change only runtime manifest/bootstrap/lookup, R discovery/probe,
  deterministic child PATH/kernelspec environment, bounded Unix fallback
  cleanup, their tests, and browser/mock state strictly required to keep a new
  desktop runtime command truthful. MAC3-MAC5 surfaces remain out of scope.

- add pinned Ark manifest/bootstrap/sidecar and installed/development lookup;
- implement R discovery, architecture/version validation, deterministic PATH,
  and platform-correct kernelspec environment;
- close Unix shutdown recovery without changing Jet launch authority;
- demonstrate editor-to-Workspace R execution, interrupt, restart, crash
  recovery, and absence of orphan process groups.

Exit gate: a tracked Ark binary is forbidden; the ignored staged sidecar must
be reproducible from the pinned manifest. The debug app must launch the bundled
arm64 Ark with arm64 R 4.4+ and recover truthfully from missing Ark, checksum or
architecture failure, missing/old/x86 R, interrupt, shutdown, and crash. Stop
after MAC2 review even if MAC3 appears mechanically adjacent.

### MAC3: System integration and UI parity — authorized 2026-08-05

Entry review:

- MAC2 is committed as `8cb1a85`; its pinned Ark/R runtime, full Rust
  workspace evidence, arm64 bundled-app smoke, baseline observations, version
  decision, and NO-GO release status are recorded below. The worktree was
  clean at MAC3 activation and contained no unrelated user changes.
- Keyring 4.1.6 is already the Windows credential library. Its documented
  `v1` feature selects Apple native Keychain Services on macOS; MAC3 may add a
  macOS-target dependency and resulting target-specific lockfile entries, but
  no second credential library, Tauri credential plugin, entitlement, access
  group, shared vault, or secret migration.
- The active system-credential contract retains service `Rho Agent LLM`,
  stable provider-profile account IDs, the 16 KiB limit, stored-over-
  `.Renviron` precedence, Agent-only child injection, presentation redaction,
  replace/delete recovery, and unavailable/fallback truth. MAC3 changes only
  the production macOS adapter and macOS acceptance evidence.
- MAC1's allowlisted, no-shell `platform.rs` open/reveal adapter remains the
  native navigation authority. MAC3 may make R picker titles, filters, paths,
  and recovery copy platform-correct and exercise the existing adapter from
  the UI; it may not add arbitrary shell, URL, directory, or file authority.
- UX-KEYS-1 retains command routing and editor-action semantics; accepted WS2
  retains definition lookup/navigation. MAC3 adds only Command equivalents to
  the existing basic-editor execution and Monaco definition gestures while
  retaining Windows Ctrl behavior. Platform state comes from the desktop
  descriptor or an explicit mock query fixture, never `navigator.platform` or
  the developer machine.
- BH1/BH2 retain normalized project identity, containment, switching,
  rollback, and two-project truth. Watcher, Git, Quarto, panel, and Agent work
  in MAC3 are regression/installed-bundle validation of existing contracts,
  not new workflow or execution authority.
- MAC2's four `rho.bridge` observations are bounded entry gates. The macOS
  `/private/var` alias requires canonical-path comparison in the test only.
  The escaped local lockfile path is an implementation defect against the
  active WS1-L2 requirement that detail be shown only when the source is
  provably inside the project; its owner and behavior remain WS1-L2, and MAC3
  may only restore that contract with regression coverage. Bioconductor
  fixture provenance must be validated independently of the packages installed
  on the test machine; fixture viewer semantics remain unchanged.
- MAC3 is one R3 vertical slice. It may touch the target-specific Keychain
  adapter, platform-facing runtime copy/dialog behavior, Command gestures,
  explicit macOS mock fixture, the three bounded baseline repairs, focused
  regression tests, and this package's evidence documents. It may not change
  schema, project ownership, approval lanes, public protocol, runtime launch
  authority, scientific semantics, update feeds, signing, versions, NEWS, or
  release workflows.

Implementation scope:

- activate Apple Keychain through the existing credential abstraction, with
  injected-store automated coverage and an opt-in cleanup-bound native smoke;
- complete native open/dialog/log behavior and macOS recovery copy;
- add Command shortcuts, Command-click, and deterministic macOS mock parity;
- repair the three bounded cross-platform test/containment gates without
  redefining their owning product contracts;
- validate project paths, watching, Git, Quarto, panels, Agent, and
  two-project isolation from an unsigned installed development app bundle.

Exit gate: Keychain success, replacement, deletion, missing-delete,
provider-isolation, backend-failure, rollback, precedence, Agent-only
injection, redaction, and cleanup evidence pass without a real provider secret
in automation. macOS labels, picker filters, recovery copy, safe open/reveal,
Command gestures, and deterministic mock fixtures pass while Windows behavior
remains covered. The complete affected Rust workspace, `rho.agent`,
`rho.bridge`, frontend/mock, two-project, and platform regression matrix passes
or every unavailable runner is recorded truthfully. A launched app bundle
passes spaces/Unicode/symlink project paths, watching, Git, Quarto, panels,
Agent credential projection, and recovery; it is not a signed candidate,
clean-install, quarantine, update, or release acceptance. Stop after MAC3
review even if MAC4 appears mechanically adjacent.

### MAC4: Signed candidate and additive update publication — authorized 2026-08-05

Entry review:

- MAC3 is committed as `94aabf1`; its Keychain/UI/bridge implementation,
  complete affected automation, final arm64 app smoke, isolated development-app
  acceptance, version decision, and NO-GO release state are recorded below.
  The worktree was clean at MAC4 activation and contained no unrelated user
  changes.
- This package is D4/R4 because it changes candidate identity, release
  evidence, signing/notarization, GitHub Release draft creation, and update-site
  publication inputs. The new exact-candidate checklist is
  `docs/release/active-0.4.0-dev.24-candidate-checklist.md`; the historical
  `0.4.0-dev.16`/`0.4.0-dev.23`, rejected
  `0.4.0-dev.18`/`0.4.0-dev.20`/`0.4.0-dev.21`/`0.4.0-dev.22`, and old `0.2.0`
  checklist and `rho-0.2-release.json` remain authorities for their own
  candidate and are not reused as MAC4 acceptance.
- The accepted About/update design retains schema version 1, endpoints,
  channels, SemVer, bounds, allowlists, and user-initiated redirect behavior.
  MAC4 may add optional `artifacts.macos_aarch64` data and multi-platform page
  projection. Existing Windows-only manifests stay valid and Windows remains a
  required artifact for the new cross-platform candidate.
- GitHub's current standard Apple Silicon label is `macos-26`, not
  `macos-26-arm64`. The runner image provides Xcode 26.6 at
  `/Applications/Xcode_26.6.app`; the workflow must select that exact developer
  directory and fail if `xcodebuild -version` does not report 26.6.
- Tauri consumes `APPLE_API_ISSUER`, `APPLE_API_KEY`, and a filesystem path in
  `APPLE_API_KEY_PATH`. GitHub stores the base64 `.p8` content in
  `APPLE_API_PRIVATE_KEY`; the workflow writes it under `RUNNER_TEMP`, exports
  the real temporary path, and removes it in an unconditional cleanup step.
  Treating secret content as the path variable is forbidden.
- Candidate construction and publication are separate authorities. MAC4 may
  create a draft prerelease only after both platform jobs and aggregate
  evidence validate. The publish workflow may only flip an existing immutable
  draft after MAC5 uploads bounded GO acceptance evidence matching the same
  version, tag, commit, asset names, sizes, and hashes. MAC4 never publishes.
- A tag/version is single-use. Candidate assembly fails if any release with the
  same tag already exists, never deletes or replaces release assets, and uses a
  new version after withdrawal or failed accepted-candidate replacement.
- MAC4 may update Cargo/Tauri/frontend/cache-busting identities to
  `0.4.0-dev.1` and add truthful NEWS only after implementation verification.
  R package versions remain independent and unchanged because their exported
  contracts do not change.

- amend the accepted About/update implementation and generator for
  `macos_aarch64`;
- create a D4 `0.4.0-dev.1` exact-candidate checklist;
- replace immediate single-platform publication with parallel candidate build
  jobs that create a draft release, followed by a separate verified publish
  workflow;
- pin the hosted runner to `macos-26` and Xcode 26.6; import a Developer
  ID `.p12` into a temporary keychain, notarize through App Store Connect API
  credentials, staple, and emit checksums and evidence;
- synchronize Cargo, Tauri, and frontend versions to `0.4.0-dev.1` only after
  MAC1-MAC3 review, then add truthful NEWS entries.

Required CI secret interfaces are `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `KEYCHAIN_PASSWORD`,
`APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_API_ISSUER`,
`APPLE_API_KEY`, and `APPLE_API_PRIVATE_KEY`. The workflow alone sets
`APPLE_API_KEY_PATH` to its temporary `.p8` file. Secret values and local
keychain/key paths never enter logs, artifacts, manifests, or repository files.

The cross-platform draft contains:

- `Rho_0.4.0-dev.1_x64-setup.exe` and its `.sha256` sidecar;
- `rho-0.4.0-dev.1-windows-x86_64-evidence.json`;
- `Rho_0.4.0-dev.1_aarch64.dmg`;
- `Rho_0.4.0-dev.1_aarch64.dmg.sha256`;
- `rho-0.4.0-dev.1-macos-aarch64-evidence.json`;
- `rho-0.4.0-dev.1-candidate-evidence.json`, binding both platform records to
  one version, tag, commit, asset set, sizes, and hashes.

Exit gate: deterministic tests reject malformed identity, missing platforms,
duplicate names, stale/foreign evidence, checksum/size mismatches, draft input
to Pages, and publish attempts without exact MAC5 GO evidence. Local unsigned
DMG builds may validate configuration but are never reported as signed. Hosted
Developer ID signing, notarization, stapling, Gatekeeper checks, draft creation,
and uploaded hashes are separate facts and remain `NOT RUN` until a credentialed
workflow creates the exact draft. Stop after MAC4 implementation/review and any
available draft evidence; do not perform MAC5 acceptance or publish the release.

### MAC4-R: Fork-only signed rehearsal — authorized 2026-08-05

Problem and evidence: the first upstream review requires hosted cross-platform
build evidence before merge, while GitHub does not expose repository secrets to
an upstream pull-request job and manual dispatch requires the workflow to exist
on the repository default branch. Running the current candidate mode in the
fork would create a misleading draft whose source repository contradicts the
exact-candidate checklist.

Contract:

- `workflow_dispatch` gains one required choice input, `build_mode`, whose safe
  default is `rehearsal`; accepted values are only `rehearsal` and `candidate`;
- `rehearsal` is admitted only when `github.repository` is exactly
  `YuLab-SMU/Rho_for_mac`; `candidate` is admitted only when it is exactly
  `YuLab-SMU/Rho`; every other mode/repository pairing fails in the identity
  job before a platform build uses credentials;
- the selected workflow ref and repository default branch must both be `main`,
  and the requested source ref must resolve to the current default-branch HEAD.
  Every checkout disables persisted Git credentials. This prevents a manual
  run from redirecting Apple secrets into unreviewed branch code;
- both modes resolve and check out one full commit and run the same Windows x64
  and signed/notarized macOS arm64 platform jobs. Secret names, temporary-file
  handling, unconditional Keychain cleanup, and platform evidence remain the
  MAC4 contract;
- candidate mode alone may run the `contents: write` draft-assembly job.
  Rehearsal mode has `contents: read`, never creates or mutates a tag, Release,
  Pages state, environment, issue, pull request, or repository content;
- rehearsal mode downloads both platform artifacts, runs the normal aggregate
  validator only as an internal consistency check, discards candidate aggregate
  output, and uploads one 14-day Actions artifact containing the six platform
  files plus bounded `rho_candidate_rehearsal_evidence`;
- rehearsal evidence binds status, source repository, version, release tag,
  full commit, GitHub run ID and attempt, both platform artifact names, sizes,
  hashes, and platform-evidence hashes. It contains no credential, runner path,
  Keychain path, API-key path, or unbounded log;
- rehearsal artifacts and evidence are review-only. The candidate workflow in
  `YuLab-SMU/Rho` must rebuild both platforms and may not ingest, promote, copy,
  or publish them. MAC5, publish admission, and update-site generation reject
  the rehearsal evidence type by construction.

Failure and recovery:

- missing/invalid secrets, checkout drift, platform failure, signing failure,
  notarization failure, cleanup failure, missing platform, malformed evidence,
  or checksum/size mismatch prevents final rehearsal evidence;
- Actions may retain a successfully uploaded platform input when a sibling job
  later fails, but such partial artifacts have no aggregate rehearsal record and
  therefore are not passing evidence;
- a retry is a new run attempt bound into new evidence. It does not delete or
  replace a release because rehearsal has no release permission or mutation.
  Intermediate platform artifact containers are scoped to the GitHub Run ID
  and may be replaced within that Run; the final rehearsal artifact includes
  both Run ID and Attempt and is never overwritten by a later attempt;
- cancellation still executes the existing unconditional Apple credential
  cleanup step. Absence of final rehearsal evidence is reported as failure or
  cancellation, never success.

Verification and exit gate:

- deterministic tests cover safe-default mode, exact repository guards,
  least-privilege rehearsal assembly, candidate-only write permission, no
  Release step in rehearsal, 14-day retention, exact seven-file artifact, and
  rejection of malformed/foreign/stale/oversized rehearsal evidence;
- the hosted exit requires both platform jobs and the aggregate rehearsal job
  to pass for one exact fork commit, followed by recording the run URL, artifact
  identity, and hashes in this active document and the pull request;
- application and R package versions remain unchanged and `NEWS.md` is not
  amended because this is an internal review-evidence lane with no user-visible
  application or package contract change;
- stop after recording rehearsal evidence. A main-repository draft remains a
  separate MAC4 fact, the release decision remains `NO-GO`, and MAC5 remains
  unauthorized.

Hosted defect-repair checkpoint:

- the exact fork rehearsal is the acceptance environment for platform-specific
  failures that cannot be reproduced on the local Apple Silicon host. A hosted
  failure does not widen MAC4-R authority: repair work is limited to restoring
  an already accepted cross-platform contract or making its deterministic test
  portable, and the replacement run must rebuild both platforms from one new
  default-branch commit;
- Windows must reject every local lockfile source containing a lexical `..`
  component before filesystem probing. This preserves the accepted WS1-L2
  fail-closed rule even when Windows path APIs collapse missing ancestors
  differently from Unix. Missing and Unicode paths without `..` remain
  reportable only when their nearest existing ancestor proves containment;
- the R architecture regression must test Apple-Silicon policy through the
  existing explicit target-OS/target-architecture predicate. A test running on
  Windows must not require the current Windows process to reject x86 R; product
  behavior and stable `R_ARCH_MISMATCH` recovery on Apple Silicon are unchanged;
- release-contract parsing of every checked-out text input must accept both LF
  and CRLF. The hosted Windows failure at the workflow `build_mode` assertion
  proves that fixing only the Cargo lockfile record separator is insufficient;
  the test harness must normalize line endings before all existing workflow,
  metadata, and source contract assertions. The Cargo lockfile assertion
  continues to require all local Rho packages and synchronized versions, and
  every workflow assertion retains its exact semantic pattern; only
  line-ending portability may change;
- the hosted macOS run must distinguish Tauri's automatic app notarization
  from notarization of the final DMG. Tauri 2 submits a temporary app archive
  before it creates the DMG; an Accepted `Rho.zip` submission does not prove
  that the later `Rho_<version>_aarch64.dmg` was submitted. To avoid two
  independent queues exceeding the hosted-job window, the Tauri bundle command
  must retain signing identity/team variables but remove notarization API
  variables only from that command. After the signed DMG exists, the workflow
  must explicitly run one `notarytool submit` against that exact path with the
  temporary Team API key and wait for completion. A project validator must
  reject a receipt over 64 KiB, malformed/non-object JSON, a missing/non-UUID
  submission ID, or any status other than `Accepted` before stapling,
  Gatekeeper, smoke, or platform evidence. A global history lookup or an
  app-only Accepted record cannot satisfy this gate;
- app and bundled-Ark architecture assertions remain exact arm64 gates, but
  must print a bounded label, path, and observed `lipo -archs` value before
  failing so a hosted regression identifies which binary violated the
  contract. This diagnostic output contains no credential or private path
  outside the checked-out candidate tree;
- each observed defect retains or gains a deterministic regression. The full
  hosted replacement run, not a rerun of only the failed job, is required for
  MAC4-R exit because admission binds the exact current fork `main` commit and
  the final evidence requires both platform artifacts.

Hosted evidence that activated the final-DMG repair: fork rehearsal run
`31065316772` built and signed both bundles at commit
`6f55fac14ccbc291c87dced48dab96b84b35dba1`; Tauri reported Accepted app
submission `cda1ed1c-71d0-461f-9e19-3ac0b5c8c030`, stapled the app, then
created and signed the DMG. The subsequent history assertion looked for the
DMG filename even though no final-DMG submission had occurred, exited without
platform evidence, and correctly caused the aggregate rehearsal job to skip.
The unconditional Apple credential cleanup passed. This is failure evidence,
not MAC4-R acceptance, and no cross-run artifact composition is permitted.

### MAC4-R3: Asynchronous notarization orchestration — authorized 2026-08-07

Problem and evidence: successful fork runs `31079170163` and `31097468979`
showed that the Tauri build, signing, entitlement checks, and DMG creation
finish in roughly six minutes, while Apple's final-DMG notarization queue holds
the macOS runner for approximately three and a half hours. Apple service time
cannot be shortened by Rho, but it need not consume an idle macOS runner.

Contract:

- preserve one final-DMG submission and the existing Developer ID, exact
  entitlement, hardened-runtime, evidence, repository, and publication gates;
- split the current macOS platform job into `macos-submit`, an Ubuntu
  `macos-notary-wait`, and `macos-finalize`;
- `macos-submit` builds/signs the DMG, verifies both final executable
  signatures, records the exact pre-submission size and SHA-256, submits with
  `notarytool --no-wait`, normalizes only that command's UUID into a bounded
  pending record, uploads immutable intermediate artifacts, and completes
  unconditional `.p12`, `.p8`, and Keychain cleanup before success;
- the pending record binds repository, build mode, version, release tag, full
  commit, Run ID/Attempt, UUID, final filename, byte size, and lowercase hash.
  It contains no Apple response history, credential, token, private path, or
  unbounded output;
- `macos-notary-wait` uses Node's built-in crypto and HTTPS support only. It
  creates short-lived ES256 team-key JWTs and may request only the fixed Apple
  status URL for the pending UUID and that UUID's `/logs` URL. API-key secrets
  are step-scoped; tokens and the private key never enter command lines,
  output, files uploaded as artifacts, or evidence;
- exact Apple statuses are `In Progress`, `Accepted`, `Invalid`, and
  `Rejected`. Only `In Progress`, bounded transport failures, HTTP 429, and 5xx
  are retryable under bounded delay/attempt/deadline rules. Unknown statuses,
  wrong identity/name, malformed or oversized responses, 401/403/404, and
  other 4xx fail closed;
- after `Accepted`, retrieve the exact submission's log URL and require
  credential-free HTTPS on an allowlisted Apple delivery host. The allowlist is
  Apple/itunes subdomains plus the observed exact Notary service bucket
  `notary-artifacts-prod.s3.amazonaws.com`; arbitrary AWS/S3 buckets remain
  forbidden. Download bounded JSON and bind its submission ID and status into
  the accepted record. A missing, rejected, mismatched, or oversized log cannot
  fabricate success;
- `macos-finalize` downloads only the exact pending, accepted, and DMG
  artifacts for its run, recomputes the original hash, and installs only
  `rho.bridge`'s declared non-base runtime import `jsonlite` into the fresh
  runner's temporary R library. It must not install bridge Suggests, Agent
  packages, or reuse the submission runner library. It then staples and
  validates the DMG, mounts it, and repeats arm64, codesign, exact entitlement,
  Gatekeeper, and full Workspace smoke against `Rho.app` inside the DMG before
  emitting the final checksum and normal macOS platform evidence;
- aggregate rehearsal/candidate consumers depend on `macos-finalize`. The
  unstapled DMG, pending record, Apple log, and accepted record use intermediate
  artifact names that cannot match or enter final candidate asset patterns;
- GitHub rerun of failed waiter/finalizer jobs reuses successful immutable
  submit artifacts, preventing duplicate submissions. A new workflow Run ID
  is a new request and may not use another run's request, DMG, or acceptance.

Failure and recovery:

- if submission, credential cleanup, upload, polling, log retrieval, hash
  binding, staple, mount, Gatekeeper, or smoke fails, no final macOS platform
  evidence exists and aggregate/draft jobs skip;
- a timeout reports failure while Apple may continue independently. Recovery
  is a failed-job rerun over the same immutable UUID and DMG, not a history
  lookup or automatic resubmission;
- response bodies are byte-bounded before parsing; retry delay and total wait
  stay below GitHub's six-hour job limit; cancellation never creates accepted
  evidence;
- the macOS submission job remains the only job that imports the Developer ID
  certificate. The Ubuntu waiter receives only the notarization team-key
  secrets, and the finalizer receives none of the eight Apple secrets.

Verification and exit gate:

- deterministic tests cover pending/accepted/log schemas and identity binding,
  ES256 token claims and lifetime, success, terminal rejection, unknown state,
  malformed/oversized input, 401/403/404, bounded transport/429/5xx recovery,
  timeout, untrusted log URL, hash mismatch, cross-run/stale evidence, and
  rerun reuse;
- workflow contract tests prove one submit call, no `--wait` in the submission
  job, exact job dependencies, least-privilege secret placement, immutable
  intermediates, finalizer gates, and aggregate consumption of only final
  platform evidence;
- complete affected JavaScript, YAML/actionlint, Rust, R, and release/update
  validation runs before review; one exact `0.4.0-dev.16` fork rehearsal must
  prove Windows retention, a materially shorter macOS submission job, accepted
  log-bound notarization, finalization, cleanup, and zero publication;
- stop after recording review-only rehearsal evidence. MAC5, authoritative
  draft creation, tag/Release mutation, and Pages publication remain
  unauthorized.

### MAC5: Exact-candidate acceptance and publication — implemented; published

- bind automated and human evidence to the exact tag, commit, assets, sizes,
  and hashes in the draft release;
- complete clean macOS 14 and current-macOS installed acceptance with R 4.4
  and the current stable R;
- verify signature, notarization, staple, Gatekeeper, quarantine launch,
  reinstall/upgrade/uninstall, core scientific workflow, and recovery;
- publish from the main repository only after an explicit D4 GO; update Pages
  only from a published release, never a build-complete or draft event.

Rollback preserves evidence: withdraw the macOS update-site entry and mark the
prerelease withdrawn. Do not replace a same-version asset; use a new candidate.

MAC5 activation and acceptance evidence on 2026-08-09: authoritative candidate
run `31295799312` created draft prerelease ID `367387340` from exact commit
`7c18e08d7b34dc7d976fa3685242402ccd7da2e8`; both platform records, the
aggregate record, the signed/notarized/stapled macOS DMG, and independent local
hash/signature checks passed. The owner installed that exact DMG, reported
`MAC5 PASS`, and authorized public publication. Acceptance asset
`rho-0.4.0-dev.24-acceptance.json` is 1,598 bytes with SHA-256
`642e0b7774ff60e4a6db35956dcb65883623c510995030ddbae9ccf71faf20a3`
and is bound to aggregate SHA-256
`fe4a9fba56cd1b5f1d62d1ef7c6cc462f980c8c2fa4595e504d76f2d6743279d`.

Protected publish run `31297205980` then failed closed before checkout or
Release mutation because `getReleaseByTag` returned 404 for the draft's not-yet-
created public tag. The owner authorized a bounded release-only correction:
discover exactly one matching draft through paginated Release listing, pass its
numeric ID forward, retrieve subsequent snapshots by ID, and retain all exact-
commit, eight-asset, byte/hash, one-mutation, and environment-approval gates.
The accepted application artifacts remain immutable and are not rebuilt.

The correction landed as `f30b1ae240d056ef97f670d85c8e925d89b9415d`.
Protected publish run `31297462728` passed the exact-candidate and MAC5
admission and published prerelease `v0.4.0-dev.24` without changing its eight
assets. Automatic update-site run `31297482853` published `gh-pages` commit
`dcfbdbdb5a53e4fedc2c18880a18b4145804e014`; its hosted check and an
independent live fetch both confirmed development manifest version
`0.4.0-dev.24` with the accepted Windows and macOS sizes and hashes. MAC5 is
complete for this Apple Silicon development candidate; macOS x64 and Linux x64
remain separate milestone scope.

## Verification Contract

### MAC1 focused evidence

- JSON/config validation proves the base config has no platform resources,
  Windows retains NSIS/current resources, and macOS owns app/DMG, minimum 14.0,
  hardened runtime, entitlements, and `.icns`;
- all PNG bundle icons report RGBA and the `.icns` is a valid Apple icon file;
- platform helper tests cover supported command selection, URL allowlist
  preservation, HOME fallback, missing HOME, spaces, and Unicode;
- `cargo check -p rho-desktop` passes on Apple Silicon;
- `cargo fmt --all -- --check`, focused tests, frontend syntax, configuration
  validation, and `git diff --check` pass;
- Windows target/config regression is run in CI or explicitly recorded as
  unrun locally, never inferred from macOS results.

### MAC3 focused evidence

- injected credential-store tests cover success, replacement, deletion,
  missing delete, provider isolation, validation, backend failure,
  metadata/credential rollback, `.Renviron` fallback and precedence, Agent-only
  process injection, and proof that secrets never enter persisted settings,
  runtime profiles, command arguments, diagnostics, events, logs, or mock
  responses;
- an opt-in serial native-Keychain smoke uses a unique MAC3 service/account,
  proves set/get/replace/delete, and verifies cleanup even on failure; it is
  never part of default automation and never uses a real provider credential;
- platform tests cover macOS R picker title/filter, missing/old/wrong-arch R
  copy, safe product URL and log reveal, spaces/Unicode paths, missing HOME,
  and unchanged Windows command selection/copy;
- frontend tests cover basic-editor Command+Enter and Command+Shift+Enter,
  Monaco Command-click, existing command-or-control shortcuts, Ctrl regression,
  input/dialog ownership, and an explicit deterministic macOS mock fixture;
- `rho.bridge` regressions cover canonical temporary-directory equality,
  local-source containment including `..`, sibling-prefix, absolute, missing,
  Unicode, and symlink cases, and fixture provenance checks independent of
  locally installed Bioconductor versions;
- the affected Rust workspace, both affected R packages, frontend/mock suites,
  configuration/syntax checks, `git diff --check`, and available Windows
  regression run before completion;
- unsigned app-bundle acceptance exercises spaces, Unicode, symlinks, project
  watching, supervised Git and Quarto discovery, editor/Console/Environment/
  Viewer/Plots/Problems/Agent panels, two-project isolation, Keychain status,
  interrupt/restart/failure recovery, and post-test Keychain/process cleanup.

### MAC4-MAC5 evidence

- R discovery precedence, persisted/invalid/missing R, R 4.3 rejection, arm64
  acceptance, x86 mismatch, Unicode/space paths, PATH bounds and no-shell
  execution;
- Ark missing/checksum/architecture/bootstrap failures, sidecar lookup, smoke
  execution, interrupt/shutdown/crash recovery, and no orphan process group;
- update compatibility fixtures for Windows-only and two-platform feeds,
  missing current artifact, malformed metadata, untrusted URL, and tampered
  checksum/evidence;
- complete affected Rust workspace, R package, frontend/mock, two-project,
  Windows regression, and installed-app workflow suites;
- release checks use `codesign --verify --deep --strict --verbose=4`, `spctl`,
  notarization result inspection, and `xcrun stapler validate` against the exact
  candidate.

## Version, Documentation, And Release State

MAC1-MAC3 remain `0.4.0-dev.0` and are not distributable candidates. They do
not change exported R package behavior, so no R package bump is expected.
`NEWS.md` is unchanged until reviewed user-visible behavior enters MAC4's
candidate.

The fork's `0.4.0-dev.1` and `0.4.0-dev.2` identities are retained only as
historical rehearsal evidence. Upstream independently used those prerelease
numbers and reached `0.4.0-dev.15`; the merged application authorities,
frontend cache-busting, mock identity, roadmap, workflow defaults, checklist,
and `NEWS.md` therefore advance together to `0.4.0-dev.16`. R package versions
remain independently governed and unchanged. MAC4-R3 changes release
orchestration only and does not require a second application-version advance.

This document stays active after code lands while later packages, installed
acceptance, or release gates remain open. The roadmap may record implementation
presence only after the corresponding evidence exists. Neither a local build,
an unsigned DMG, automated tests, nor a notarized draft alone constitutes M3
acceptance or release GO.

## MAC1 Implementation Evidence — 2026-08-05

Implementation present:

- Tauri base configuration is platform-neutral; Windows retains its existing
  NSIS, WebView2, Ark resource, and icon inputs in the Windows platform file;
  macOS owns app/DMG, macOS 14.0, hardened runtime, minimal entitlements, and
  `.icns` configuration in the macOS platform file.
- Existing brand artwork was regenerated as RGBA PNG, ICO, and ICNS assets.
  Pixel/color inspection and a composited visual review confirmed the teal
  field and white R were preserved.
- Safe product URL validation remains in the update boundary. A tested platform
  command adapter now maps validated navigation and log reveal to
  `explorer.exe`, macOS `open`, or `xdg-open` without invoking a shell.
- Windows keeps its existing `D:\Rho`/`USERPROFILE` default behavior. macOS
  uses `HOME/Documents/Rho` before the unchanged project normalization and
  containment boundary. Missing-home, spaces, and Unicode cases are covered.
- The previously Windows-only symlink security fixture now uses the native
  Windows or Unix API, allowing the same escape regression to run on macOS.
  Non-Windows no-op console helpers no longer emit unused-parameter warnings.

Automated evidence passed:

- `node scripts/test-desktop-platform-config.mjs`;
- `node --check desktop/dist/app.js`;
- `plutil -lint desktop/src-tauri/Entitlements.plist`;
- `cargo fmt --all -- --check`;
- focused platform and default-project-root tests: 6 passed;
- `cargo test -p rho-desktop`: 109 passed;
- `cargo check -p rho-desktop`;
- `cargo test --workspace`: all workspace and documentation tests passed;
- Tauri CLI 2.11.4 arm64 debug build with `--no-bundle`;
- Tauri CLI 2.11.4 arm64 debug app bundle build;
- the resulting local debug executable is Mach-O arm64 and its app metadata
  reports `LSMinimumSystemVersion` 14.0 and identifier `org.yulab.rho`;
- `git diff --check`.

Independent contract review found no blocking ownership, security, or scope
deviation. It confirmed that Windows build/release scripts continue reading
shared product/version metadata while Tauri owns platform-file merging, and
that no Ark manifest, R discovery, Keychain backend, updater schema, workflow,
application/R-package version, or NEWS change entered MAC1.

Explicitly not run or not accepted:

- Windows compilation, NSIS packaging, and installed-app regression require a
  Windows runner and remain open evidence; static Windows configuration and
  command behavior tests passed locally.
- R package tests were not run because MAC1 changes no R source or package
  contract.
- The debug app has only the Rust linker's ad-hoc executable signature; bundle
  signing, notarization, stapling, Gatekeeper, clean-install, and runtime smoke
  acceptance are MAC4-MAC5 gates and are not claimed.

Version outcome: application remains `0.4.0-dev.0`, R package versions are
unchanged, and `NEWS.md` is unchanged. Release decision: NO-GO; MAC1 is a
reviewed development checkpoint, not a distributable candidate.

## MAC2 Implementation Evidence — 2026-08-05

Implementation present:

- `runtime/ark.json` pins the official Ark 0.1.252 macOS arm64 archive and
  lowercase SHA-256. The macOS bootstrap accepts only the repository manifest,
  uses HTTPS-only redirects, verifies the hash before extraction, requires the
  expected binary plus license and notice, validates arm64 Mach-O, stages the
  Tauri target-triple sidecar atomically, and leaves generated runtime inputs
  ignored.
- Tauri packages the sidecar beside the app executable as `ark` and installs
  Ark's `LICENSE` and `NOTICE` under app resources. Installed lookup prefers
  that sibling; development lookup falls back to the ignored staged sidecar.
  Existing Windows resource paths and x64 discovery behavior remain present.
- R discovery now follows the authorized persisted selection, `RHO_RSCRIPT`,
  standard macOS framework locations, and deterministic child PATH order. The
  probe records R home/bin, architecture, native path separator, version,
  libraries, and effective startup-file paths. Apple Silicon rejects non-arm64
  R with stable code `R_ARCH_MISMATCH`; R 4.4 remains the minimum.
- Kernelspec, supervised Git, and Agent R processes consume the same native,
  deduplicated child PATH without invoking a shell. The runtime cache moved to
  version 2 so older records cannot omit architecture or R-bin evidence.
- The desktop's exceptional shared-`Arc` shutdown path now delegates to a
  narrow Unix Ark process-group fallback: SIGTERM, one-second bounded wait,
  then SIGKILL. Jet remains the sole launch, interrupt, watchdog, and ordinary
  shutdown authority.
- The desktop smoke path now uses the platform sidecar, verifies an interrupt
  followed by successful execution, retains the existing two-project and
  graceful-restart checks, kills one Ark process group to simulate failure,
  and proves a fresh Ark session can execute afterward.

Automated and local runtime evidence passed:

- `bash -n scripts/bootstrap-ark-macos.sh scripts/test-bootstrap-ark-macos.sh`;
- `scripts/test-bootstrap-ark-macos.sh`, covering checksum mismatch, missing
  Ark, and non-Mach-O architecture rejection;
- a real `scripts/bootstrap-ark-macos.sh` run against the pinned archive; its
  downloaded SHA-256 was
  `aa1186f6e1ad271abaf246fd76e0aa9039cdeeff2cb52147e8887060afd5fb07`,
  staged Ark reported `Ark 0.1.252`, and `file`/`lipo` reported arm64 Mach-O;
- `node scripts/test-desktop-platform-config.mjs` and
  `node --check desktop/dist/app.js`;
- `cargo fmt --all -- --check`, `cargo check -p rho-desktop`, focused kernel,
  Agent-environment, runtime-discovery, Ark-lookup, recovery-code, PATH, cache,
  and platform tests;
- `cargo test -p rho-desktop` and `cargo test --workspace`;
- `testthat::test_local('r/rho.agent')`;
- Tauri CLI 2.11.4 `--debug --bundles app --target
  aarch64-apple-darwin --no-sign` produced an arm64 debug `Rho.app` containing
  arm64 `rho-desktop`, arm64 Ark 0.1.252, and the Ark license/notice;
- the bundled app executable, with `RHO_RSCRIPT` unset, selected arm64 R 4.5.2
  from the standard framework and passed Workspace execution, Plot,
  Environment, paged Viewer, stale-view rejection, two-project isolation,
  interrupt/recovery, graceful restart, simulated crash/restart, and durable
  event smoke checks;
- after smoke exit, `pgrep` found no process whose command referenced the
  bundled `Rho.app/Contents/MacOS/ark`;
- `git diff --check`.

Separate contract review found no blocking ownership or scope deviation. The
implementation does not change public protocol, persistence schema, project
identity, approval lanes, credentials, frontend command state, update schema,
signing, publication, application/R-package version, or `NEWS.md`. The only
contract wording correction permits the pinned GitHub URL's HTTPS-only asset
redirect while retaining exact hash and architecture admission. Browser/mock
changes were unnecessary because no Tauri command or visible runtime-state
shape changed.

Explicitly open or not accepted:

- the local Rust installation has only the `aarch64-apple-darwin` target, so
  Windows compilation, Windows installed regression, and NSIS packaging were
  not run. Static Windows Ark lookup, R discovery, command behavior, and Tauri
  configuration tests passed; this is not equivalent to a Windows runner.
- an additional, non-affected `rho.bridge` full-suite run exposed four existing
  environment/fixture differences: macOS `/private/var` canonicalization, one
  escaped local-source expectation, and two installed Bioconductor versions
  newer than the checked-in fixture. No R source changed in MAC2, the actual
  embedded bridge passed the bundled-app smoke, and these results are not
  reported as passing. They remain a bounded MAC3 workflow-validation gate.
- network-dependent Agent model smoke was not run; deterministic tests prove
  the supervised Agent R command receives the child PATH without exposing its
  credential, and MAC2 does not claim provider/network acceptance.
- the debug app was intentionally unsigned and was not a DMG, release
  candidate, notarized artifact, clean install, or public distribution.
  Keychain, native UI parity, exact-candidate signing, update publication, and
  release acceptance remain MAC3-MAC5 work.

Version outcome: application remains `0.4.0-dev.0`, R package versions are
unchanged, and `NEWS.md` is unchanged. Release decision: NO-GO. The mandatory
MAC2 stop is reached. MAC3 was subsequently entry-reviewed and activated by
the authorization recorded above; MAC4 was later entry-reviewed and authorized
after the MAC3 stop. MAC5 remains inactive until its own entry review and
authorization record.

## MAC3 Implementation Evidence — 2026-08-05

Implementation present:

- macOS now enables keyring 4.1.6 with its Apple-native `v1` backend behind the
  existing credential-store abstraction. Production retains service
  `Rho Agent LLM`, stable provider-profile accounts, the 16 KiB bound,
  stored-over-environment precedence, Agent-only injection, redacted views,
  rollback behavior, and unchanged Windows production selection. No Keychain
  entitlement, access group, migration, project credential, or second store
  was added.
- the opt-in macOS Keychain test uses a unique service/account and cleanup
  guard to prove set, get, replace, delete, idempotent missing delete, and a
  final missing read without using a provider credential. Default tests still
  use injected memory/failure stores.
- R selection dialogs and recovery copy now use platform-owned `Rscript` or
  `Rscript.exe` names and filters. The existing no-shell open/reveal adapter
  remains the only native navigation authority.
- the basic editor accepts Command+Enter and Command+Shift+Enter, Monaco
  definition navigation accepts Command-click, and existing Ctrl behavior is
  retained. Browser mode uses only an explicit `platform=macos-aarch64`
  fixture; unknown or absent fixtures remain Windows and no navigator field is
  consulted.
- the three bounded bridge gates are repaired without changing their owners:
  lint test paths compare canonical roots; local lockfile labels resolve the
  nearest existing ancestor and reject `..`, sibling-prefix, absolute, and
  symlink escapes while retaining missing/Unicode paths inside the project;
  portable fixture provenance is checked independently from the developer's
  installed Bioconductor package versions.

Automated verification passed:

- `cargo fmt --all -- --check` and `cargo check -p rho-desktop`;
- `cargo test --workspace --no-fail-fast`; the desktop binary reported 121
  passed and one intentionally ignored native-Keychain test, and every other
  workspace unit and documentation test passed;
- the exact ignored native-Keychain test run reported one passed test and
  verified its cleanup-bound final missing read;
- `testthat::test_local('r/rho.agent')` reported 45 passed expectations and
  `testthat::test_local('r/rho.bridge')` reported 514 passed expectations; its
  focused workspace run reported 324 passed expectations;
- `node --check desktop/dist/app.js` and all 32 `scripts/test-*.mjs` suites,
  including the editor-shortcut and deterministic macOS-platform suites;
- Tauri CLI 2.11.4 `--debug --bundles app --target
  aarch64-apple-darwin --no-sign` produced `Rho.app`; `file` and `lipo`
  reported an arm64 Mach-O desktop executable linked to Apple's Security and
  WebKit frameworks;
- the standard `Rho.app` executable, with `RHO_RSCRIPT` unset, passed the
  complete bundled smoke including Workspace execution, Plot, Environment,
  Viewer, stale rejection, two-project isolation, interrupt, restart, crash
  recovery, and durable events;
- `git diff --check`.

Unsigned development-app acceptance passed with an acceptance-only product
name and bundle identifier so existing Rho application state was not reused.
The same built code opened a temporary Git project whose name contained spaces
and Unicode, displayed its canonical `/private/var` root, and excluded a
project symlink targeting outside the root. The installed UI then demonstrated:

- Workspace R 4.5.2 startup and `getwd()` at the canonical project root;
- explicit Quarto discovery at `/usr/local/bin/quarto` with version 1.8.27 and
  a truthful `Quarto ready` Environment projection;
- a Console-created data frame and Plot, Environment object projection,
  Outputs preview, Logs, and a zero-state Problems panel;
- creation and immediate file-tree observation of
  `mac3-watcher-研究.R`, Git projection as one untracked working change, and
  Command+Enter execution returning `42`;
- the Agent/model-settings surface's presentation-safe system-store wording
  and existing environment-source projection without displaying, entering, or
  transmitting a credential; no provider-network request was made;
- Workspace R restart from the UI followed by a return to `R idle`; the exact
  bundle smoke separately covered interrupt and crash recovery.

After the app quit, no process command referenced the acceptance bundle or its
Ark child. The isolated project, application support, cache, and preferences
were moved to Trash under explicit MAC3 acceptance names; existing Rho state
and an unrelated already-running Ark session were left untouched.

Separate contract review found no blocking ownership or scope deviation. The
implementation changes no public protocol, persistence schema, project or
approval authority, Workspace/Agent process ownership, update feed, signing,
release workflow, application/R-package version, or `NEWS.md`. The bridge
source change is the bounded WS1-L2 containment conformance repair authorized
at entry, and its exported package contract is unchanged. Cargo lockfile
content did not change because the Apple keyring transitive packages were
already resolved.

Explicitly open or not accepted:

- only `aarch64-apple-darwin` is installed locally, so Windows compilation,
  Windows installed regression, and NSIS packaging were not run. Static
  Windows platform, frontend fallback, and credential-selection regressions
  passed; this is not equivalent to a Windows runner.
- Command-click definition behavior is covered deterministically in frontend
  automation but was not claimed as a separate mouse-level manual result.
- no real provider-network Agent smoke was run and no real provider credential
  was written during UI acceptance; credential semantics are covered by
  injected tests and the isolated native-Keychain smoke.
- the development app was unsigned and is not a DMG, exact candidate,
  notarized artifact, quarantined clean install, upgrade/uninstall test, update
  publication, or public release.

Version outcome: application remains `0.4.0-dev.0`, R package versions are
unchanged, and `NEWS.md` is unchanged. Release decision: NO-GO. The mandatory
MAC3 stop is reached. MAC4 was subsequently entry-reviewed and authorized as
recorded above; MAC5 remains inactive until its own entry review and explicit
authorization are recorded.

## MAC4 Implementation Evidence — 2026-08-05

Implementation present:

- update schema version 1 now accepts and validates optional
  `artifacts.macos_aarch64` metadata without breaking Windows-only feeds. The
  Apple Silicon client reports stable `UPDATE_PLATFORM_UNAVAILABLE` when the
  current release has no Mac artifact; Windows admission retains its required
  Windows artifact. Rust tests cover both-platform success, legacy Windows
  compatibility, missing Mac, untrusted URL, hash, and size failures;
- the release-page generator accepts legacy `rho-0.2-release.json` evidence or
  exact cross-platform aggregate evidence. It validates tag, commit, channel,
  artifact names, URLs, sizes, hashes, and recognized platform keys, then
  renders platform-specific downloads without changing the redirect-only
  application update policy;
- `candidate-release.mjs` owns bounded platform, aggregate, acceptance, and
  publish-admission schemas. It recomputes local artifact/sidecar/evidence
  hashes, requires both platforms and exact source identity, caps evidence,
  and rejects missing/duplicate/foreign/stale, NO-GO, oversized, or
  content-mismatched inputs;
- `Build Rho Candidate / Rehearsal` resolves one full commit, runs parallel
  least-privilege Windows x64 and `macos-26` jobs, selects Xcode 26.6, stages
  the API `.p8` under `RUNNER_TEMP`, imports Developer ID credentials into a
  temporary Keychain, uses Tauri's App Store Connect variables, verifies
  notarization history/signature/staple/Gatekeeper/arm64/smoke gates, and
  unconditionally removes temporary Apple files and Keychain;
- draft assembly requires both platform jobs, rejects an existing tag or
  release, creates one draft prerelease, uploads the seven exact assets once,
  and re-reads names and sizes. It contains no asset deletion or replacement;
- `Publish Rho Candidate` is separately protected by environment
  `rho-release`. It downloads and re-hashes the exact draft, requires bounded
  MAC5 `status: passed` and `decision: GO` evidence matching aggregate SHA,
  version/tag/commit and the complete platform mapping, then performs only the
  draft-to-published state transition. It cannot build, upload, delete, rename,
  or replace assets;
- Pages automation now handles legacy or aggregate evidence only after a
  release is public, downloads and re-hashes both candidate platform-evidence
  assets, emits Windows and optional Apple Silicon downloads, and
  post-deployment checks every present recognized artifact. Draft construction
  cannot trigger Pages;
- application metadata, workspace lockfile packages, Tauri, frontend
  package/lockfile, browser mock, cache-busting, roadmap, and NEWS are
  synchronized at `0.4.0-dev.1`. R package versions remain unchanged because
  their exported contracts did not change.

Local automated and packaging evidence passed:

- candidate and update-site self-tests plus all 33 deterministic
  `scripts/test-*.mjs` suites and `node --check desktop/dist/app.js`;
- YAML parsing for all three affected workflows, shell syntax, Ark bootstrap
  checksum/missing/non-Mach-O failure fixtures, and `git diff --check`;
- `cargo fmt --all -- --check` and `cargo test --workspace --no-fail-fast`;
- complete `rho.bridge` and `rho.agent` testthat suites;
- Tauri CLI 2.11.4 release build with `--bundles app,dmg --no-sign --ci` on
  arm64 macOS 26.5.2 / Xcode 26.6 / R 4.5.2. The app metadata reports version
  `0.4.0-dev.1` and macOS 14.0 minimum; app and bundled Ark are arm64; the DMG
  mounted with `Rho.app`; and its bundled executable passed the complete
  Workspace smoke, including Viewer, stale rejection, two-project isolation,
  interrupt, restart, and crash recovery.

The local DMG was explicitly unsigned. Its locally observed bytes and hash are
not candidate evidence and are not recorded as a distributable identity.
Only `aarch64-apple-darwin` is installed locally and PowerShell is unavailable,
so Windows compilation/NSIS/smoke and the PowerShell metadata check are
`NOT RUN`; deterministic Windows contract/configuration tests passed but are
not a substitute for the hosted job.

No repository Apple secret, credentialed hosted run, signed/notarized artifact,
staple, Gatekeeper candidate assessment, candidate upload, aggregate hosted
evidence, GitHub draft, installed candidate, or Pages publication was created
or claimed. Local inspection found a developer signing identity, but MAC4 did
not use it because no exact App Store Connect notarization credential set was
provided and the contract forbids treating a partial local signature as a
candidate.

Post-test contract review found no blocking ownership, schema, policy,
persistence, authority, or sequencing deviation. The only implementation-time
correction was to build both `app,dmg`: Tauri cleans the app bundle after a
DMG-only build, while MAC4 must retain the app for exact architecture,
signature, Gatekeeper, and smoke checks. This stays inside the accepted bundle
targets and is statically enforced.

Version outcome: application is `0.4.0-dev.1`; R package versions are
unchanged; NEWS records the user-visible Mac and cross-platform update support.
Release decision: NO-GO. The MAC4 mandatory stop is reached. MAC5 remains
unauthorized, and no acceptance evidence upload, release publication, or live
Pages acceptance may proceed without its own activation.

## MAC4-R Local Implementation Evidence — 2026-08-05

Implementation present:

- the manual workflow has a safe-default `rehearsal` choice and uses the same
  complete Windows and signed/notarized macOS platform jobs for both modes;
- deterministic admission accepts only rehearsal/fork and candidate/main
  pairings, requires the workflow and source commit to be the current default
  `main`, and disables persisted credentials on all five checkouts;
- rehearsal aggregation has only `contents: read`, validates both platform
  inputs, deletes transient candidate aggregate evidence, and uploads exactly
  six platform files plus one bounded rehearsal record for 14 days;
- candidate draft assembly alone retains `contents: write` and is additionally
  guarded to `build_mode=candidate` in `YuLab-SMU/Rho`;
- rehearsal evidence has an exact schema, 256-KiB limit, fork repository, full
  source commit, Run ID/Attempt, and both platform artifact/evidence hashes.
  Candidate publish admission rejects it;
- credential cleanup now fails closed: the conditional always-run step removes
  and verifies absence of the temporary Keychain, `.p12`, and `.p8`, so final
  rehearsal evidence cannot follow an observed cleanup failure;
- intermediate artifact containers use Run-ID names and explicit v4 overwrite
  semantics for retry recovery, while final rehearsal artifacts are unique per
  Run Attempt.

Local automated evidence passed:

- `node scripts/candidate-release.mjs --test true`, including accepted and
  rejected repository/mode pairs, non-main workflow ref, foreign repository,
  invalid Run ID/Attempt, stale commit, output containment, byte budget, and
  candidate/publish type-confusion rejection;
- `node scripts/generate-update-site.mjs --test true`, all deterministic
  `scripts/test-*.mjs` suites, and JavaScript syntax checks;
- Ruby YAML parsing, actionlint 1.7.12, and `git diff --check` for the affected
  workflow and repository state.

The separate post-test review found no unresolved ownership, schema, secret,
permission, recovery, update, or release-authority conflict. The review added
the default-branch source guard and retry-safe intermediate artifact naming;
both corrections remain inside the authorized rehearsal boundary.

Version outcome: application and R package versions remain unchanged;
`NEWS.md` is unchanged because this lane is internal review tooling.

## MAC4-R Hosted Rehearsal Evidence — PASSED 2026-08-06

Failure and recovery chronology remained inside the authorized repair lane:

- run `31046586627` passed admission and validation but the original PKCS#12
  did not expose the configured Developer ID identity; it produced no platform
  or aggregate evidence and was canceled after success became impossible;
- run `31047725385` exposed three deterministic Windows portability defects.
  Its macOS job signed the app and submitted notarization, then failed with
  `NSURLErrorDomain Code=-1009` (`No network route`) while polling Apple. The
  temporary Apple credential cleanup passed;
- run `31052585442` proved those three repairs and exposed the remaining CRLF
  workflow-regex defect. The macOS job was canceled after the sibling Windows
  result made aggregate success impossible; cancellation cleanup passed;
- run `31065316772` at
  `6f55fac14ccbc291c87dced48dab96b84b35dba1` passed Windows and received an
  Accepted Tauri app submission, but correctly blocked aggregate evidence
  because the final DMG had not been independently submitted. Cleanup passed;
- run `31079170163` attempt 1 at
  `f951db593cd1d48c7a862431b691a852a37e840f` passed admission, complete
  Windows and macOS validation, both platform builds/smokes, and aggregate
  rehearsal assembly. It is the only run used for passing evidence; no result
  or artifact was composed across runs.

The successful macOS job selected Xcode 26.6, imported the exact Developer ID
identity into a temporary Keychain, signed the app, Ark and DMG, observed exact
arm64 architectures for both executables, and explicitly skipped Tauri's app
submission. Final DMG submission `40373d62-63be-4d75-bf22-0ed6c668b69c`
returned `Accepted`; DMG staple/validate, app and DMG Gatekeeper assessment
(`Notarized Developer ID`), complete Workspace smoke, platform evidence upload,
and unconditional `.p12`/`.p8`/Keychain cleanup all passed.

The final review-only Actions artifact is ID `8965129826`, named
`rho-0.4.0-dev.1-rehearsal-f951db593cd1d48c7a862431b691a852a37e840f-31079170163-1`,
37,992,384 bytes in the Actions API, retained through 2026-08-20. Independent
download verification found exactly seven regular files and passed both
checksum sidecars plus aggregate size/hash/content binding:

- Windows installer: 17,686,967 bytes,
  SHA-256 `9462d57f2f50dbbcbf182dcd300860a392738892a8fd815d2c14298b86d472f0`;
- macOS DMG: 20,391,851 bytes,
  SHA-256 `bb57b9f8cc1f8db1a3ea3c92ba3a7d28b2e43ff3acff95239ed77c5d1a983866`;
- Windows platform evidence SHA-256
  `5fe2628fd03b898fcaa533cfcf55187c013b3e73069b8ce3c0cc69e19dd65e41`;
- macOS platform evidence SHA-256
  `f3f679b929fd1a3801c6f7d376c6de84134ea5472eb0fb30edf00f212247f86b`;
- rehearsal evidence SHA-256
  `777b623e9bc83e79283074b763626781d4d73b8195b8baa7f33d9b52d441ecc1`.

Read-only post-run checks found no fork tag or Release and a 404 Pages state;
the candidate draft job was skipped. GitHub emitted Node 20 action-deprecation
and upload-action Node API deprecation warnings, but all affected steps passed;
updating action majors is a bounded follow-up, not a release-gate failure.

Release decision remains NO-GO. The MAC4-R mandatory stop is reached. The main
repository must rebuild any authoritative candidate; MAC5 remains unauthorized.

## MAC4-R2 Installed-App Library Validation Repair — PASSED 2026-08-06

The installed `0.4.0-dev.1` rehearsal DMG passed signature, notarization,
staple, Gatekeeper, and CI Workspace smoke, but failed on the project owner's
Apple Silicon Mac with official CRAN R 4.5.2. Runtime discovery completed and
recorded arm64 R successfully; bundled Ark then received `kernel_info_request`
and panicked while opening `libR.dylib`. The bounded Ark log recorded macOS
rejecting the mapping because the process and library had different Team IDs:

- signed Rho and bundled Ark: `GAAY6Z9874`;
- official CRAN `libR.dylib`: `VZLD955F6P`;
- both Rho executables used hardened runtime with an empty entitlement set.

This proves a release-blocking installed-app defect, not missing R. The
`0.4.0-dev.1` rehearsal remains historical automation evidence but its DMG is
rejected and may not enter candidate, MAC5, update, or publication evidence.
Because it was distributed and installed, replacement behavior must use the
new single-use identity `0.4.0-dev.2`.

Apple's hardened runtime allows libraries signed by Apple or the executable's
own Team ID by default. Rho intentionally embeds Ark as an R front end while R
is installed and signed independently, so the exact required exception is
`com.apple.security.cs.disable-library-validation = true`. Tauri applies the
configured macOS entitlement file while signing each executable target; both
the app executable and Ark therefore receive the same exact one-key plist.
No App Sandbox, DYLD environment, JIT, unsigned executable memory, debugger,
or executable-protection exception is authorized.

The repair acceptance gate requires all of the following:

- the tracked plist has exactly the one authorized Boolean entitlement;
- negative tests reject missing, false, unknown, malformed, oversized, and
  symlinked entitlement evidence;
- the hosted job extracts the final code-signature entitlements from both
  `rho-desktop` and bundled Ark, validates the exact key set, and does so before
  notarization, Gatekeeper, and Workspace smoke;
- a temporary locally re-signed copy of the installed app starts bundled Ark
  and completes Workspace smoke against the observed official CRAN arm64 R;
- the exact `0.4.0-dev.2` fork commit passes the full Windows/macOS rehearsal,
  final-DMG notarization, staple, Gatekeeper, smoke, and cleanup gates;
- no tag, Release, Pages state, candidate evidence, or MAC5 acceptance is
  created.

Version impact: application version and synchronized frontend metadata advance
to `0.4.0-dev.2`; R package versions remain unchanged because their package
contracts do not change. `NEWS.md` receives a Fixed entry only after the local
runtime regression passes; that entry is now present because the regression
passed. Release decision remains NO-GO.

Local implementation evidence passed on 2026-08-06:

- `Entitlements.plist` contains exactly the one authorized Boolean key, and
  `plutil -lint` passes;
- the bounded validator accepts the exact JSON projection and rejects null,
  array, missing, false, extra-key, malformed, oversized, and symlink inputs;
- the MAC4 contract test proves final Rho/Ark signature extraction and
  validation precede final-DMG submission, receipt validation, staple, and
  Gatekeeper;
- an isolated copy of the rejected installed app was re-signed with the new
  plist; both executable signatures validated and the complete Workspace smoke
  passed against official arm64 R 4.5.2, including project isolation,
  interrupt, restart, and crash recovery;
- Tauri CLI 2.11.4 then built and Developer-ID-signed the actual
  `0.4.0-dev.2` app; extracted final `rho-desktop` and Ark signatures each
  passed the exact entitlement validator, `codesign --verify --deep --strict`
  passed, and the same complete Workspace smoke passed;
- all deterministic `scripts/test-*.mjs` suites, JavaScript syntax,
  candidate/update self-tests, workflow YAML parse, both R package suites,
  Rust format, and the complete Rust workspace tests passed. The existing
  unrelated Rust dead-code warnings remained; `actionlint` was unavailable
  locally and is not reported as run.

The replacement exact-commit rehearsal passed in run `31097468979` attempt 1
at `5b33a8f7e09a8e1466afd88cca117cf505cdd98f`. Windows completed in about 17
minutes; macOS completed in about 216 minutes, with Apple queue wait accounting
for nearly all time after build/signing. Actions artifact ID `8972987578` is
named
`rho-0.4.0-dev.2-rehearsal-5b33a8f7e09a8e1466afd88cca117cf505cdd98f-31097468979-1`
and is retained through 2026-08-20.

Independent download verification found exactly seven files:

- Windows installer: 17,686,318 bytes, SHA-256
  `1615c8bd3383ef821e6289ee611c779707dbcec88469f0f4cdefe1b34ba0d343`;
- macOS DMG: 20,393,780 bytes, SHA-256
  `789919effd0fcddd61d7a12dcd2e0cf4cd5d51bf96ab8217566d0b255293f5de`;
- Windows evidence SHA-256
  `17165c8fec4eb7ecff6fc91518ef6becc864cd7271563a70a47a718e7bfa9890`;
- macOS evidence SHA-256
  `b8a32655e75642f455df36348e288f29fd50b3b74cbb0c9fe172920750b34d24`;
- aggregate rehearsal evidence SHA-256
  `213342874a00cc558f85636de69450aa2020f2f14790f3f87ffab2877f29e4c3`.

Final-DMG notarization, staple, Gatekeeper, exact Rho/Ark entitlement checks,
Workspace smoke, cross-platform evidence binding, and credential cleanup all
passed. Read-only audit found no tag, Release, draft, or Pages publication.
MAC4-R2 is closed as review-only fork evidence. Upstream integration makes
`0.4.0-dev.16` the next candidate, and MAC4-R3 must rehearse that exact source.
Release decision remains NO-GO; MAC5 remains unauthorized.

## MAC4-R3 Upstream Integration Entry Evidence — PASSED 2026-08-07

The clean fork branch at
`5b33a8f7e09a8e1466afd88cca117cf505cdd98f` fetched upstream `main` at
`28ba1345efe70d28dc34214e5cc3ef03542c8122`. The histories had 14 fork-only
and 11 upstream-only commits, so synchronization used an ordinary merge rather
than a rebase, hard reset, force push, or history rewrite.

Conflict review preserved both contracts rather than choosing one branch
wholesale:

- upstream's current-user first-start root supersedes the old development-path
  fallback;
- Agent R keeps the macOS runtime-discovery `PATH`, while upstream's credential
  simplification removes `R_ENVIRON_USER` and every process-environment secret
  fallback;
- upstream's UI, output-review, KaTeX, Agent, and repair work is retained with
  the macOS platform/runtime/signing changes;
- the two independently reused `dev.1/dev.2` histories remain documented as
  historical evidence, while every live application authority advances to
  `0.4.0-dev.16`.

Post-resolution validation passed: `cargo fmt --all -- --check`; the complete
Rust workspace with 284 passed tests and one opt-in native-Keychain test
ignored; complete `rho.bridge` and `rho.agent` testthat suites; all 38
deterministic `scripts/test-*.mjs` suites; candidate-release and update-site
self-tests; JavaScript syntax; workflow YAML parsing; and `git diff --check`.
Two stale upstream frontend assertions were corrected to the already-required
camelCase Tauri argument contract and `dev.16` asset identity. `actionlint` was
not installed as a standalone binary at this checkpoint; the later MAC4-R3
validation invoked version 1.7.7 through Go and is recorded below.

This entry closed only the upstream integration prerequisite. At that
checkpoint MAC4-R3 implementation and the exact-commit hosted rehearsal were
open; the subsequent local implementation evidence is recorded below. MAC5
and all publication mutations remain unauthorized.

## MAC4-R3 Local Implementation Evidence — PASSED 2026-08-07

The candidate workflow now has three bounded macOS jobs. `macos-submit` builds,
signs, validates, smokes, and submits the copied final DMG exactly once with
`notarytool --no-wait`; it removes the temporary certificate, API-key file,
Keychain, receipt, and inherited credential-path variables before uploading
the immutable DMG/pending pair. `macos-notary-wait` runs on Ubuntu, receives
only the three team API-key secrets in its polling step, creates 120-second
ES256 JWTs in memory, and polls only the fixed submission/status and log
endpoints. `macos-finalize` receives no Apple secret, verifies the exact
repository/mode/version/tag/commit/Run/UUID/name/size/hash/log binding, staples
and mounts the same DMG, and repeats arm64, codesign, exact-entitlement,
Gatekeeper, and Workspace smoke gates before final platform evidence exists.

The built-in-Node implementation bounds receipts, API responses, evidence,
developer logs, artifact size, retry count, polling delay, and total wait. It
fails closed on unknown or terminal status, identity/name/log mismatch,
malformed or oversized input, untrusted log URL, authentication/not-found
responses, timeout, and cross-run or tampered finalizer input. Intermediate
artifact names are outside the final platform pattern, aggregate consumers now
depend on `macos-finalize`, and final macOS evidence requires the independent
`notary_binding` check.

Local evidence passed: all 39 deterministic `scripts/test-*.mjs` suites;
candidate-release and update-site self-tests; JavaScript syntax; workflow YAML
parsing; `actionlint` 1.7.7 with only its stale runner-label catalog warning for
the already exercised `macos-26` label suppressed; `cargo fmt`; the complete
Rust workspace with 284 passed and one opt-in native-Keychain smoke ignored;
complete `rho.bridge` and `rho.agent` testthat suites; and `git diff --check`.
Contract review found no change to application behavior, package contract,
entitlements, credential authority, or public protocol, so the integrated
application identity remains `0.4.0-dev.16` and `NEWS.md` does not change for
this CI-only orchestration slice.

At this local checkpoint the required exact-commit credentialed fork rehearsal
was NOT RUN; the subsequent failed-closed attempt is recorded below. The local
checkpoint created no tag, Release, draft, Pages state, acceptance record, or
MAC5 authority; the release decision remained NO-GO.

## MAC4-R3 Hosted Rehearsal Attempt 1 — FAILED CLOSED 2026-08-07

Fork Actions run `31160557569` checked out exact commit
`d40abeec0e3688e668a8fcf8d68a4e8bdf15e5f9`. The new `macos-submit` job
completed build, signing, exact architecture/entitlement checks, Workspace
smoke, one no-wait submission, credential cleanup, and immutable handoff in
7 minutes 57 seconds, proving that the multi-hour Apple wait no longer holds a
macOS runner. The Ubuntu waiter observed `In Progress` and then `Accepted`
roughly two minutes later, but deliberately rejected the authenticated `/logs`
response because its credential-free HTTPS URL used
`notary-artifacts-prod.s3.amazonaws.com`, outside the initial Apple/itunes-only
host allowlist. `macos-finalize` was skipped and no aggregate rehearsal,
candidate, tag, Release, draft, Pages state, or acceptance evidence was created.

A read-only query using the local Team API key reproduced HTTP 200,
`submissionsLog`, HTTPS/443, and that exact hostname without printing the
presigned URL, bearer token, or private key. The bounded repair adds only this
exact host; it does not allow arbitrary `amazonaws.com` or S3 buckets, redirects,
credentials in URLs, non-HTTPS, or non-443 ports. Regression tests must prove
the exact host succeeds and sibling/arbitrary S3 hosts remain rejected. The
same `0.4.0-dev.16` identity remains valid because attempt 1 emitted no final
platform or aggregate artifact and created no candidate/draft; a new commit and
full workflow Run ID are required for replacement evidence.

The exact-host repair passed its positive case plus arbitrary-bucket,
region-qualified sibling, and hostname-suffix-confusion negative cases in the
complete 39-suite deterministic matrix. A local read-only replay then
downloaded attempt 1's immutable pending/DMG artifact, queried the already
Accepted UUID through the patched implementation, retrieved the real developer
log, and passed UUID/status/filename/SHA-256 binding. No token, private key, or
presigned URL was printed or retained, and the temporary replay directory was
removed. The replacement hosted rehearsal remains required.

## MAC4-R3 Hosted Rehearsal Attempt 2 — FAILED CLOSED 2026-08-07

Fork Actions run `31161705717` checked out exact repair commit
`de63af75f0cc6e3aa3142725c1b4b8712c7221b3`. `macos-submit` released its
runner after 7 minutes 39 seconds. The Ubuntu waiter completed in 2 minutes
13 seconds, accepted Apple's exact S3 log delivery, and emitted log-bound
acceptance. The secret-free finalizer then passed pending/accepted/log/DMG
binding, staple and validation, DMG Gatekeeper, read-only mount, app/Ark
codesign, exact arm64, exact entitlements, and app Gatekeeper.

The final Workspace smoke failed when its fresh R installation reached
`jsonlite::toJSON()` and `loadNamespace()` without `jsonlite` installed.
`rho.bridge/DESCRIPTION` declares `jsonlite` as its only non-base `Imports`
dependency; the submission runner had it only because that earlier job runs
the complete R test-dependency installation. Finalization and aggregate
evidence remained absent, so no candidate, draft, tag, Release, Pages state,
acceptance record, or MAC5 mutation occurred.

The bounded repair adds a pre-smoke finalizer step that installs and verifies
only `jsonlite` in the temporary runner library from the already configured
public R repository. It does not bundle or mutate R, install bridge Suggests or
Agent dependencies, weaken the smoke, or claim clean-user-machine acceptance;
that installed-app condition remains owned by MAC5. Workflow regression tests
must bind this exact dependency step before final verification. Because attempt
2 created no final platform/aggregate evidence or candidate/draft, the same
`0.4.0-dev.16` identity remains valid for a new exact-commit full run.

The dependency repair passed the complete 39-suite deterministic matrix,
candidate/update self-tests, the exact R DESCRIPTION/namespace command,
workflow YAML, `actionlint` 1.7.7 with the known `macos-26` catalog suppression,
and diff checks. Static review proves the dependency step precedes immutable
finalizer verification, admits only `jsonlite`, and contains no bridge Suggests,
Agent dependency installation, Apple secret, or smoke bypass. This CI-only
repair has no application/version/NEWS impact. At that checkpoint, replacement
hosted evidence was still required.

## MAC4-R3 Replacement Hosted Rehearsal — PASSED / REVIEW-ONLY STOP REACHED 2026-08-07

Fork Actions run `31163017077` attempt 1 checked out exact repair commit
`8de3dcc1dafc9e8562d239a6051a9113b778f1c3` and completed successfully. The
macOS submission job ran for 12 minutes 23 seconds, including the complete
affected validation matrix, signing, Workspace smoke, final-DMG creation, one
no-wait submission, credential cleanup, and immutable handoff. Apple had
already accepted the request when the Ubuntu waiter made its first poll, so
the accepted/log-bound wait job completed in 8 seconds. The secret-free macOS
finalizer installed and verified only `jsonlite`, then passed original-DMG
binding, staple and validation, read-only mount, app/Ark codesign, exact arm64,
exact entitlements, both Gatekeeper assessments, and DMG-internal Workspace
smoke in 1 minute 28 seconds. Total macOS-runner use was therefore 13 minutes
51 seconds. Windows completed in 18 minutes 43 seconds, the review-only
aggregate completed in 7 seconds, and the whole workflow completed in about
19 minutes.

GitHub artifact ID `8988354217`, named
`rho-0.4.0-dev.16-rehearsal-8de3dcc1dafc9e8562d239a6051a9113b778f1c3-31163017077-1`,
has compressed size 38,699,030 bytes, GitHub digest
`sha256:36f27de38736b09051ddf41302073c55783c66f3450012f98976ee234b0985c6`,
and expires at `2026-08-21T09:04:33Z`. An independent post-run download
required exactly these seven non-empty regular files, rejected any directory
or symlink, recomputed every size and SHA-256, validated both platform schemas
and the rehearsal schema against the exact repository/version/tag/commit/run,
compared every aggregate record, and checked both checksum sidecars byte for
byte:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `Rho_0.4.0-dev.16_aarch64.dmg` | 20,786,895 | `d897e62566ff0ad1469c24c343bc2e955065f236d7d5ed4582321c35c04377ce` |
| `Rho_0.4.0-dev.16_aarch64.dmg.sha256` | 95 | `b9937b6cea86b91740a7e7cca2414dc0050dfd2b8cb1bf5bad3442296992dcf4` |
| `rho-0.4.0-dev.16-macos-aarch64-evidence.json` | 1,358 | `55d06e517c83d151b5e15a532cbd4434a01cc804bfe786f196e72a256e0f5709` |
| `Rho_0.4.0-dev.16_x64-setup.exe` | 17,999,268 | `038b6aa0ce61f2d255d977b302616ae063621601e51f124a376fb3c52089018a` |
| `Rho_0.4.0-dev.16_x64-setup.exe.sha256` | 97 | `a849ce540e0861fe56de22db72dded15ae492355c6488a3a022da271f5e47775` |
| `rho-0.4.0-dev.16-windows-x86_64-evidence.json` | 904 | `387cb518195d7b3bae23ca1a88011616ba46108683a3aa67a0caa528dc226738` |
| `rho-0.4.0-dev.16-rehearsal-evidence.json` | 1,582 | `be937442fa855e3fc718052797ede62c89c9a2dae4fd9977f6b15c3e53cf1d19` |

The macOS platform record contains all required checks, including independent
`notary_binding`, `notarization`, `staple`, `gatekeeper`, `arm64`, exact
`entitlements`, and `workspace_smoke`. The draft-release job was skipped.
Read-only repository audit found zero tags, zero Releases/drafts, and no Pages
site (`GET /pages` returned 404); the seven-file artifact contains no candidate
aggregate or MAC5 acceptance record. Thus no publication or acceptance state
was mutated.

GitHub emitted non-blocking warnings that the current checkout, Node setup,
upload-artifact, and download-artifact action revisions still declare the
retiring Node.js 20 runtime and were forced onto Node.js 24. The exact evidence
run passed; updating those action majors is a bounded follow-up and must not
rewrite this immutable rehearsal result. This CI-only slice does not change
application behavior or package contracts, so the version remained
`0.4.0-dev.16` and `NEWS.md` remained unchanged for that slice. MAC4-R3 was
complete at its mandatory review-only stop for that source. Release decision
remained NO-GO; an authoritative main-repository candidate/draft and MAC5
required separate authorization.

## MAC4-R3 Latest Upstream Refresh — PASSED / REVIEW-ONLY STOP REACHED 2026-08-07

A final pre-handoff fetch found that upstream `main` had advanced from
`28ba1345efe70d28dc34214e5cc3ef03542c8122` to
`b5800ae2dcb81da2ba90f4ba03d65cedffcc8d44` after run `31163017077`.
Commits `801e38e` and `b5800ae` narrow Check Project scanning to R-family and
extensionless source files, add its regression contract and NEWS entry, and
update the overlapping current-project/environment documents. They were
integrated by ordinary merge `9d3086e` without rebase, reset, force push, or
history rewriting; upstream `main` is an ancestor of the resulting branch.

The upstream source-filter contract remains its behavior owner. Integration
moved its user-visible NEWS bullet from upstream's `0.4.0-dev.15` section into
the combined `0.4.0-dev.16` section; no application metadata changes are
required because no authoritative `dev.16` candidate, draft, tag, or Release
exists. Local verification passed `cargo fmt --all -- --check`, the complete
Rust workspace with 285 passed and one opt-in native-Keychain test ignored
(including all 92 `rho-store` tests), all 39 deterministic JavaScript contract
suites, complete `rho.bridge` and `rho.agent` testthat suites, JavaScript
syntax, the human-friendly project-check UI contract, the MAC4 release
contract, and `git diff --check`.

The passing artifact from run `31163017077` remains immutable historical
evidence for exact commit `8de3dcc1dafc9e8562d239a6051a9113b778f1c3`, but
it cannot validate source added afterward. Initial refresh dispatch
`31165206877` supplied a nonexistent manually expanded input ref. The identity
job rejected it in 7 seconds, every platform/aggregate/draft job skipped, and
the run created zero artifacts. The corrected dispatch used Git's exact full
SHA and did not reuse that failed run.

Replacement run `31165265090` attempt 1 checked out exact post-merge commit
`c4661bbe25dcc326737c51b385c65865a795edb9` and passed. Identity took
6 seconds; macOS submit/validation/build/sign/smoke/one-submit/cleanup took
10 minutes 44 seconds; the Ubuntu Accepted/log waiter took 12 seconds; the
secret-free macOS staple/Gatekeeper/Workspace finalizer took 1 minute
26 seconds; Windows took 17 minutes 27 seconds; and review-only aggregation
took 9 seconds. Total macOS-runner use was 12 minutes 10 seconds and the whole
workflow completed in about 18 minutes 6 seconds.

GitHub artifact ID `8989186831`, named
`rho-0.4.0-dev.16-rehearsal-c4661bbe25dcc326737c51b385c65865a795edb9-31165265090-1`,
has compressed size 38,699,930 bytes, GitHub digest
`sha256:ac346d98b9a93af78ddce3fb7b14a143114babe2a619e6dcd0afdbd088cf6e13`,
and expires at `2026-08-21T09:35:31Z`. Independent post-run download again
required exactly seven non-empty regular files, rejected non-files and
symlinks, recomputed all sizes and hashes, validated both platform and
rehearsal schemas against the exact repository/version/tag/commit/run, checked
all aggregate records, and compared both checksum sidecars byte for byte:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `Rho_0.4.0-dev.16_aarch64.dmg` | 20,786,925 | `1e8b2623852af68bb7bf5ffffa66d8f58a0f097be00d3c09bc02d3b67ee0a66c` |
| `Rho_0.4.0-dev.16_aarch64.dmg.sha256` | 95 | `fb971878188aef33af3d38412bc635544df57666f667a888b8935289f027db3f` |
| `rho-0.4.0-dev.16-macos-aarch64-evidence.json` | 1,358 | `e03a2205a3203f156b04a1464cf9d8830fe91ae9e144531636770387e65d19d5` |
| `Rho_0.4.0-dev.16_x64-setup.exe` | 18,000,150 | `8336f2c513a35708ec5dede230b9d67c27a2ddc9e4b212baee514a13f0316cd3` |
| `Rho_0.4.0-dev.16_x64-setup.exe.sha256` | 97 | `9b72e14b1ee8b6ab4fa0ea27cd738f31a36a434ac8c805f0b97104736326fb05` |
| `rho-0.4.0-dev.16-windows-x86_64-evidence.json` | 904 | `db2235adff7077318251d7d037b78b367e6d83582296d8fe5f5e05cbce3111d3` |
| `rho-0.4.0-dev.16-rehearsal-evidence.json` | 1,582 | `4902575f84d3ef4c3c5bb9ed666ccbbe4d9618d9a71d73ce40daa10789a2a88c` |

Every required macOS record, including `notary_binding`, notarization, staple,
Gatekeeper, exact arm64/entitlements, and DMG-internal Workspace smoke, passed.
The draft job skipped. Read-only audit again found zero tags, zero Releases or
drafts, and no Pages site (`GET /pages` returned 404); the artifact contains no
candidate aggregate or MAC5 record. A final fetch confirmed upstream remained
exactly `b5800ae` and was an ancestor of the rehearsal commit.

The same non-blocking Node.js 20 action-runtime warnings remain a bounded
follow-up; GitHub forced those revisions onto Node.js 24 and the immutable run
passed. The refreshed MAC4-R3 review-only stop is closed. Release decision
remains NO-GO; MAC5 and every authoritative candidate/publication mutation
remain unauthorized.

## CRED-UX2 Application Identity Amendment — 2026-08-07

Issue #4 completes a user-visible Model settings workflow under the active
system-credential/LLM specification. It does not change the macOS runtime,
Keychain adapter, entitlements, notarization protocol, artifact schema, update
protocol, or MAC4/MAC5 authority. It does require the shared application
identity to advance from `0.4.0-dev.16` to `0.4.0-dev.17` across Cargo, Tauri,
frontend package/lockfile, browser mock, cache-busting, workflow defaults,
roadmap, checklist, and `NEWS.md`; R package versions remain independent and
unchanged.

The `0.4.0-dev.16` rehearsal checklist is historical. Its exact-commit runs,
notarization receipts, artifacts, hashes, and zero-publication audits remain
valid only for their recorded source and cannot satisfy any `dev.17` gate. The
active `0.4.0-dev.17` checklist owns all future candidate, installed-acceptance,
and GO/NO-GO facts. No authoritative workflow, tag, Release, draft, MAC5, Pages,
or publication action is authorized by this amendment.

Local implementation evidence completed on 2026-08-07. The complete affected
JavaScript, release-contract, Rust workspace, R package, Ark fixture, workflow,
format, syntax, and diff matrix passed. Tauri CLI 2.11.4 produced an unsigned
arm64 `Rho.app` and `Rho_0.4.0-dev.17_aarch64.dmg`; both the main executable and
bundled Ark are arm64, the app reports macOS 14.0 minimum and version
`0.4.0-dev.17`, `hdiutil verify` passed, and Workspace smoke passed from both
the app and a read-only mounted DMG. The local DMG is 21,079,685 bytes with
SHA-256 `0f919f8366bade4d12554be87bf07f9117cbeac04397de9e7447935555516f76`.
These are development facts only: signing, notarization, Gatekeeper,
authoritative candidate evidence, exact installed acceptance, MAC5, and
publication remain `NOT RUN` for `dev.17`.

## CRED-UX3 Application Identity Amendment — 2026-08-07

Provider model discovery is a user-visible desktop workflow owned by the
active system-credential/LLM specification. It does not change the macOS
runtime, Keychain adapter, entitlements, notarization protocol, artifact
schema, update protocol, or MAC4/MAC5 authority. It advances the shared
application identity from `0.4.0-dev.17` to `0.4.0-dev.18` across Cargo,
Tauri, frontend package/lockfile, browser mock, cache-busting, workflow
defaults, roadmap, checklist, and `NEWS.md`; R package versions remain
unchanged.

The `0.4.0-dev.17` checklist and its local app/DMG evidence become historical.
No prior artifact, hash, test result, notarization receipt, or acceptance fact
can satisfy a `dev.18` row. The active `0.4.0-dev.18` checklist owns all future
candidate, installed-acceptance, and GO/NO-GO facts. This amendment authorizes
only a local unsigned arm64 development candidate; authoritative workflows,
tag, Release, draft, MAC5, Pages, and publication remain unauthorized.

## CRED-UX4A Application Identity Amendment — 2026-08-07

The separately authorized capability-routing foundation joins the still-unused
`0.4.0-dev.18` integration identity and supersedes the pre-redesign local
artifact. It does not change the macOS runtime, Keychain adapter, entitlement,
notarization, artifact, update, MAC4, or MAC5 authority. Its exported Agent R
session route contract independently advances `rho.agent` to `0.1.3`.

The complete affected automation and deterministic browser review passed.
Tauri CLI 2.11.4 then produced a post-redesign unsigned arm64
`Rho_0.4.0-dev.18_aarch64.dmg`. `hdiutil verify` passed; the read-only mounted
app and bundled Ark are arm64; the app reports version `0.4.0-dev.18` and
minimum macOS 14.0; and its complete Workspace smoke passed. The DMG is
21,166,579 bytes with SHA-256
`75d6cdf20affb75ca94b5a81050c321eb41975b14c0a43bea2c40a9652da2723`.
These are local development facts only. Signing, notarization, staple,
Gatekeeper candidate assessment, real Keychain/live-Provider acceptance,
authoritative candidate evidence, MAC5, and publication remain `NOT RUN` and
unauthorized.

## CRED-UX4A-R1 Replacement Identity Amendment — 2026-08-07

The owner installed and rejected the exact `0.4.0-dev.18` DMG after its first
settings read failed and the composer disabled the only Model settings entry.
That identity, artifact, and hash are historical and cannot be rebuilt or
relabelled. The explicitly authorized recovery/provider package advances every
shared application authority to `0.4.0-dev.19` and independently advances
`rho.agent` to `0.1.4` for the reviewed `aisdk.providers` adapter contract.

This amendment does not change R discovery, bundled Ark, Keychain ownership,
entitlements, notarization, update schema, or MAC4/MAC5 authority. It permits
the complete affected local matrix and one unsigned arm64 replacement app/DMG
with architecture, metadata, `hdiutil`, and mounted Workspace smoke. Signing,
notarization submission, Gatekeeper candidate assessment, a hosted candidate,
tag, Release/draft, MAC5, Pages, and publication remain `NOT RUN` and
unauthorized.

The complete affected local matrix, independent security/contract review, and
deterministic browser review passed. Tauri CLI 2.11.4 produced the authorized
unsigned `Rho_0.4.0-dev.19_aarch64.dmg`; it is 21,213,923 bytes with SHA-256
`8fbe232b92b752216e907743cba45316acaaae1e0b20c5f9a12e77c6122906c1`.
`hdiutil verify` passed; the read-only mounted app and bundled Ark are exactly
arm64; the bundle reports `0.4.0-dev.19` and macOS 14.0 minimum; and mounted
Workspace smoke passed. The bundle has only linker ad-hoc signing, so no
Developer ID, notarization, staple, or Gatekeeper candidate claim is made.
The final credential review also verified the repair that forces reviewed
runtime endpoint defaults and the explicitly injected system-store key rather
than accepting undeclared ambient Provider values.

## Issue #6 `dev.20` Local Packaging Handoff — 2026-08-08

Issue #6 changes durable execution diagnostics and the Problems-to-Agent UI,
so it supersedes `dev.19` artifact evidence without changing Ark discovery,
Keychain, entitlements, signing, notarization, update schema, or MAC4/MAC5
authority. Tauri CLI 2.11.4 produced the local unsigned arm64
`Rho_0.4.0-dev.20_aarch64.dmg`; the final post-format build is 21,224,710
bytes with SHA-256
`9f011b15ab90c792ac177c3c0a87b530d8f92279fd77c1d9e362645ba11073ca`.
`hdiutil verify` passed; the read-only mounted app and bundled Ark are exactly
arm64; the bundle reports `0.4.0-dev.20` and macOS 14.0 minimum; and mounted
Workspace smoke passed. The build explicitly used `--no-sign`, so it provides
no Developer ID, notarization, staple, Gatekeeper, installed-user, MAC5, or
publication evidence.

## Issue #6 `dev.21` Corrective Local Packaging Handoff — 2026-08-08

The owner-installed `dev.20` rejection exposed a registered Provider runtime
identity mismatch before the repair task could contact its admitted model and
a selected Data Viewer that retained its old token after Workspace R changed.
The authorized PROBLEMS-AGENT-REPAIR-3, CRED-UX4A-R2, and WS3-Q1-R1 correction
therefore advances the replacement identity to `0.4.0-dev.21` without changing
Ark discovery, Keychain ownership, entitlements, signing/notarization policy,
update schema, or MAC4/MAC5 authority.

From clean feature-branch source commit
`ee96146e5c3760b38b729e78b60d596a08bd995b`, Tauri CLI `2.11.4` produced the
local unsigned arm64 `Rho_0.4.0-dev.21_aarch64.dmg`. It is 21,234,112 bytes
with SHA-256
`e52a37305eea076275e4c6eb88a7bb3e9faba9db71fec1161c13d5e7c5cd657f`.
`hdiutil verify` passed; the read-only mounted app and bundled Ark are exactly
arm64; the bundle reports `0.4.0-dev.21` and macOS 14.0 minimum; and complete
mounted Workspace smoke passed, including Plot, Environment, paged Data View,
stale-token rejection, two-project and restart isolation, interrupt recovery,
and crash recovery. The bundle has linker ad-hoc signing only. No Developer ID,
notarization, staple, Gatekeeper, installed-user, live-Provider/Keychain, MAC5,
hosted candidate, or publication claim is made.

## Issue #6 `dev.22` Console Repair Local Packaging Handoff — 2026-08-08

The owner rejected `dev.21` because repair remained discoverable only after
navigating away from the just-failed Console to Problems. The authorized
PROBLEMS-AGENT-REPAIR-4 correction exposes the shared durable repair action at
the Console error site while Problems remains the diagnostic-history
authority. This user-visible behavior advances the replacement application
identity to `0.4.0-dev.22` without changing Ark discovery, Keychain ownership,
entitlements, signing/notarization policy, update schema, or MAC4/MAC5
authority.

From exact clean feature-branch source commit
`2b5809151a154c0ca35c092f2e53dd7e064ab11b`, Tauri CLI `2.11.4` produced the
local unsigned arm64 `Rho_0.4.0-dev.22_aarch64.dmg`. It is 21,224,605 bytes
with SHA-256
`6550ebd87d1def65c317eeb9e8de12077c39cfa3c0b3ae00bee8dea7df0e0c50`.
`hdiutil verify` passed; the read-only mounted app and bundled Ark are arm64;
the bundle reports `0.4.0-dev.22` and macOS 14.0 minimum; and complete mounted
Workspace smoke passed, including Plot, Environment, paged Data View,
stale-token rejection, two-project and restart isolation, interrupt recovery,
and crash recovery. The bundle has an ad-hoc signature and no Team ID. No
Developer ID, notarization, staple, Gatekeeper, owner-installed-user,
live-Provider/Keychain, MAC5, hosted candidate, or publication claim is made.

## Issue #9 `dev.24` Hosted Rehearsal And Candidate Entry — 2026-08-09

The owner accepted the exact `0.4.0-dev.24` local development-app experience,
requested integration of the reviewed Issue #9 branch into
`YuLab-SMU/Rho_for_mac` `main`, and explicitly authorized the macOS build
workflow. The only macOS signing/notarization dispatch surface is the combined
`Build Rho Candidate / Rehearsal` workflow. This amendment therefore permits
one review-only `rehearsal` dispatch against the exact post-integration default-
branch commit, including the workflow's coupled Windows validation job.

Run `31294667960` completed successfully against exact commit
`c83ddfb4563778c1bf6190bd5ce833bb0a6a2e72`. Both platform matrices passed;
the macOS lane passed Developer ID signing, entitlements, notarization and its
binding, staple, Gatekeeper, and mounted-DMG Workspace smoke. GitHub artifact
`9032765026` contains the seven expected review-only files. Independent
download validation confirmed both platform hashes and admitted the platform
and rehearsal records against the exact repository/version/tag/commit/run.
The macOS DMG is 20,967,642 bytes with SHA-256
`b28b70e285e770b17836ab9c3bd3524fff23c86e153ccfd8e3138419dd9db6ce`.

After the same commit was fast-forwarded to `YuLab-SMU/Rho/main`, the owner
requested "开始发布". This activates one authoritative `candidate` build/draft
dispatch against the exact post-amendment default-branch commit. The candidate
workflow must rebuild and bind its own immutable files; the fork artifacts may
not be copied or relabelled. This authorization permits draft prerelease
creation but does not convert local development-app or rehearsal review into
installed-DMG acceptance. MAC5 GO, the `Publish Rho Candidate` workflow,
update-site mutation, and public publication remain blocked until the owner
installs and accepts the exact draft candidate.

## `dev.32` Candidate Failure And `dev.33` Replacement — 2026-08-11

Protected candidate run `31552396659` selected exact current upstream
`main@29faba2b4d08bbebb4d9e2e251e7e1d69d393d6f`. Identity resolution passed.
The Windows lane passed complete validation and Workspace smoke, built
`Rho_0.4.0-dev.32_x64-setup.exe`, and uploaded immutable run artifact
`9125093343`. The macOS arm64 lane failed a Provider-discovery timeout unit-test
fixture during complete source validation, before credentials were imported,
an application or DMG was built, or Apple notarization was submitted. Draft
assembly was skipped, and no tag, Release, publication, or updater mutation
exists for `dev.32`.

The run-scoped Windows artifact consumes the single-use `dev.32` identity and
is non-composable. The owner explicitly authorized the bounded deterministic
test repair and push. The replacement identity is `0.4.0-dev.33`; it carries
the already integrated editor-envelope repair without changing macOS runtime,
Ark/R discovery, Keychain, entitlements, signing/notarization, artifact schema,
update schema, or MAC4/MAC5 authority. The active `dev.33` checklist owns every
future source, candidate, installed, acceptance, MAC5, publication, and updater
fact. A new candidate remains blocked until its exact source is integrated and
passes the complete protected source gates.

## `dev.39` Conditional Prerelease Cross-Review — 2026-08-13

CPREL1 does not weaken MAC4 artifact construction: exact arm64 identity,
Developer ID signing, entitlements, notarization binding, stapling, hosted
Gatekeeper assessment, mounted Workspace smoke, hashes, and aggregate evidence
remain mandatory. The owner's one-release conditional authorization applies
only to enabled-Gatekeeper human launch on an available user Mac, which is
recorded as `not_run` because local assessments are disabled. It is not a MAC5
pass and cannot be reused by another version. The download page must expose
that limitation while the update manifest retains its existing schema and
exact macOS artifact hash.
