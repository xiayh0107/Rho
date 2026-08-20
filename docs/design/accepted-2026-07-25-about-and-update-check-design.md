# About And Update Check V1 Specification

Date: 2026-07-25

Status: implementation active; MAC4 macOS artifact extension implemented and
locally verified on 2026-08-05; LIC-2 About legal notice and fixed bundled-
license reveal implemented and locally reviewed on 2026-08-12; exact hosted
candidate, live Pages deployment, and installed-app acceptance pending;
SP-READY1 manual-only network admission implemented and locally verified on
2026-08-11, with hosted integration and installed acceptance pending

Release inclusion boundary: this feature was implemented after the locked
`0.2.0-dev.12` candidate baseline. It is not retroactively part of that
candidate's release evidence. Including it in a `0.2.0` installer requires an
explicit release-spec/checklist amendment, a new exact candidate identity, and
rerunning all affected automated and manual P0 gates. Its Pages and
update-discovery acceptance cannot satisfy or retroactively block the existing
`dev.12` GO/NO-GO contract.

Target: Rho Windows desktop after `0.2.0-dev.12`

## 1. Goal

Give every Rho installation a visible, copyable build identity and let users
discover new releases without visiting or understanding GitHub.

V1 adds:

- a `Help` menu with `Check for Updates...` and `About Rho`;
- an About dialog that reports the installed Rho build and relevant runtime
  versions;
- a bounded update check against a static manifest under
  `https://yulab-smu.top/Rho/`;
- a Rho-hosted release page that explains the available version and links to
  the existing GitHub Release installer;
- release automation that publishes the page and update manifests from
  validated GitHub Release metadata.

The feature must improve support reports such as Agent startup crashes: a user
must be able to copy the exact Rho version and build commit without locating
files or running a command.

## 2. Product Decisions

### 2.1 Rho-owned update discovery endpoint

The application must not use the GitHub API as its primary update endpoint.
It checks one of these static HTTPS resources:

```text
https://yulab-smu.top/Rho/updates/stable.json
https://yulab-smu.top/Rho/updates/development.json
```

The custom domain is the stable product-facing entry point. The GitHub Release
remains the source of the V1 installer and authoritative publication record.

The public release page is:

```text
https://yulab-smu.top/Rho/
```

The page must identify the latest stable and development versions, publication
dates, checksums, and download actions. A user should not need to navigate the
GitHub repository or GitHub Releases interface to learn that an update exists.

### 2.2 Explicit release channels

The installed version determines the default channel:

- a version with prerelease identifiers, such as `0.2.0-dev.12`, uses
  `development`;
- a version without prerelease identifiers, such as `0.2.0`, uses `stable`.

V1 has no channel selector. A stable installation must never offer a
prerelease. A development installation may advance to a newer development
release or to a newer stable release when SemVer ordering says the stable
release is newer.

Versions must be parsed and compared as Semantic Versioning values. String,
date, release ID, and filename ordering are not acceptable substitutes. The
optional leading `v` in a release tag is not part of the application version.

### 2.3 Inform and redirect, but do not install (V1 compatibility boundary)

V1 may discover an update and open the Rho release page. It must not:

- download an installer in the background;
- execute an installer;
- replace application files;
- silently update or restart Rho;
- claim that a SHA-256 checksum is equivalent to a signed updater package.

The update result must say that installation is user initiated. Automatic
download and installation require a later signed-updater specification and
end-to-end installer acceptance. `plans/active-2026-08-15-tauri-native-
updater-spec.md` is the separately active contract: it authorizes a bounded
supported-platform source implementation, but no Release may claim that native
updating is enabled until its exact candidate, installed-app, and public
endpoint gates pass. This V1 section remains the compatibility contract for
the existing Pages/download manifest; it does not authorize native install by
itself.

### 2.4 Update failure never blocks the workbench

Update discovery is optional network activity. DNS failure, TLS failure,
timeout, proxy failure, HTTP error, invalid JSON, invalid metadata, or an
unavailable GitHub download must not prevent Rho startup, Workspace R startup,
Agent startup, or normal use.

Manual checks show an actionable result. Update discovery is manual-only;
startup does not schedule or perform an update request.

## 3. Scope And Non-goals

### 3.1 Included in V1

- Help menu integration in the current HTML menu bar;
- About dialog with installed build and runtime information;
- one-click copying of bounded support diagnostics;
- manual update checks;
- no automatic or startup update request, notification, throttle, or dismissed-
  version persistence;
- stable and development static manifests;
- a small Rho release/download page on the existing GitHub Pages custom domain;
- GitHub Actions publication of the page and manifests after a validated Rho
  release is published;
- tests for version identity, manifest validation, channel policy, update
  states, and failure behavior.

### 3.2 Deferred

V1 does not include:

- Tauri updater integration, update signing keys/signatures, automatic
  installer download/execution/restart, rollback, or repair **within V1**.
  Those behaviors are owned only by the separate active native-updater
  contract above; V1 must not implement or claim them by itself;
- hosting installers in the GitHub Pages repository;
- a domestic object-storage or CDN mirror;
- fallback mirrors or download-speed selection;
- delta updates;
- macOS or Linux packages in the accepted Windows V1 implementation; the
  separately active macOS arm64 contract may add one compatible discovery
  artifact only after its MAC4 package is authorized;
- an in-application release-notes browser;
- user-selected update channels or skipped-version preferences.

Because the V1 installer URL still resolves to GitHub Release infrastructure,
V1 improves release discovery for domestic users but does not guarantee that
the installer itself is reachable. That limitation must be stated on the
release page and in release acceptance records.

### 3.3 Authorized macOS extension boundary

The active macOS arm64 specification extends schema version 1 with an
optional `artifacts.macos_aarch64` entry using the existing artifact field
shape and validation policy. It does not change channels, endpoints, SemVer,
fetch limits, allowlists, user-initiated installation, or the prohibition on
automatic update execution. Existing Windows-only manifests remain valid; the
new `0.4.0-dev.1` candidate requires both Windows x64 and macOS arm64 entries.
Generation validates every present recognized artifact and rejects unknown
artifact keys. This extension is authorized only in MAC4 and uses the new
exact-candidate D4 checklist rather than amending `0.2.0-dev.12` evidence.

## 4. User Experience

### 4.1 Help menu

Add `Help` after the existing `Tools` menu. Its initial commands are:

```text
Check for Updates...
--------------------
About Rho
```

The menu must follow the existing keyboard, focus, outside-click, and ARIA
behavior of the other workbench menus.

### 4.2 About Rho

`About Rho` opens a compact modal. It shows:

- product name and application version;
- release channel;
- short build commit, with the full commit in copied diagnostics;
- Windows architecture;
- R version and selected `Rscript.exe` path when runtime bootstrap succeeded;
- Agent runtime status and `aisdk` version when available;
- the Rho license identity, a short corresponding-source statement, and an
  offline action that reveals the fixed bundled license file;
- links to the Rho website and source repository;
- `Copy Diagnostics` and `Close` commands.

Unavailable runtime facts must render as `Unavailable` or `Not started`, not as
blank text and not as a dialog failure. No credential value, environment value,
project content, user name, or model prompt may appear in copied diagnostics.

The legal notice is static product information, not part of `app_info` or the
copied diagnostics payload. It must identify Rho as `GNU AGPL v3.0 only`, say
that corresponding source is available from the Source Repository, and keep
both Source Repository and bundled-license actions keyboard/focus reachable.
The bundled-license action invokes a no-input desktop command that resolves a
constant Tauri resource; the WebView never supplies or displays a filesystem
path. Missing-resource failure is shown truthfully and does not close About.

This LIC-2 extension does not change the update endpoint, channel policy,
network allowlist, diagnostics schema, or user-initiated installation rule.

The copied text uses a stable, support-friendly shape:

```text
Rho: 0.2.0-dev.12
Channel: development
Build: 4090cf725c53ab657ba9dfc9743ec6159f27dcf9
Platform: windows-x86_64
R: R version 4.6.0
Rscript: C:\Program Files\R\R-4.6.0\bin\Rscript.exe
Agent runtime: available
aisdk: 1.5.0
```

### 4.3 Check for Updates

Manual invocation opens an update modal immediately in a `Checking...` state
and disables duplicate checks until the request finishes or times out.

The terminal states are:

| State | Required presentation | Primary action |
| --- | --- | --- |
| `update_available` | Installed version, available version, publication date, and bounded notes | `Install and Restart` on supported native-updater builds; otherwise `View Update` for V1 compatibility builds |
| `up_to_date` | Installed version and confirmation that it is current for its channel | `Close` |
| `check_failed` | Concise failure category without raw response bodies or secrets | `Try Again` |

For the active native updater on Windows x64 and Apple Silicon macOS,
`Install and Restart` is a second explicit user action. It downloads a pending
signed artifact, verifies it before stopping runtime work, then invokes the
platform installer/restart path. The WebView does not receive a generic
updater command, signing key, arbitrary endpoint, or installer arguments. An
unavailable platform, stale pending update, download/signature failure, or
installation failure is shown truthfully and never presented as installed.

`View Update` remains the V1 compatibility action: it opens the validated
`release_page_url` in the system browser. Neither action navigates the
workbench WebView away from Rho.

Rho does not check automatically. The update service is first contacted only
after the user selects `Check for Updates...` or `Try Again` in this dialog.

## 5. Application Information Contract

Add a Tauri command named `app_info`. It returns a typed payload equivalent to:

```json
{
  "version": "0.4.0-dev.1",
  "channel": "development",
  "commit": "4090cf725c53ab657ba9dfc9743ec6159f27dcf9",
  "platform": "windows-x86_64",
  "website_url": "https://yulab-smu.top/Rho/",
  "source_url": "https://github.com/YuLab-SMU/Rho",
  "runtime": {
    "rscript": "C:\\Program Files\\R\\R-4.6.0\\bin\\Rscript.exe",
    "r_version": "R version 4.6.0",
    "agent_available": true,
    "aisdk_version": "1.5.0"
  }
}
```

The version must come from Tauri package metadata, not a new frontend literal.
The build commit must be embedded by `build.rs`; local builds without Git
metadata may report `unknown`. The runtime fields reuse current startup state
and must not initiate another R or Agent probe.

The existing release metadata check continues to enforce agreement between the
Cargo workspace, Tauri config, and frontend package versions.

## 6. Legacy Update-Site Manifest Contract

This schema-v1 manifest remains the GitHub Pages download-site compatibility
format. It is not parsed by the supported native updater. The separate active
native-updater specification owns the Tauri manifest, its signature files, and
the native install path.

Each channel endpoint returns one UTF-8 JSON document:

```json
{
  "schema_version": 1,
  "channel": "development",
  "version": "0.2.0-dev.12",
  "published_at": "2026-07-22T14:45:23Z",
  "summary": "Agent reliability and release hardening fixes.",
  "release_page_url": "https://yulab-smu.top/Rho/",
  "github_release_url": "https://github.com/YuLab-SMU/Rho/releases/tag/v0.4.0-dev.1",
  "artifacts": {
    "windows_x86_64": {
      "url": "https://github.com/YuLab-SMU/Rho/releases/download/v0.4.0-dev.1/Rho_0.4.0-dev.1_x64-setup.exe",
      "sha256": "97bc0a0aad9889c9027e30e07dd3a5ef38885c43e5ace5dbb14aaf8bca0ef019",
      "size": 15854119
    },
    "macos_aarch64": {
      "url": "https://github.com/YuLab-SMU/Rho/releases/download/v0.4.0-dev.1/Rho_0.4.0-dev.1_aarch64.dmg",
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "size": 23456789
    }
  }
}
```

Required validation:

- `schema_version` equals `1`;
- `channel` equals the requested channel;
- `version` is valid SemVer;
- `published_at` is a valid RFC 3339 timestamp;
- `summary` is plain text and no more than 500 Unicode scalar values;
- `release_page_url` is HTTPS, its host is exactly `yulab-smu.top`, and its
  path is `/Rho/` or a descendant of `/Rho/`;
- `github_release_url` is HTTPS and belongs to
  `github.com/YuLab-SMU/Rho/releases/`;
- `artifacts.windows_x86_64` is required; `artifacts.macos_aarch64` is optional
  for backward compatibility, and unrecognized artifact keys are rejected by
  publication generation;
- every present artifact URL is HTTPS and belongs to the expected Rho GitHub
  Release download path;
- every present `sha256` is exactly 64 lowercase hexadecimal characters;
- every present `size` is a positive integer.

The V1 compatibility consumer must reject the entire manifest if a required
field fails validation. It must cap the response body at 64 KiB and use a
total request timeout of 10 seconds. Redirects for the manifest endpoint must
remain HTTPS and end on `yulab-smu.top`.

V1 displays the artifact checksum on the website but does not download the
artifact, so checksum verification is not a client V1 behavior.

## 7. Publication Contract

The existing Windows publish workflow remains the legacy authority for its
`0.2.x` release records. The `0.4.0-dev.1` cross-platform candidate instead
uses the active D4 checklist: parallel platform jobs produce evidence, a
separate assembly job creates one immutable draft, and a separately protected
publish workflow can only expose that draft after exact MAC5 GO evidence.
Neither candidate construction nor Pages automation may publish a draft.

After those assets exist, a separate Pages publication job must:

1. query published, non-draft releases for `YuLab-SMU/Rho`;
2. select the highest valid stable SemVer release for `stable.json`;
3. select the highest valid SemVer release, including stable and prerelease
   versions, for `development.json`; this promotes development installations
   to a newer stable release when appropriate;
4. require the expected Windows installer, checksum, and validated release
   evidence for every legacy release, or the complete Windows/macOS artifact
   and aggregate-evidence set for a cross-platform candidate;
5. generate the two manifests and the release page from those records;
6. validate all generated URLs, versions, sizes, and SHA-256 values;
7. deploy through GitHub Pages only after generation tests pass;
8. fetch the deployed manifests and verify their version and checksum values.

If no stable release exists, the website may state that stable is not yet
available and `updates/stable.json` may return `404`. A development manifest
must never be copied into the stable endpoint.

The workflow must regenerate the complete Pages artifact from GitHub Release
state. It must not require committing installers or accumulating generated
release binaries in the Rho Git repository.

A Pages deployment failure does not invalidate an already published GitHub
Release, but it leaves update discovery incomplete and must be reported as a
separate failed publication gate.

## 8. Backend And Frontend Boundaries

The Tauri backend owns:

- application/build identity;
- manifest HTTP retrieval, size and timeout limits;
- manifest parsing and validation;
- SemVer comparison and channel policy;
- opening only allowlisted HTTPS product URLs in the system browser.

The frontend owns:

- Help menu behavior;
- About and update modal rendering;
- loading, success, failure, and retry states.

The frontend must not fetch the manifest directly, compare versions, accept an
arbitrary URL from JSON, interpolate raw network errors into HTML, or schedule
an update request outside the explicit manual commands.

## 9. Privacy And Security

User-initiated update requests disclose the normal network metadata of an
HTTPS request, including IP address, request time, and user agent. Rho must not
append project, user, R, Agent, provider, or credential information to the
request URL or headers. Startup and ordinary workbench use make no update
request.

The V1 update checker does not execute remote content. All external navigation
is user initiated and constrained to the allowlisted product and repository
URLs. Manifest content is rendered as text, never as HTML.

The website must publish SHA-256 values from validated release evidence. It
must not describe an unsigned installer as cryptographically authenticated.

## 10. Implementation Work Packages

### WP1: Build identity and About

- embed the source commit during the desktop build;
- add the typed `app_info` command;
- add Help menu and About dialog;
- implement bounded diagnostic copying and safe external-link opening;
- add mock-mode data for frontend verification.

### WP2: Update domain model and backend checker

- define manifest and result types;
- add an HTTP client with response-size, redirect, and timeout policy;
- implement SemVer channel selection and comparison;
- return structured failure categories rather than raw request bodies;
- cover allowlist and malformed-manifest cases with Rust tests.

### WP3: Update user interface

- add manual checking, retry, terminal states, and external navigation;
- keep update discovery manual-only with no startup scheduler or update-
  throttle/dismiss persistence;
- ensure dialogs are keyboard accessible and fit the minimum desktop window;
- implement all update states in mock mode.

### WP4: Pages publication

- add deterministic manifest and release-page generation;
- configure the Rho project Pages path under `yulab-smu.top/Rho/`;
- publish only from validated, non-draft GitHub Releases;
- add post-deployment verification for both channel endpoints;
- keep installer binaries out of the Pages artifact and Git history.

The Rho repository deploys its project Pages artifact to the orphan
`gh-pages` branch because the repository Pages source is configured as legacy
branch publishing from `gh-pages:/`. GitHub then serves that branch at `/Rho/`
under the organization Pages custom domain. The workflow must not use
`actions/deploy-pages`, because that deployment is attributed to the invoking
branch and is rejected by the `github-pages` environment policy, which allows
only `gh-pages`.

Site publication is a separate `ubuntu-latest` workflow. It runs automatically
after a successful legacy Windows publication or protected cross-platform
candidate publication and may also be dispatched by itself. It reads only
already-published GitHub Releases and must not rebuild an installer merely to
refresh Pages. Live endpoint verification remains an open acceptance gate
until the corrected workflow has run successfully.

### WP5: Release integration and documentation

- extend release checks to validate build commit and website/update constants;
- record Pages deployment separately from installer publication evidence;
- add user-facing update behavior and limitations to release documentation;
- update `NEWS.md` only when implementation is complete.

## 11. Testing Strategy

### 11.1 Automated backend tests

Tests must cover:

- stable and development channel derivation;
- ordering of `dev.9`, `dev.10`, `dev.12`, release candidates, and stable
  versions;
- equal, newer, and older manifest versions;
- stable refusing a prerelease manifest;
- invalid schema, channel, SemVer, timestamp, URL, checksum, and size;
- oversized body, timeout, redirect escape, HTTP error, and invalid JSON;
- diagnostics redaction and unavailable runtime fields;
- builds with a known and unknown commit.

Network tests use a local test server or injected transport and must not depend
on the live website or GitHub.

### 11.2 Automated frontend tests

Static/mock verification must cover:

- Help menu commands and menu dismissal;
- About rendering with ready, unavailable, and not-started runtimes;
- diagnostic copy success and failure feedback;
- every update modal terminal state;
- duplicate-check suppression;
- absence of startup/background scheduling and update-throttle/dismiss
  persistence;
- safe text rendering for release summaries;
- keyboard focus entry, containment, Escape, and restoration.

### 11.3 Publication tests

Generation and workflow tests must prove:

- draft releases are ignored;
- stable and development releases cannot cross channels;
- missing platform artifact, evidence, or checksum fails generation;
- Windows-only legacy evidence remains valid, while aggregate candidate
  evidence rejects a missing macOS platform, unknown platforms, stale commit,
  and mismatched asset size or hash;
- manifest values match the GitHub Release asset and evidence JSON;
- the deployed page and both available manifests return HTTPS success;
- the expected version remains available after a second deployment.

### 11.4 Manual acceptance

Against the exact installed candidate:

- About shows the installer version and expected commit;
- copied diagnostics are accurate and contain no secret or project content;
- manual check reports the correct state when current and when older;
- stable does not offer a prerelease;
- offline and blocked-network checks are recoverable and do not affect Rho;
- startup and ordinary workbench use make no update request, and the first
  request follows the explicit manual command;
- a V1 compatibility build's `View Update` opens the Rho page in the system
  browser; supported native-updater builds instead verify and install only
  after the separate `Install and Restart` action;
- the Rho page is understandable without GitHub knowledge;
- the GitHub-hosted installer limitation is visible;
- menus and dialogs work at the minimum window size and with keyboard-only
  operation.

## 12. Acceptance Gates

V1 is implementation-complete only when all of the following are true:

1. the installed version and build commit are visible and copyable from About;
2. version identity comes from build/package metadata and remains covered by
   the existing three-source version-agreement check;
3. manual update checks use the correct `yulab-smu.top` channel endpoint and
   produce every specified terminal state;
4. startup performs no update request, and a manual update failure cannot block
   Workspace/Agent operation;
5. stable installations never offer prereleases;
6. manifest parsing, URL allowlisting, response bounds, timeout, and SemVer
   behavior have automated regression coverage;
7. the Pages workflow generates and post-deployment verifies the Rho page and
   all available channel manifests from validated GitHub Releases;
8. no installer binary is committed to Git history or included in the Pages
   artifact;
9. the exact candidate passes the manual acceptance cases above;
10. documentation states that V1 discovers updates but neither guarantees
    domestic installer reachability nor installs updates automatically.

Passing application tests alone does not prove the feature is operational.
The Pages deployment and live endpoint verification are required evidence, and
manual installed-application acceptance remains distinct from both.
