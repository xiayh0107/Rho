use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use directories::ProjectDirs;
use fs4::FileExt;
use reqwest::Client;
use rho_protocol::{
    DependencyAction, DependencyComponent, DependencyComponentStatus, DependencyIssue,
    DependencyPhase, DependencyReport, DependencySource, DependencyStatus,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

const MANIFEST_JSON: &str = include_str!("../../../runtime/dependencies.json");
const MAX_ARTIFACT_BYTES: u64 = 128 * 1024 * 1024;
const R_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const DEPENDENCY_SCHEMA_VERSION: &str = "1";
const BRIDGE_VERSION: &str = "0.1.0";

const BRIDGE_FILES: &[(&str, &str)] = &[
    (
        "DESCRIPTION",
        include_str!("../../../r/rho.bridge/DESCRIPTION"),
    ),
    ("NAMESPACE", include_str!("../../../r/rho.bridge/NAMESPACE")),
    ("LICENSE", include_str!("../../../r/rho.bridge/LICENSE")),
    ("R/state.R", include_str!("../../../r/rho.bridge/R/state.R")),
    (
        "R/execute.R",
        include_str!("../../../r/rho.bridge/R/execute.R"),
    ),
    (
        "R/workspace.R",
        include_str!("../../../r/rho.bridge/R/workspace.R"),
    ),
];

#[derive(Debug, Clone, Deserialize)]
struct DependencyManifest {
    schema_version: u32,
    r: RManifest,
    ark: ArkManifest,
}

#[derive(Debug, Clone, Deserialize)]
struct RManifest {
    requirement: String,
    recommended_version: String,
    #[serde(default)]
    installers: BTreeMap<String, InstallerArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
struct InstallerArtifact {
    url: String,
    sha256: String,
    minimum_os: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ArkManifest {
    version: String,
    artifacts: BTreeMap<String, ArkArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
struct ArkArtifact {
    url: String,
    sha256: String,
    size: u64,
    executable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallReceipt {
    schema_version: u32,
    component: String,
    version: String,
    target: String,
    source_url: String,
    archive_sha256: String,
    executable_sha256: String,
    installed_at: String,
}

#[derive(Debug, Deserialize)]
struct BindingReceipt {
    schema_version: u32,
    r_version: String,
    r_home: String,
    rscript: String,
    ark_version: String,
    ark: String,
}

#[derive(Debug, Deserialize)]
struct BindingKernelSpec {
    argv: Vec<String>,
    env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RRuntime {
    pub rscript_path: PathBuf,
    pub r_home: PathBuf,
    pub r_bin: PathBuf,
    pub library_paths: Vec<PathBuf>,
    pub version: Version,
    pub version_string: String,
    pub architecture: String,
    pub source: DependencySource,
}

#[derive(Debug, Clone)]
pub struct PreparedRuntime {
    pub kernelspec_path: PathBuf,
    pub bridge_package: PathBuf,
    pub ark_path: PathBuf,
    pub r: RRuntime,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EnsureOptions {
    pub offline: bool,
    pub repair: bool,
    pub install_r: bool,
}

#[derive(Clone)]
pub struct DependencyManager {
    project_root: PathBuf,
    project_runtime_root: PathBuf,
    cache_root: PathBuf,
    manifest: Arc<DependencyManifest>,
    client: Client,
    report: Arc<RwLock<DependencyReport>>,
    ensure_gate: Arc<Mutex<()>>,
    rscript_override: Option<PathBuf>,
    ark_override: Option<PathBuf>,
    bundled_ark: Option<PathBuf>,
}

impl DependencyManager {
    pub fn new(project_root: impl Into<PathBuf>) -> Result<Self> {
        let project_root = project_root.into();
        let project_root = project_root.canonicalize().with_context(|| {
            format!(
                "resolving dependency project root {}",
                project_root.display()
            )
        })?;
        ensure!(
            project_root.is_dir(),
            "dependency project root must be a directory"
        );
        let cache_root = dependency_cache_root(&project_root)?;
        let manifest: DependencyManifest =
            serde_json::from_str(MANIFEST_JSON).context("decoding embedded dependency manifest")?;
        ensure!(
            manifest.schema_version == 1,
            "unsupported dependency manifest schema"
        );
        let target = current_target();
        Ok(Self {
            project_runtime_root: project_root.join(".rho/runtime"),
            project_root,
            cache_root,
            manifest: Arc::new(manifest),
            client: Client::builder()
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(180))
                .build()
                .context("building dependency download client")?,
            report: Arc::new(RwLock::new(initial_report(&target))),
            ensure_gate: Arc::new(Mutex::new(())),
            rscript_override: env::var_os("RHO_RSCRIPT").map(PathBuf::from),
            ark_override: env::var_os("RHO_ARK").map(PathBuf::from),
            bundled_ark: None,
        })
    }

    pub fn with_cache_root(mut self, cache_root: impl Into<PathBuf>) -> Self {
        self.cache_root = cache_root.into();
        self
    }

    pub fn with_rscript(mut self, rscript: impl Into<PathBuf>) -> Self {
        self.rscript_override = Some(rscript.into());
        self
    }

    pub fn with_ark(mut self, ark: impl Into<PathBuf>) -> Self {
        self.ark_override = Some(ark.into());
        self
    }

    /// Registers an Ark executable from a trusted, application-packaged resource directory.
    /// The adjacent managed-install receipt is still checked against the embedded manifest and
    /// the executable is rehashed before use.
    pub fn with_bundled_ark(mut self, ark: impl Into<PathBuf>) -> Self {
        self.bundled_ark = Some(ark.into());
        self
    }

    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn prepare_bridge_package(&self) -> Result<PathBuf> {
        self.materialize_bridge()
    }

    pub async fn open_verified_r_installer(&self) -> Result<()> {
        let report = self.current_report().await;
        let component = report
            .components
            .iter()
            .find(|component| component.name == "r" && component.verified)
            .context("no verified R installer is ready to open")?;
        let path = PathBuf::from(
            component
                .path
                .as_deref()
                .context("verified R installer has no local path")?,
        );
        let path = path
            .canonicalize()
            .with_context(|| format!("resolving verified R installer {}", path.display()))?;
        let cache_root = self
            .cache_root
            .canonicalize()
            .unwrap_or_else(|_| self.cache_root.clone());
        ensure!(
            path.starts_with(&cache_root),
            "refusing to open an installer outside the Rho dependency cache"
        );
        #[cfg(target_os = "macos")]
        let mut command = std::process::Command::new("open");
        #[cfg(target_os = "linux")]
        let mut command = std::process::Command::new("xdg-open");
        #[cfg(windows)]
        let mut command = std::process::Command::new(&path);
        #[cfg(not(windows))]
        command.arg(&path);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("opening verified R installer {}", path.display()))?;
        Ok(())
    }

    pub async fn current_report(&self) -> DependencyReport {
        self.report.read().await.clone()
    }

    pub async fn inspect(&self) -> Result<DependencyReport> {
        let _gate = self.ensure_gate.lock().await;
        self.publish(
            DependencyStatus::Checking,
            DependencyPhase::Discovering,
            Vec::new(),
            None,
            Vec::new(),
        )
        .await;
        let result = self.inspect_locked().await;
        if let Err(error) = &result {
            self.publish_failure("dependency.inspect_failed", error, true)
                .await;
        }
        result
    }

    pub async fn ensure(&self, options: EnsureOptions) -> Result<Option<PreparedRuntime>> {
        let _gate = self.ensure_gate.lock().await;
        let result = self.ensure_locked(options).await;
        if let Err(error) = &result {
            self.publish_failure("dependency.ensure_failed", error, true)
                .await;
        }
        result
    }

    async fn inspect_locked(&self) -> Result<DependencyReport> {
        let target = current_target();
        let mut components = Vec::new();
        let r = match self.discover_r().await? {
            RDiscovery::Ready(runtime) => {
                components.push(r_component(&runtime, &self.manifest.r.requirement));
                Some(runtime)
            }
            RDiscovery::Missing => {
                components.push(component(
                    "r",
                    DependencyComponentStatus::Missing,
                    Some(self.manifest.r.requirement.clone()),
                    None,
                    None,
                    None,
                    false,
                    Some(format!(
                        "R {} is recommended; Rho never asks an Agent to install it",
                        self.manifest.r.recommended_version
                    )),
                ));
                None
            }
            RDiscovery::Invalid(detail) => {
                components.push(component(
                    "r",
                    DependencyComponentStatus::Invalid,
                    Some(self.manifest.r.requirement.clone()),
                    None,
                    Some(DependencySource::Explicit),
                    self.rscript_override
                        .as_ref()
                        .map(|path| normalized_path(path)),
                    false,
                    Some(detail),
                ));
                None
            }
            RDiscovery::Incompatible(runtime) => {
                components.push(component(
                    "r",
                    DependencyComponentStatus::Incompatible,
                    Some(self.manifest.r.requirement.clone()),
                    Some(runtime.version.to_string()),
                    Some(runtime.source),
                    Some(normalized_path(&runtime.rscript_path)),
                    true,
                    Some("The discovered R version is outside Rho's supported range".into()),
                ));
                None
            }
        };

        let ark = self.resolve_existing_ark(&target, false)?;
        components.push(match &ark {
            Some(resolved) => component(
                "ark",
                DependencyComponentStatus::Ready,
                Some(format!("={}", self.manifest.ark.version)),
                Some(self.manifest.ark.version.clone()),
                Some(resolved.source),
                Some(normalized_path(&resolved.path)),
                resolved.verified,
                None,
            ),
            None if self.manifest.ark.artifacts.contains_key(&target) => component(
                "ark",
                DependencyComponentStatus::Missing,
                Some(format!("={}", self.manifest.ark.version)),
                None,
                None,
                None,
                false,
                Some("A checksum-pinned Ark artifact is available for automatic install".into()),
            ),
            None => component(
                "ark",
                DependencyComponentStatus::Unsupported,
                Some(format!("={}", self.manifest.ark.version)),
                None,
                None,
                None,
                false,
                Some(format!(
                    "No verified Ark artifact is configured for {target}"
                )),
            ),
        });
        let binding = match (&r, &ark) {
            (Some(r), Some(ark)) => {
                let binding = self.resolve_existing_binding(r, &ark.path);
                components.push(binding_component(r, &self.manifest.ark.version, &binding));
                Some(binding)
            }
            _ => None,
        };
        components.push(bridge_component(None));

        let (status, ready, issue, actions) = if r.is_none() {
            let invalid = components
                .first()
                .is_some_and(|value| value.status == DependencyComponentStatus::Invalid);
            (
                DependencyStatus::ActionRequired,
                false,
                Some(r_issue(invalid, &self.manifest.r.recommended_version)),
                vec![DependencyAction {
                    id: "install_r".into(),
                    label: format!("Install R {}", self.manifest.r.recommended_version),
                    requires_human: true,
                }],
            )
        } else if ark.is_none() && self.manifest.ark.artifacts.contains_key(&target) {
            (
                DependencyStatus::ActionRequired,
                false,
                Some(DependencyIssue {
                    code: "ark.missing".into(),
                    title: "Ark needs to be prepared".into(),
                    message: "Rho can download and verify its pinned Ark runtime automatically."
                        .into(),
                    retryable: true,
                    requires_user_action: false,
                    action_url: Some("rho://setup/dependencies".into()),
                }),
                vec![DependencyAction {
                    id: "ensure".into(),
                    label: "Prepare Workspace R".into(),
                    requires_human: false,
                }],
            )
        } else if ark.is_none() {
            (
                DependencyStatus::Failed,
                false,
                Some(DependencyIssue {
                    code: "platform.unsupported".into(),
                    title: "Ark is not available for this platform".into(),
                    message: format!(
                        "Rho has no verified Ark {} artifact for {target}.",
                        self.manifest.ark.version
                    ),
                    retryable: false,
                    requires_user_action: true,
                    action_url: Some("rho://setup/dependencies".into()),
                }),
                Vec::new(),
            )
        } else if let Some(binding) = binding.as_ref().filter(|binding| !binding.is_ready()) {
            let invalid = matches!(binding, BindingResolution::Invalid { .. });
            (
                DependencyStatus::ActionRequired,
                false,
                Some(DependencyIssue {
                    code: if invalid {
                        "binding.invalid"
                    } else {
                        "binding.missing"
                    }
                    .into(),
                    title: if invalid {
                        "The Workspace R binding is stale"
                    } else {
                        "Workspace R needs a controlled binding"
                    }
                    .into(),
                    message: if invalid {
                        "The project binding does not match the selected R and Ark runtimes. Rho can regenerate it safely."
                    } else {
                        "R and Ark are ready, but this project does not yet have its controlled Workspace R binding."
                    }
                    .into(),
                    retryable: true,
                    requires_user_action: false,
                    action_url: Some("rho://setup/dependencies".into()),
                }),
                vec![DependencyAction {
                    id: "ensure".into(),
                    label: "Generate Workspace R binding".into(),
                    requires_human: false,
                }],
            )
        } else {
            (DependencyStatus::Ready, true, None, Vec::new())
        };
        Ok(self
            .publish(status, DependencyPhase::Idle, components, issue, actions)
            .await
            .with_ready(ready))
    }

    async fn ensure_locked(&self, options: EnsureOptions) -> Result<Option<PreparedRuntime>> {
        self.publish(
            DependencyStatus::Preparing,
            DependencyPhase::Discovering,
            Vec::new(),
            None,
            Vec::new(),
        )
        .await;

        let r = match self.discover_r().await? {
            RDiscovery::Ready(runtime) => runtime,
            RDiscovery::Missing | RDiscovery::Incompatible(_) | RDiscovery::Invalid(_) => {
                if options.install_r {
                    self.prepare_r_installer(options.offline).await?;
                } else {
                    self.inspect_locked().await?;
                }
                return Ok(None);
            }
        };

        let target = current_target();
        let mut components = vec![r_component(&r, &self.manifest.r.requirement)];
        self.publish(
            DependencyStatus::Preparing,
            DependencyPhase::Discovering,
            components.clone(),
            None,
            Vec::new(),
        )
        .await;

        let ark = if let Some(ark) = self.resolve_existing_ark(&target, options.repair)? {
            ark
        } else {
            if options.offline {
                components.push(component(
                    "ark",
                    DependencyComponentStatus::Missing,
                    Some(format!("={}", self.manifest.ark.version)),
                    None,
                    None,
                    None,
                    false,
                    Some("No verified cached artifact is available while offline".into()),
                ));
                let report = self
                    .publish(
                        DependencyStatus::ActionRequired,
                        DependencyPhase::Idle,
                        components,
                        Some(DependencyIssue {
                            code: "network.offline".into(),
                            title: "Workspace runtime is not cached".into(),
                            message: "Connect to the network and retry, or provide a verified offline Runtime Pack.".into(),
                            retryable: true,
                            requires_user_action: true,
                            action_url: Some("rho://setup/dependencies".into()),
                        }),
                        vec![DependencyAction {
                            id: "ensure".into(),
                            label: "Retry online".into(),
                            requires_human: true,
                        }],
                    )
                    .await;
                debug_assert!(!report.0.ready);
                return Ok(None);
            }
            self.install_ark(&target, &mut components).await?
        };

        if !components.iter().any(|value| value.name == "ark") {
            components.push(component(
                "ark",
                DependencyComponentStatus::Ready,
                Some(format!("={}", self.manifest.ark.version)),
                Some(self.manifest.ark.version.clone()),
                Some(ark.source),
                Some(normalized_path(&ark.path)),
                ark.verified,
                None,
            ));
        }

        self.publish(
            DependencyStatus::Preparing,
            DependencyPhase::Installing,
            components.clone(),
            None,
            Vec::new(),
        )
        .await;
        let bridge_package = self.materialize_bridge()?;
        components.push(bridge_component(Some(&bridge_package)));

        self.publish(
            DependencyStatus::Preparing,
            DependencyPhase::GeneratingKernelspec,
            components.clone(),
            None,
            Vec::new(),
        )
        .await;
        let kernelspec_path = self.generate_kernelspec(&r, &ark.path)?;
        let binding = self.resolve_existing_binding(&r, &ark.path);
        ensure!(
            binding.is_ready(),
            "generated Workspace R binding failed validation: {}",
            binding.detail().unwrap_or("unknown validation failure")
        );
        components.push(binding_component(&r, &self.manifest.ark.version, &binding));

        let report = self
            .publish(
                DependencyStatus::Ready,
                DependencyPhase::Idle,
                components,
                None,
                Vec::new(),
            )
            .await
            .with_ready(true);
        *self.report.write().await = report;
        Ok(Some(PreparedRuntime {
            kernelspec_path,
            bridge_package,
            ark_path: ark.path,
            r,
        }))
    }

    async fn discover_r(&self) -> Result<RDiscovery> {
        let explicit = self.rscript_override.clone();
        let candidates = if let Some(path) = explicit.as_ref() {
            if !path.is_file() {
                return Ok(RDiscovery::Invalid(format!(
                    "RHO_RSCRIPT does not point to a file: {}",
                    path.display()
                )));
            }
            vec![path.clone()]
        } else {
            rscript_candidates()
        };
        if candidates.is_empty() {
            return Ok(RDiscovery::Missing);
        }
        let requirement = VersionReq::parse(&self.manifest.r.requirement)
            .context("parsing R version requirement")?;
        let mut last_error = None;
        let mut runtimes = Vec::new();
        for path in candidates {
            match probe_rscript(&path, explicit.is_some()).await {
                Ok(runtime) => runtimes.push(runtime),
                Err(error) => last_error = Some(error),
            }
        }
        if explicit.is_some() {
            return Ok(match runtimes.pop() {
                Some(runtime) if requirement.matches(&runtime.version) => {
                    RDiscovery::Ready(runtime)
                }
                Some(runtime) => RDiscovery::Incompatible(runtime),
                None => RDiscovery::Invalid(
                    last_error
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| "The configured Rscript could not be probed".into()),
                ),
            });
        }
        match select_preferred_r_runtime(runtimes, &requirement) {
            Some(SelectedRRuntime::Compatible(runtime)) => Ok(RDiscovery::Ready(runtime)),
            Some(SelectedRRuntime::Incompatible(runtime)) => Ok(RDiscovery::Incompatible(runtime)),
            None => Ok(RDiscovery::Missing),
        }
    }

    fn resolve_bundled_ark(&self, target: &str, path: &Path) -> Result<ResolvedArk> {
        let artifact = self
            .manifest
            .ark
            .artifacts
            .get(target)
            .with_context(|| format!("bundled Ark is not supported on platform {target}"))?;
        ensure!(
            path.is_file(),
            "bundled Ark executable does not exist: {}",
            path.display()
        );
        ensure!(
            path.file_name().and_then(|name| name.to_str()) == Some(artifact.executable.as_str()),
            "bundled Ark executable name does not match platform {target}: expected {}",
            artifact.executable
        );
        let receipt_path = path
            .parent()
            .context("bundled Ark executable has no parent directory")?
            .join("rho-install.json");
        ensure!(
            receipt_path.is_file(),
            "bundled Ark is missing its trusted receipt: {}",
            receipt_path.display()
        );
        let receipt: InstallReceipt = serde_json::from_slice(
            &std::fs::read(&receipt_path)
                .with_context(|| format!("reading {}", receipt_path.display()))?,
        )
        .with_context(|| format!("decoding {}", receipt_path.display()))?;
        ensure!(
            receipt.schema_version == 1
                && receipt.component == "ark"
                && receipt.version == self.manifest.ark.version
                && receipt.target == target
                && receipt.source_url == artifact.url
                && receipt
                    .archive_sha256
                    .eq_ignore_ascii_case(&artifact.sha256),
            "bundled Ark receipt does not match the pinned {target} artifact"
        );
        let executable_sha256 = sha256_file(path)?;
        ensure!(
            receipt
                .executable_sha256
                .eq_ignore_ascii_case(&executable_sha256),
            "bundled Ark executable failed SHA-256 verification"
        );
        Ok(ResolvedArk {
            path: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
            source: DependencySource::Bundled,
            verified: true,
        })
    }

    fn resolve_existing_ark(&self, target: &str, repair: bool) -> Result<Option<ResolvedArk>> {
        if let Some(path) = self.ark_override.as_ref() {
            ensure!(
                path.is_file(),
                "RHO_ARK does not point to a file: {}",
                path.display()
            );
            return Ok(Some(ResolvedArk {
                path: path.canonicalize().unwrap_or_else(|_| path.clone()),
                source: DependencySource::Explicit,
                verified: false,
            }));
        }
        if let Some(path) = self.bundled_ark.as_ref() {
            return self.resolve_bundled_ark(target, path).map(Some);
        }
        let Some(artifact) = self.manifest.ark.artifacts.get(target) else {
            return Ok(None);
        };
        let install_dir = self.ark_install_dir(target);
        let executable = install_dir.join(&artifact.executable);
        let receipt_path = install_dir.join("rho-install.json");
        if !executable.is_file() || !receipt_path.is_file() {
            return Ok(None);
        }
        let receipt: InstallReceipt = serde_json::from_slice(
            &std::fs::read(&receipt_path)
                .with_context(|| format!("reading {}", receipt_path.display()))?,
        )
        .with_context(|| format!("decoding {}", receipt_path.display()))?;
        let valid = receipt.schema_version == 1
            && receipt.component == "ark"
            && receipt.version == self.manifest.ark.version
            && receipt.target == target
            && receipt
                .archive_sha256
                .eq_ignore_ascii_case(&artifact.sha256)
            && receipt
                .executable_sha256
                .eq_ignore_ascii_case(&sha256_file(&executable)?);
        if valid {
            return Ok(Some(ResolvedArk {
                path: executable,
                source: DependencySource::Managed,
                verified: true,
            }));
        }
        if repair {
            std::fs::remove_dir_all(&install_dir)
                .with_context(|| format!("removing invalid Ark cache {}", install_dir.display()))?;
            return Ok(None);
        }
        bail!(
            "cached Ark failed integrity verification at {}; run `rho deps repair`",
            install_dir.display()
        )
    }

    async fn install_ark(
        &self,
        target: &str,
        components: &mut Vec<DependencyComponent>,
    ) -> Result<ResolvedArk> {
        let artifact = self
            .manifest
            .ark
            .artifacts
            .get(target)
            .with_context(|| format!("no Ark artifact is configured for {target}"))?
            .clone();
        ensure_https(&artifact.url)?;
        std::fs::create_dir_all(self.ark_component_root())?;
        let _lock = acquire_lock(self.ark_component_root().join(".install.lock")).await?;
        cleanup_stale_ark_installs(
            &self.ark_component_root(),
            &self.manifest.ark.version,
            target,
        )?;
        if let Some(existing) = self.resolve_existing_ark(target, false)? {
            return Ok(existing);
        }

        components.push(component(
            "ark",
            DependencyComponentStatus::Downloading,
            Some(format!("={}", self.manifest.ark.version)),
            Some(self.manifest.ark.version.clone()),
            Some(DependencySource::Downloaded),
            None,
            false,
            Some(format!("Downloading verified Ark for {target}")),
        ));
        self.publish(
            DependencyStatus::Preparing,
            DependencyPhase::Downloading,
            components.clone(),
            None,
            Vec::new(),
        )
        .await;

        let bytes = self.download(&artifact.url, artifact.size).await?;
        if artifact.size > 0 {
            ensure!(
                bytes.len() as u64 == artifact.size,
                "Ark artifact size mismatch: expected {}, got {}",
                artifact.size,
                bytes.len()
            );
        }
        let actual_sha256 = sha256_bytes(&bytes);
        ensure!(
            actual_sha256.eq_ignore_ascii_case(&artifact.sha256),
            "Ark archive checksum mismatch: expected {}, got {}",
            artifact.sha256,
            actual_sha256
        );
        if let Some(component) = components.iter_mut().find(|value| value.name == "ark") {
            component.status = DependencyComponentStatus::Verifying;
            component.verified = true;
            component.detail = Some("The downloaded archive passed SHA-256 verification".into());
        }
        self.publish(
            DependencyStatus::Preparing,
            DependencyPhase::Verifying,
            components.clone(),
            None,
            Vec::new(),
        )
        .await;

        let install_dir = self.ark_install_dir(target);
        let staging = self.ark_component_root().join(format!(
            ".ark-{}-{target}.install-{}",
            self.manifest.ark.version,
            Uuid::new_v4().simple()
        ));
        let archive = bytes;
        let expected_executable = artifact.executable.clone();
        let staging_for_extract = staging.clone();
        tokio::task::spawn_blocking(move || {
            extract_ark_archive(&archive, &staging_for_extract, &expected_executable)
        })
        .await
        .context("joining Ark extraction task")??;
        let executable = staging.join(&artifact.executable);
        let executable_sha256 = sha256_file(&executable)?;
        let receipt = InstallReceipt {
            schema_version: 1,
            component: "ark".into(),
            version: self.manifest.ark.version.clone(),
            target: target.into(),
            source_url: artifact.url,
            archive_sha256: actual_sha256,
            executable_sha256,
            installed_at: Utc::now().to_rfc3339(),
        };
        write_json(&staging.join("rho-install.json"), &receipt)?;
        if let Some(parent) = install_dir.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating Ark version cache {}", parent.display()))?;
        }
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir).with_context(|| {
                format!("removing incomplete Ark install {}", install_dir.display())
            })?;
        }
        std::fs::rename(&staging, &install_dir).with_context(|| {
            format!(
                "publishing Ark install {} to {}",
                staging.display(),
                install_dir.display()
            )
        })?;
        let path = install_dir.join(&artifact.executable);
        if let Some(component) = components.iter_mut().find(|value| value.name == "ark") {
            component.status = DependencyComponentStatus::Ready;
            component.path = Some(normalized_path(&path));
            component.source = Some(DependencySource::Managed);
            component.verified = true;
            component.detail = None;
        }
        Ok(ResolvedArk {
            path,
            source: DependencySource::Managed,
            verified: true,
        })
    }

    async fn download(&self, url: &str, expected_size: u64) -> Result<Vec<u8>> {
        ensure!(
            expected_size <= MAX_ARTIFACT_BYTES,
            "dependency artifact exceeds the configured size limit"
        );
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("downloading {url}"))?
            .error_for_status()
            .with_context(|| format!("dependency server rejected {url}"))?;
        if let Some(length) = response.content_length() {
            ensure!(
                length <= MAX_ARTIFACT_BYTES,
                "dependency artifact advertises an unsafe size of {length} bytes"
            );
        }
        let bytes = response
            .bytes()
            .await
            .context("reading dependency artifact")?;
        ensure!(
            bytes.len() as u64 <= MAX_ARTIFACT_BYTES,
            "dependency artifact exceeds the configured size limit"
        );
        Ok(bytes.to_vec())
    }

    async fn prepare_r_installer(&self, offline: bool) -> Result<()> {
        let target = current_r_installer_target();
        let Some(installer) = self.manifest.r.installers.get(&target).cloned() else {
            self.publish(
                DependencyStatus::ActionRequired,
                DependencyPhase::Idle,
                vec![component(
                    "r",
                    DependencyComponentStatus::Unsupported,
                    Some(self.manifest.r.requirement.clone()),
                    None,
                    None,
                    None,
                    false,
                    Some("Use Rho's rig or operating-system provider on this platform".into()),
                )],
                Some(DependencyIssue {
                    code: "r.install_provider_required".into(),
                    title: "R needs an operating-system install provider".into(),
                    message: "Rho cannot silently elevate privileges. Open Dependency Setup to approve the platform-specific R installation.".into(),
                    retryable: false,
                    requires_user_action: true,
                    action_url: Some("rho://setup/dependencies".into()),
                }),
                Vec::new(),
            )
            .await;
            return Ok(());
        };
        if let Some(minimum_os) = installer.minimum_os.as_deref() {
            let current_os = match current_os_version() {
                Ok(version) => version,
                Err(error) => {
                    self.publish(
                        DependencyStatus::Failed,
                        DependencyPhase::Idle,
                        vec![component(
                            "r",
                            DependencyComponentStatus::Unsupported,
                            Some(self.manifest.r.requirement.clone()),
                            None,
                            None,
                            None,
                            false,
                            Some(format!(
                                "Could not verify the operating-system requirement: {error:#}"
                            )),
                        )],
                        Some(DependencyIssue {
                            code: "r.os_version_unknown".into(),
                            title: "Rho could not verify this R installer".into(),
                            message: format!(
                                "The official R {} installer requires {target} {minimum_os} or newer, but Rho could not determine this system's version.",
                                self.manifest.r.recommended_version
                            ),
                            retryable: false,
                            requires_user_action: true,
                            action_url: Some("rho://setup/dependencies".into()),
                        }),
                        Vec::new(),
                    )
                    .await;
                    return Ok(());
                }
            };
            if !os_version_meets_minimum(&current_os, minimum_os).unwrap_or(false) {
                self.publish(
                    DependencyStatus::Failed,
                    DependencyPhase::Idle,
                    vec![component(
                        "r",
                        DependencyComponentStatus::Incompatible,
                        Some(format!("{target} >={minimum_os}")),
                        Some(current_os.clone()),
                        Some(DependencySource::System),
                        None,
                        true,
                        Some("The official R installer does not support this operating-system version".into()),
                    )],
                    Some(DependencyIssue {
                        code: "r.os_incompatible".into(),
                        title: "This R installer needs a newer operating system".into(),
                        message: format!(
                            "The official R {} installer requires {target} {minimum_os} or newer; this system reports {current_os}.",
                            self.manifest.r.recommended_version
                        ),
                        retryable: false,
                        requires_user_action: true,
                        action_url: Some("rho://setup/dependencies".into()),
                    }),
                    Vec::new(),
                )
                .await;
                return Ok(());
            }
        }
        if offline {
            self.publish(
                DependencyStatus::ActionRequired,
                DependencyPhase::Idle,
                Vec::new(),
                Some(DependencyIssue {
                    code: "network.offline".into(),
                    title: "R installer is not available offline".into(),
                    message: "Connect to the network or select an official offline R installer."
                        .into(),
                    retryable: true,
                    requires_user_action: true,
                    action_url: Some("rho://setup/dependencies".into()),
                }),
                Vec::new(),
            )
            .await;
            return Ok(());
        }
        ensure_https(&installer.url)?;
        let file_name = installer
            .url
            .rsplit('/')
            .next()
            .filter(|value| !value.is_empty())
            .context("R installer URL has no file name")?;
        let install_dir = self
            .cache_root
            .join("installers/r")
            .join(&self.manifest.r.recommended_version);
        std::fs::create_dir_all(&install_dir)?;
        let installer_path = install_dir.join(file_name);
        if !installer_path.is_file()
            || !sha256_file(&installer_path)?.eq_ignore_ascii_case(&installer.sha256)
        {
            self.publish(
                DependencyStatus::Preparing,
                DependencyPhase::Downloading,
                vec![component(
                    "r",
                    DependencyComponentStatus::Downloading,
                    Some(self.manifest.r.requirement.clone()),
                    Some(self.manifest.r.recommended_version.clone()),
                    Some(DependencySource::Downloaded),
                    None,
                    false,
                    Some("Downloading the official R installer from CRAN".into()),
                )],
                None,
                Vec::new(),
            )
            .await;
            let bytes = self.download(&installer.url, 0).await?;
            let actual = sha256_bytes(&bytes);
            ensure!(
                actual.eq_ignore_ascii_case(&installer.sha256),
                "R installer checksum mismatch: expected {}, got {}",
                installer.sha256,
                actual
            );
            atomic_write(&installer_path, &bytes)?;
        }
        self.publish(
            DependencyStatus::ActionRequired,
            DependencyPhase::Idle,
            vec![component(
                "r",
                DependencyComponentStatus::CandidateFound,
                Some(self.manifest.r.requirement.clone()),
                Some(self.manifest.r.recommended_version.clone()),
                Some(DependencySource::Downloaded),
                Some(normalized_path(&installer_path)),
                true,
                Some("The official installer is downloaded and verified but has not run".into()),
            )],
            Some(DependencyIssue {
                code: "r.install_approval_required".into(),
                title: "Approve the R installation".into(),
                message: format!(
                    "Rho verified the official R {} installer. Open it from Dependency Setup, complete the operating-system prompt, then retry.",
                    self.manifest.r.recommended_version
                ),
                retryable: true,
                requires_user_action: true,
                action_url: Some(format!(
                    "rho://setup/dependencies/install-r?path={}",
                    normalized_path(&installer_path)
                )),
            }),
            vec![DependencyAction {
                id: "open_r_installer".into(),
                label: "Open verified R installer".into(),
                requires_human: true,
            }, DependencyAction {
                id: "ensure".into(),
                label: "Retry after installing R".into(),
                requires_human: false,
            }],
        )
        .await;
        Ok(())
    }

    fn materialize_bridge(&self) -> Result<PathBuf> {
        let bridge_root = self
            .cache_root
            .join("components/bridge")
            .join(BRIDGE_VERSION)
            .join("rho.bridge");
        for (relative, contents) in BRIDGE_FILES {
            let path = bridge_root.join(relative);
            if path.is_file() && std::fs::read(&path)? == contents.as_bytes() {
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            atomic_write(&path, contents.as_bytes())?;
        }
        Ok(bridge_root)
    }

    fn generate_kernelspec(&self, r: &RRuntime, ark: &Path) -> Result<PathBuf> {
        let binding_dir = self.binding_dir(r);
        std::fs::create_dir_all(&binding_dir)?;
        let empty_renviron = binding_dir.join("empty.Renviron");
        atomic_write(&empty_renviron, b"")?;
        let log_path = binding_dir.join("ark.log");
        let kernelspec = binding_dir.join("kernel.json");
        let libraries = env::join_paths(&r.library_paths)
            .context("joining R library paths")?
            .to_string_lossy()
            .to_string();
        let mut search_paths = vec![r.r_bin.clone()];
        if let Some(path) = env::var_os("PATH") {
            search_paths.extend(env::split_paths(&path));
        }
        let search_path = env::join_paths(search_paths)
            .context("joining R executable search path")?
            .to_string_lossy()
            .to_string();
        let mut kernel_env = serde_json::Map::from_iter([
            ("R_HOME".into(), json!(normalized_path(&r.r_home))),
            ("R_LIBS".into(), json!(libraries)),
            (
                "R_ENVIRON_USER".into(),
                json!(normalized_path(&empty_renviron)),
            ),
            ("PATH".into(), json!(search_path)),
        ]);
        if cfg!(target_os = "linux") {
            let mut library_paths = vec![r.r_home.join("lib")];
            if let Some(existing) = env::var_os("LD_LIBRARY_PATH") {
                library_paths.extend(env::split_paths(&existing));
            }
            kernel_env.insert(
                "LD_LIBRARY_PATH".into(),
                json!(env::join_paths(library_paths)?.to_string_lossy()),
            );
        } else if cfg!(target_os = "macos") {
            let mut library_paths = vec![r.r_home.join("lib")];
            if let Some(existing) = env::var_os("DYLD_FALLBACK_LIBRARY_PATH") {
                library_paths.extend(env::split_paths(&existing));
            }
            kernel_env.insert(
                "DYLD_FALLBACK_LIBRARY_PATH".into(),
                json!(env::join_paths(library_paths)?.to_string_lossy()),
            );
        }
        let spec = json!({
            "argv": [
                normalized_path(ark),
                "--connection_file",
                "{connection_file}",
                "--session-mode",
                "console",
                "--log",
                normalized_path(&log_path),
                "--",
                "--interactive",
                "--no-environ",
                "--no-init-file",
                "--no-site-file"
            ],
            "display_name": format!("Ark {} · R {} · Rho", self.manifest.ark.version, r.version),
            "language": "R",
            "interrupt_mode": "message",
            "kernel_protocol_version": "5.4",
            "env": kernel_env
        });
        atomic_write(&kernelspec, &serde_json::to_vec_pretty(&spec)?)?;
        write_json(
            &binding_dir.join("runtime.json"),
            &json!({
                "schema_version": 1,
                "r_version": r.version.to_string(),
                "r_version_string": r.version_string,
                "r_home": normalized_path(&r.r_home),
                "rscript": normalized_path(&r.rscript_path),
                "ark_version": self.manifest.ark.version,
                "ark": normalized_path(ark),
                "generated_at": Utc::now().to_rfc3339()
            }),
        )?;
        Ok(kernelspec)
    }

    fn binding_id(&self, r: &RRuntime) -> String {
        format!(
            "r-{}-ark-{}-{}",
            r.version,
            self.manifest.ark.version,
            current_target()
        )
    }

    fn binding_dir(&self, r: &RRuntime) -> PathBuf {
        self.project_runtime_root
            .join("bindings")
            .join(self.binding_id(r))
    }

    fn resolve_existing_binding(&self, r: &RRuntime, ark: &Path) -> BindingResolution {
        let binding_dir = self.binding_dir(r);
        let kernelspec_path = binding_dir.join("kernel.json");
        let receipt_path = binding_dir.join("runtime.json");
        let empty_renviron = binding_dir.join("empty.Renviron");
        if !kernelspec_path.is_file() || !receipt_path.is_file() || !empty_renviron.is_file() {
            return BindingResolution::Missing {
                expected_path: kernelspec_path,
            };
        }
        match validate_binding(
            &kernelspec_path,
            &receipt_path,
            &empty_renviron,
            r,
            ark,
            &self.manifest.ark.version,
        ) {
            Ok(()) => BindingResolution::Ready { kernelspec_path },
            Err(error) => BindingResolution::Invalid {
                kernelspec_path,
                detail: format!("{error:#}"),
            },
        }
    }

    fn ark_component_root(&self) -> PathBuf {
        self.cache_root.join("components/ark")
    }

    fn ark_install_dir(&self, target: &str) -> PathBuf {
        self.ark_component_root()
            .join(&self.manifest.ark.version)
            .join(target)
    }

    async fn publish(
        &self,
        status: DependencyStatus,
        phase: DependencyPhase,
        components: Vec<DependencyComponent>,
        issue: Option<DependencyIssue>,
        available_actions: Vec<DependencyAction>,
    ) -> PublishedReport {
        let revision = self.report.read().await.revision.saturating_add(1);
        let report = DependencyReport {
            schema_version: DEPENDENCY_SCHEMA_VERSION.into(),
            revision,
            status,
            ready: status == DependencyStatus::Ready,
            phase,
            managed_by: "rho".into(),
            platform: current_target(),
            components,
            issue,
            available_actions,
            updated_at: Utc::now().to_rfc3339(),
        };
        *self.report.write().await = report.clone();
        PublishedReport(report)
    }

    async fn publish_failure(&self, code: &str, error: &anyhow::Error, retryable: bool) {
        let previous = self.current_report().await;
        self.publish(
            DependencyStatus::Failed,
            DependencyPhase::Idle,
            previous.components,
            Some(DependencyIssue {
                code: code.into(),
                title: "Runtime dependency preparation failed".into(),
                message: format!("{error:#}"),
                retryable,
                requires_user_action: true,
                action_url: Some("rho://setup/dependencies".into()),
            }),
            vec![DependencyAction {
                id: "repair".into(),
                label: "Repair runtime dependencies".into(),
                requires_human: true,
            }],
        )
        .await;
    }
}

struct PublishedReport(DependencyReport);

impl PublishedReport {
    fn with_ready(mut self, ready: bool) -> DependencyReport {
        self.0.ready = ready;
        self.0
    }
}

enum RDiscovery {
    Ready(RRuntime),
    Missing,
    Invalid(String),
    Incompatible(RRuntime),
}

enum SelectedRRuntime {
    Compatible(RRuntime),
    Incompatible(RRuntime),
}

fn select_preferred_r_runtime(
    mut runtimes: Vec<RRuntime>,
    requirement: &VersionReq,
) -> Option<SelectedRRuntime> {
    runtimes.sort_by(|left, right| {
        right.version.cmp(&left.version).then_with(|| {
            normalized_path(&left.rscript_path).cmp(&normalized_path(&right.rscript_path))
        })
    });
    if let Some(index) = runtimes
        .iter()
        .position(|runtime| requirement.matches(&runtime.version))
    {
        return Some(SelectedRRuntime::Compatible(runtimes.remove(index)));
    }
    runtimes
        .into_iter()
        .next()
        .map(SelectedRRuntime::Incompatible)
}

#[derive(Debug)]
struct ResolvedArk {
    path: PathBuf,
    source: DependencySource,
    verified: bool,
}

enum BindingResolution {
    Missing {
        expected_path: PathBuf,
    },
    Ready {
        kernelspec_path: PathBuf,
    },
    Invalid {
        kernelspec_path: PathBuf,
        detail: String,
    },
}

impl BindingResolution {
    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    fn path(&self) -> &Path {
        match self {
            Self::Missing { expected_path } => expected_path,
            Self::Ready { kernelspec_path }
            | Self::Invalid {
                kernelspec_path, ..
            } => kernelspec_path,
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            Self::Invalid { detail, .. } => Some(detail),
            _ => None,
        }
    }
}

fn initial_report(platform: &str) -> DependencyReport {
    DependencyReport {
        schema_version: DEPENDENCY_SCHEMA_VERSION.into(),
        revision: 0,
        status: DependencyStatus::Checking,
        ready: false,
        phase: DependencyPhase::Idle,
        managed_by: "rho".into(),
        platform: platform.into(),
        components: Vec::new(),
        issue: None,
        available_actions: Vec::new(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

#[allow(clippy::too_many_arguments)]
fn component(
    name: &str,
    status: DependencyComponentStatus,
    requirement: Option<String>,
    version: Option<String>,
    source: Option<DependencySource>,
    path: Option<String>,
    verified: bool,
    detail: Option<String>,
) -> DependencyComponent {
    DependencyComponent {
        name: name.into(),
        status,
        requirement,
        version,
        source,
        path,
        verified,
        detail,
    }
}

fn r_component(r: &RRuntime, requirement: &str) -> DependencyComponent {
    component(
        "r",
        DependencyComponentStatus::Ready,
        Some(requirement.into()),
        Some(r.version.to_string()),
        Some(r.source),
        Some(normalized_path(&r.rscript_path)),
        true,
        Some(format!("{} · {}", r.version_string, r.architecture)),
    )
}

fn binding_component(
    r: &RRuntime,
    ark_version: &str,
    binding: &BindingResolution,
) -> DependencyComponent {
    let version = format!("r-{}-ark-{ark_version}", r.version);
    let (status, verified, detail) = match binding {
        BindingResolution::Missing { .. } => (
            DependencyComponentStatus::Missing,
            false,
            Some("The project-scoped controlled kernelspec has not been generated".into()),
        ),
        BindingResolution::Ready { .. } => (
            DependencyComponentStatus::Ready,
            true,
            Some("Project-scoped binding validated against the selected R and Ark runtimes".into()),
        ),
        BindingResolution::Invalid { detail, .. } => (
            DependencyComponentStatus::Invalid,
            false,
            Some(detail.clone()),
        ),
    };
    component(
        "binding",
        status,
        Some(format!("R {} + Ark {ark_version}", r.version)),
        Some(version),
        Some(DependencySource::Managed),
        Some(normalized_path(binding.path())),
        verified,
        detail,
    )
}

fn bridge_component(path: Option<&Path>) -> DependencyComponent {
    component(
        "rho.bridge",
        DependencyComponentStatus::Ready,
        Some(format!("={BRIDGE_VERSION}")),
        Some(BRIDGE_VERSION.into()),
        Some(DependencySource::Embedded),
        path.map(normalized_path),
        true,
        Some("Bundled with this Rho build; no source checkout is required".into()),
    )
}

fn r_issue(invalid: bool, recommended_version: &str) -> DependencyIssue {
    DependencyIssue {
        code: if invalid { "r.invalid" } else { "r.missing" }.into(),
        title: if invalid {
            "The configured R runtime is invalid"
        } else {
            "R is required for Workspace R"
        }
        .into(),
        message: format!(
            "Rho did not find a compatible R. It can prepare the official R {recommended_version} installer after explicit approval; Agent setup will not install or replace R."
        ),
        retryable: true,
        requires_user_action: true,
        action_url: Some("rho://setup/dependencies".into()),
    }
}

fn validate_binding(
    kernelspec_path: &Path,
    receipt_path: &Path,
    empty_renviron: &Path,
    r: &RRuntime,
    ark: &Path,
    ark_version: &str,
) -> Result<()> {
    let receipt: BindingReceipt = serde_json::from_slice(
        &std::fs::read(receipt_path)
            .with_context(|| format!("reading binding receipt {}", receipt_path.display()))?,
    )
    .with_context(|| format!("decoding binding receipt {}", receipt_path.display()))?;
    ensure!(
        receipt.schema_version == 1,
        "unsupported binding receipt schema {}",
        receipt.schema_version
    );
    ensure!(
        receipt.r_version == r.version.to_string(),
        "binding R version {} does not match selected R {}",
        receipt.r_version,
        r.version
    );
    ensure!(
        receipt.r_home == normalized_path(&r.r_home),
        "binding R_HOME does not match the selected R runtime"
    );
    ensure!(
        receipt.rscript == normalized_path(&r.rscript_path),
        "binding Rscript does not match the selected R runtime"
    );
    ensure!(
        receipt.ark_version == ark_version,
        "binding Ark version {} does not match selected Ark {}",
        receipt.ark_version,
        ark_version
    );
    ensure!(
        receipt.ark == normalized_path(ark),
        "binding Ark path does not match the selected Ark runtime"
    );

    let kernelspec: BindingKernelSpec =
        serde_json::from_slice(&std::fs::read(kernelspec_path).with_context(|| {
            format!("reading binding kernelspec {}", kernelspec_path.display())
        })?)
        .with_context(|| format!("decoding binding kernelspec {}", kernelspec_path.display()))?;
    ensure!(
        kernelspec.argv.first().map(String::as_str) == Some(normalized_path(ark).as_str()),
        "binding kernelspec does not launch the selected Ark runtime"
    );
    ensure!(
        kernelspec.env.get("R_HOME").map(String::as_str)
            == Some(normalized_path(&r.r_home).as_str()),
        "binding kernelspec R_HOME does not match the selected R runtime"
    );
    ensure!(
        kernelspec.env.get("R_ENVIRON_USER").map(String::as_str)
            == Some(normalized_path(empty_renviron).as_str()),
        "binding kernelspec does not use its controlled R environment"
    );
    for required in ["--no-environ", "--no-init-file", "--no-site-file"] {
        ensure!(
            kernelspec.argv.iter().any(|argument| argument == required),
            "binding kernelspec omitted required isolation flag {required}"
        );
    }
    Ok(())
}

fn dependency_cache_root(project_root: &Path) -> Result<PathBuf> {
    if let Some(path) = env::var_os("RHO_DEPENDENCY_CACHE") {
        return Ok(PathBuf::from(path));
    }
    if let Some(project_dirs) = ProjectDirs::from("org", "Rho", "Rho") {
        return Ok(project_dirs.cache_dir().join("runtime"));
    }
    Ok(project_root.join(".rho/runtime/cache"))
}

fn current_target() -> String {
    let os = match env::consts::OS {
        "macos" => "darwin",
        value => value,
    };
    let architecture = match env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        value => value,
    };
    // Official R for Windows is currently x64-only. Keep Ark in the same
    // architecture under emulation rather than mixing arm64 Ark with x64 R.
    if os == "windows" && architecture == "arm64" {
        "windows-x64".into()
    } else {
        format!("{os}-{architecture}")
    }
}

fn current_r_installer_target() -> String {
    current_target()
}

fn current_os_version() -> Result<String> {
    #[cfg(target_os = "macos")]
    let output = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .stdin(Stdio::null())
        .output()
        .context("running sw_vers to verify the R installer requirement")?;
    #[cfg(windows)]
    let output = std::process::Command::new("cmd.exe")
        .args(["/C", "ver"])
        .stdin(Stdio::null())
        .output()
        .context("running cmd.exe ver to verify the R installer requirement")?;
    #[cfg(not(any(target_os = "macos", windows)))]
    bail!("operating-system version detection is unavailable on this platform");
    #[cfg(any(target_os = "macos", windows))]
    {
        ensure!(
            output.status.success(),
            "operating-system version command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        let output = String::from_utf8(output.stdout)
            .context("operating-system version command returned non-UTF-8 output")?;
        let components = numeric_version_components(&output)
            .context("operating-system version command returned no numeric version")?;
        Ok(components
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("."))
    }
}

fn numeric_version_components(value: &str) -> Option<Vec<u64>> {
    value
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .filter(|token| !token.is_empty())
        .find_map(|token| {
            let components = token
                .split('.')
                .map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            (!components.is_empty()).then_some(components)
        })
}

fn os_version_meets_minimum(current: &str, minimum: &str) -> Option<bool> {
    let mut current = numeric_version_components(current)?;
    let mut minimum = numeric_version_components(minimum)?;
    let width = current.len().max(minimum.len());
    current.resize(width, 0);
    minimum.resize(width, 0);
    Some(current >= minimum)
}

fn rscript_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(r_home) = env::var_os("R_HOME") {
        candidates.push(
            PathBuf::from(r_home)
                .join("bin")
                .join(executable_name("Rscript")),
        );
    }
    if let Some(path) = find_command("Rscript") {
        candidates.push(path);
    }
    if cfg!(target_os = "macos") {
        candidates.extend([
            PathBuf::from("/Library/Frameworks/R.framework/Resources/bin/Rscript"),
            PathBuf::from("/opt/homebrew/bin/Rscript"),
            PathBuf::from("/usr/local/bin/Rscript"),
        ]);
    } else if cfg!(target_os = "linux") {
        candidates.extend([
            PathBuf::from("/usr/bin/Rscript"),
            PathBuf::from("/usr/local/bin/Rscript"),
        ]);
    } else if cfg!(windows)
        && let Some(program_files) = env::var_os("ProgramFiles")
    {
        let root = PathBuf::from(program_files).join("R");
        if let Ok(entries) = std::fs::read_dir(root) {
            let mut installed = entries
                .flatten()
                .map(|entry| entry.path().join("bin/Rscript.exe"))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            installed.sort();
            installed.reverse();
            candidates.extend(installed);
        }
    }
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|path| path.is_file())
        .filter(|path| seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())))
        .collect()
}

async fn probe_rscript(path: &Path, explicit: bool) -> Result<RRuntime> {
    let expression = r#"cat('RHO_VERSION=', paste(R.version$major, R.version$minor, sep='.'), '\n', sep=''); cat('RHO_VERSION_STRING=', R.version.string, '\n', sep=''); cat('RHO_HOME=', normalizePath(R.home(), winslash='/', mustWork=TRUE), '\n', sep=''); cat('RHO_BIN=', normalizePath(R.home('bin'), winslash='/', mustWork=TRUE), '\n', sep=''); cat('RHO_LIBS=', paste(normalizePath(.libPaths(), winslash='/', mustWork=TRUE), collapse=.Platform$path.sep), '\n', sep=''); cat('RHO_ARCH=', R.version$arch, '\n', sep='')"#;
    let output = tokio::time::timeout(
        R_PROBE_TIMEOUT,
        tokio::process::Command::new(path)
            .args(["--vanilla", "-e", expression])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .with_context(|| format!("timed out probing {}", path.display()))?
    .with_context(|| format!("running {}", path.display()))?;
    ensure!(
        output.status.success(),
        "R runtime probe failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let stdout = String::from_utf8(output.stdout).context("R probe returned non-UTF-8 output")?;
    let fields = stdout
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect::<BTreeMap<_, _>>();
    let version_text = fields
        .get("RHO_VERSION")
        .context("R probe omitted version")?;
    let version = Version::parse(version_text)
        .with_context(|| format!("parsing discovered R version {version_text}"))?;
    let r_home = PathBuf::from(fields.get("RHO_HOME").context("R probe omitted R_HOME")?);
    let r_bin = PathBuf::from(fields.get("RHO_BIN").context("R probe omitted R bin")?);
    let library_paths = fields
        .get("RHO_LIBS")
        .map(|value| env::split_paths(value).collect::<Vec<_>>())
        .unwrap_or_default();
    ensure!(
        r_home.is_dir(),
        "R_HOME does not exist: {}",
        r_home.display()
    );
    ensure!(r_bin.is_dir(), "R bin does not exist: {}", r_bin.display());
    Ok(RRuntime {
        rscript_path: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
        r_home,
        r_bin,
        library_paths,
        version,
        version_string: fields
            .get("RHO_VERSION_STRING")
            .cloned()
            .unwrap_or_else(|| format!("R {version_text}")),
        architecture: fields.get("RHO_ARCH").cloned().unwrap_or_default(),
        source: if explicit {
            DependencySource::Explicit
        } else {
            DependencySource::System
        },
    })
}

fn find_command(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .map(str::to_ascii_lowercase)
            .collect()
    } else {
        vec![String::new()]
    };
    env::split_paths(&path).find_map(|directory| {
        extensions.iter().find_map(|extension| {
            let candidate = if extension.is_empty() {
                directory.join(name)
            } else {
                directory.join(format!("{name}{extension}"))
            };
            candidate.is_file().then_some(candidate)
        })
    })
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    }
}

async fn acquire_lock(path: PathBuf) -> Result<File> {
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("opening dependency lock {}", path.display()))?;
        FileExt::lock(&file)
            .with_context(|| format!("locking dependency cache {}", path.display()))?;
        Ok(file)
    })
    .await
    .context("joining dependency lock task")?
}

fn extract_ark_archive(bytes: &[u8], destination: &Path, executable: &str) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("opening Ark zip")?;
    let allowed = HashSet::from([executable, "LICENSE", "NOTICE"]);
    let mut extracted = HashSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).context("reading Ark zip entry")?;
        let enclosed = entry
            .enclosed_name()
            .context("Ark archive contains an unsafe path")?;
        ensure!(
            enclosed.components().count() == 1,
            "Ark archive entry must be at its root: {}",
            enclosed.display()
        );
        let name = enclosed
            .file_name()
            .and_then(|value| value.to_str())
            .context("Ark archive contains a non-UTF-8 name")?;
        if entry.is_dir() {
            continue;
        }
        if let Some(mode) = entry.unix_mode() {
            ensure!(
                mode & 0o170000 != 0o120000,
                "Ark archive contains a symlink"
            );
        }
        if !allowed.contains(name) {
            continue;
        }
        let output_path = destination.join(name);
        let mut output = File::create(&output_path)
            .with_context(|| format!("creating {}", output_path.display()))?;
        std::io::copy(&mut entry, &mut output).with_context(|| format!("extracting {name}"))?;
        output.sync_all()?;
        extracted.insert(name.to_string());
    }
    for required in [executable, "LICENSE", "NOTICE"] {
        ensure!(
            extracted.contains(required),
            "Ark archive omitted required file {required}"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = destination.join(executable);
        let mut permissions = path.metadata()?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn cleanup_stale_ark_installs(root: &Path, version: &str, target: &str) -> Result<()> {
    let prefix = format!(".ark-{version}-{target}.install-");
    for entry in std::fs::read_dir(root)
        .with_context(|| format!("reading Ark component cache {}", root.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.file_name().to_string_lossy().starts_with(&prefix) {
            std::fs::remove_dir_all(entry.path()).with_context(|| {
                format!(
                    "removing stale Ark staging directory {}",
                    entry.path().display()
                )
            })?;
        }
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .with_context(|| format!("hashing {}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = File::create(&temporary)
            .with_context(|| format!("creating {}", temporary.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("writing {}", temporary.display()))?;
        file.sync_all()?;
        if path.exists() {
            std::fs::remove_file(path).with_context(|| format!("replacing {}", path.display()))?;
        }
        std::fs::rename(&temporary, path)
            .with_context(|| format!("publishing {}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    atomic_write(path, &serde_json::to_vec_pretty(value)?)
}

fn ensure_https(url: &str) -> Result<()> {
    let url = reqwest::Url::parse(url).context("parsing dependency artifact URL")?;
    ensure!(
        url.scheme() == "https",
        "dependency artifacts must use HTTPS"
    );
    ensure!(
        url.host_str().is_some(),
        "dependency artifact URL has no host"
    );
    Ok(())
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;

    fn fixture_runtime(root: &Path) -> RRuntime {
        let r_home = root.join("r-home");
        let r_bin = r_home.join("bin");
        let r_library = r_home.join("library");
        std::fs::create_dir_all(&r_bin).unwrap();
        std::fs::create_dir_all(&r_library).unwrap();
        let rscript_path = r_bin.join(executable_name("Rscript"));
        std::fs::write(&rscript_path, b"fixture").unwrap();
        RRuntime {
            rscript_path,
            r_home,
            r_bin,
            library_paths: vec![r_library],
            version: Version::new(4, 5, 2),
            version_string: "R version 4.5.2 (fixture)".into(),
            architecture: env::consts::ARCH.into(),
            source: DependencySource::System,
        }
    }

    fn fixture_bundled_ark(
        manager: &DependencyManager,
        root: &Path,
        target: &str,
        receipt_target: &str,
    ) -> PathBuf {
        let artifact = manager.manifest.ark.artifacts.get(target).unwrap();
        let ark = root.join(&artifact.executable);
        std::fs::write(&ark, b"bundled ark fixture").unwrap();
        write_json(
            &root.join("rho-install.json"),
            &InstallReceipt {
                schema_version: 1,
                component: "ark".into(),
                version: manager.manifest.ark.version.clone(),
                target: receipt_target.into(),
                source_url: artifact.url.clone(),
                archive_sha256: artifact.sha256.clone(),
                executable_sha256: sha256_file(&ark).unwrap(),
                installed_at: Utc::now().to_rfc3339(),
            },
        )
        .unwrap();
        ark
    }

    #[test]
    fn manifest_covers_supported_ark_targets() {
        let manifest: DependencyManifest = serde_json::from_str(MANIFEST_JSON).unwrap();
        assert_eq!(manifest.schema_version, 1);
        for target in [
            "darwin-arm64",
            "darwin-x64",
            "linux-arm64",
            "linux-x64",
            "windows-arm64",
            "windows-x64",
        ] {
            let artifact = manifest.ark.artifacts.get(target).unwrap();
            assert!(artifact.url.starts_with("https://"));
            assert_eq!(artifact.sha256.len(), 64);
            assert!(artifact.size > 1_000_000);
        }
        VersionReq::parse(&manifest.r.requirement).unwrap();
        Version::parse(&manifest.r.recommended_version).unwrap();
        assert!(manifest.r.installers.values().all(|installer| {
            installer
                .minimum_os
                .as_deref()
                .and_then(numeric_version_components)
                .is_some()
        }));
    }

    #[test]
    fn implicit_r_selection_prefers_the_highest_compatible_version() {
        let directory = TempDir::new().unwrap();
        let mut old = fixture_runtime(directory.path());
        old.version = Version::new(4, 3, 3);
        old.rscript_path = PathBuf::from("/r/old/Rscript");
        let mut compatible = fixture_runtime(directory.path());
        compatible.version = Version::new(4, 4, 2);
        compatible.rscript_path = PathBuf::from("/r/compatible/Rscript");
        let mut highest = fixture_runtime(directory.path());
        highest.version = Version::new(4, 6, 1);
        highest.rscript_path = PathBuf::from("/r/highest/Rscript");
        let requirement = VersionReq::parse(">=4.4.0,<5.0.0").unwrap();

        let selected =
            select_preferred_r_runtime(vec![old, compatible, highest], &requirement).unwrap();
        let SelectedRRuntime::Compatible(selected) = selected else {
            panic!("expected a compatible R runtime");
        };
        assert_eq!(selected.version, Version::new(4, 6, 1));
        assert_eq!(selected.rscript_path, PathBuf::from("/r/highest/Rscript"));
    }

    #[test]
    fn operating_system_version_comparison_is_numeric() {
        assert_eq!(os_version_meets_minimum("14", "14"), Some(true));
        assert_eq!(os_version_meets_minimum("14.0", "14"), Some(true));
        assert_eq!(os_version_meets_minimum("15.2.1", "14"), Some(true));
        assert_eq!(os_version_meets_minimum("10.0.26100", "11"), Some(false));
        assert_eq!(
            numeric_version_components("Microsoft Windows [Version 10.0.26100.1]"),
            Some(vec![10, 0, 26100, 1])
        );
    }

    #[test]
    fn bundled_ark_requires_matching_platform_receipt_and_executable_hash() {
        let project = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let bundle = TempDir::new().unwrap();
        let target = "darwin-arm64";
        let manager = DependencyManager::new(project.path())
            .unwrap()
            .with_cache_root(cache.path());
        let ark = fixture_bundled_ark(&manager, bundle.path(), target, target);
        let manager = manager.with_bundled_ark(&ark);

        let resolved = manager
            .resolve_existing_ark(target, false)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.source, DependencySource::Bundled);
        assert!(resolved.verified);

        std::fs::write(&ark, b"tampered bundled ark").unwrap();
        let error = manager.resolve_existing_ark(target, false).unwrap_err();
        assert!(error.to_string().contains("SHA-256 verification"));

        fixture_bundled_ark(&manager, bundle.path(), target, "darwin-x64");
        let error = manager.resolve_existing_ark(target, false).unwrap_err();
        assert!(error.to_string().contains("does not match the pinned"));
    }

    #[test]
    fn ark_extraction_is_allowlisted_and_preserves_notices() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut bytes);
            let options = SimpleFileOptions::default();
            writer.start_file("ark", options).unwrap();
            writer.write_all(b"binary").unwrap();
            writer.start_file("LICENSE", options).unwrap();
            writer.write_all(b"license").unwrap();
            writer.start_file("NOTICE", options).unwrap();
            writer.write_all(b"notice").unwrap();
            writer.start_file("ignored.txt", options).unwrap();
            writer.write_all(b"ignored").unwrap();
            writer.finish().unwrap();
        }
        let directory = TempDir::new().unwrap();
        extract_ark_archive(bytes.get_ref(), directory.path(), "ark").unwrap();
        assert!(directory.path().join("ark").is_file());
        assert!(directory.path().join("LICENSE").is_file());
        assert!(directory.path().join("NOTICE").is_file());
        assert!(!directory.path().join("ignored.txt").exists());
    }

    #[test]
    fn ark_extraction_rejects_path_traversal() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut bytes);
            writer
                .start_file("../ark", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"binary").unwrap();
            writer.finish().unwrap();
        }
        let directory = TempDir::new().unwrap();
        let error = extract_ark_archive(bytes.get_ref(), directory.path(), "ark").unwrap_err();
        assert!(error.to_string().contains("unsafe path"));
    }

    #[tokio::test]
    async fn invalid_explicit_r_is_reported_without_falling_back_to_path() {
        let project = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let manager = DependencyManager::new(project.path())
            .unwrap()
            .with_cache_root(cache.path())
            .with_rscript(project.path().join("missing-Rscript"));
        let report = manager.inspect().await.unwrap();
        assert_eq!(report.status, DependencyStatus::ActionRequired);
        assert_eq!(report.issue.unwrap().code, "r.invalid");
        assert_eq!(
            report.components[0].status,
            DependencyComponentStatus::Invalid
        );
    }

    #[test]
    fn atomic_write_replaces_one_exact_file() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("value.json");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"two");
    }

    #[test]
    fn generated_binding_validates_and_stale_runtime_metadata_is_rejected() {
        let project = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let manager = DependencyManager::new(project.path())
            .unwrap()
            .with_cache_root(cache.path());
        let r = fixture_runtime(project.path());
        let ark = project.path().join(executable_name("ark"));
        std::fs::write(&ark, b"fixture").unwrap();

        let kernelspec = manager.generate_kernelspec(&r, &ark).unwrap();
        let binding = manager.resolve_existing_binding(&r, &ark);
        assert!(binding.is_ready());
        assert_eq!(binding.path(), kernelspec);

        let runtime_path = kernelspec.parent().unwrap().join("runtime.json");
        let mut runtime: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&runtime_path).unwrap()).unwrap();
        runtime["ark"] = json!(normalized_path(&project.path().join("stale-ark")));
        write_json(&runtime_path, &runtime).unwrap();

        let binding = manager.resolve_existing_binding(&r, &ark);
        assert!(matches!(binding, BindingResolution::Invalid { .. }));
        assert!(
            binding
                .detail()
                .unwrap()
                .contains("Ark path does not match")
        );
    }

    #[test]
    fn stale_kernelspec_runtime_path_is_rejected() {
        let project = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let manager = DependencyManager::new(project.path())
            .unwrap()
            .with_cache_root(cache.path());
        let r = fixture_runtime(project.path());
        let ark = project.path().join(executable_name("ark"));
        std::fs::write(&ark, b"fixture").unwrap();

        let kernelspec = manager.generate_kernelspec(&r, &ark).unwrap();
        let mut spec: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&kernelspec).unwrap()).unwrap();
        spec["argv"][0] = json!(normalized_path(&project.path().join("stale-ark")));
        write_json(&kernelspec, &spec).unwrap();

        let binding = manager.resolve_existing_binding(&r, &ark);
        assert!(matches!(binding, BindingResolution::Invalid { .. }));
        assert!(
            binding
                .detail()
                .unwrap()
                .contains("does not launch the selected Ark")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn inspect_requires_missing_binding_and_ensure_reports_it_ready() {
        use std::os::unix::fs::PermissionsExt;

        let project = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let r = fixture_runtime(project.path());
        let rscript = project.path().join("fixture-Rscript");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' 'RHO_VERSION=4.5.2' 'RHO_VERSION_STRING=R version 4.5.2 (fixture)' 'RHO_HOME={}' 'RHO_BIN={}' 'RHO_LIBS={}' 'RHO_ARCH={}'\n",
            normalized_path(&r.r_home),
            normalized_path(&r.r_bin),
            normalized_path(&r.library_paths[0]),
            env::consts::ARCH,
        );
        std::fs::write(&rscript, script).unwrap();
        let mut permissions = std::fs::metadata(&rscript).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&rscript, permissions).unwrap();
        let ark = project.path().join("fixture-ark");
        std::fs::write(&ark, b"fixture").unwrap();

        let manager = DependencyManager::new(project.path())
            .unwrap()
            .with_cache_root(cache.path())
            .with_rscript(&rscript)
            .with_ark(&ark);
        let report = manager.inspect().await.unwrap();
        assert_eq!(report.status, DependencyStatus::ActionRequired);
        assert_eq!(report.issue.unwrap().code, "binding.missing");
        let binding = report
            .components
            .iter()
            .find(|component| component.name == "binding")
            .unwrap();
        assert_eq!(binding.status, DependencyComponentStatus::Missing);
        assert_eq!(binding.source, Some(DependencySource::Managed));

        let prepared = manager
            .ensure(EnsureOptions::default())
            .await
            .unwrap()
            .unwrap();
        assert!(prepared.kernelspec_path.is_file());
        let report = manager.inspect().await.unwrap();
        assert_eq!(report.status, DependencyStatus::Ready);
        assert!(report.ready);
        let binding = report
            .components
            .iter()
            .find(|component| component.name == "binding")
            .unwrap();
        assert_eq!(binding.status, DependencyComponentStatus::Ready);
        assert_eq!(binding.source, Some(DependencySource::Managed));
        assert!(binding.verified);
        assert!(binding.version.as_deref().unwrap().contains("ark-"));
        assert_eq!(
            binding.path.as_deref(),
            Some(normalized_path(&prepared.kernelspec_path).as_str())
        );
    }
}
