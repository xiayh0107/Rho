#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use rho_runtime_deps::{DependencyManager, EnsureOptions};
use rho_server::runtime::{RuntimeService, RuntimeServiceConfig};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Default, Eq, PartialEq)]
struct LaunchOptions {
    project_root: Option<PathBuf>,
    kernelspec: Option<PathBuf>,
    bridge_package: Option<PathBuf>,
}

fn main() {
    let launch = launch_options(env::args_os().skip(1)).unwrap_or_else(|error| {
        eprintln!("rho-desktop: {error:#}");
        std::process::exit(2);
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| Ok(setup(app, &launch)?))
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running the Rho desktop wrapper");
}

fn setup(app: &mut tauri::App, launch: &LaunchOptions) -> Result<()> {
    let project_root = launch
        .project_root
        .clone()
        .unwrap_or(env::current_dir().context("resolving current project directory")?)
        .canonicalize()
        .context("resolving Rho project root")?;
    ensure!(
        project_root.is_dir(),
        "Rho project root must be a directory"
    );

    let store_path = project_root.join(".rho/state/runtime.sqlite");
    let runtime = RuntimeService::open(
        RuntimeServiceConfig::new(store_path).with_project_root(project_root.clone()),
    )?;
    let mut dependencies = DependencyManager::new(&project_root)?;
    if let Some(bundled_ark) = resolve_bundled_ark(app)? {
        dependencies = dependencies.with_bundled_ark(bundled_ark);
    }
    tauri::async_runtime::block_on(runtime.set_dependency_manager(dependencies.clone()));
    let listener = tauri::async_runtime::block_on(tokio::net::TcpListener::bind((
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        0,
    )))?;
    let address = listener.local_addr()?;
    let runtime_url = format!("http://{address}");

    let server_runtime = runtime.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = rho_server::api::serve(listener, server_runtime).await {
            eprintln!("rho-desktop server stopped: {error:#}");
        }
    });

    let explicit_start = launch.kernelspec.clone().map(|kernelspec| {
        let bridge_package = resolve_bridge_package(app, launch.bridge_package.as_deref())?;
        Ok::<_, anyhow::Error>((kernelspec, bridge_package))
    });
    let runtime_start = runtime.clone();
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let result = match explicit_start.transpose() {
            Ok(Some((kernelspec, bridge_package))) => runtime_start
                .start_ark(kernelspec, bridge_package)
                .await
                .map(Some),
            Ok(None) => {
                runtime_start
                    .ensure_managed_workspace(EnsureOptions::default())
                    .await
            }
            Err(error) => Err(error),
        };
        let message = match result {
            Ok(Some(_)) => "Workspace R is connected.".to_string(),
            Ok(None) => {
                let report = runtime_start.dependency_report().await;
                report.issue.map_or_else(
                    || "Workspace R is waiting for runtime dependencies.".to_string(),
                    |issue| issue.message,
                )
            }
            Err(error) => {
                eprintln!("rho-desktop could not prepare Workspace R: {error:#}");
                format!("Workspace R could not start: {error}")
            }
        };
        let _ = app_handle
            .notification()
            .builder()
            .title("Rho")
            .body(message)
            .show();
    });

    build_window(app, address, &runtime_url)?;
    build_tray(app)?;
    let _ = app
        .notification()
        .builder()
        .title("Rho runtime started")
        .body(format!("Scientific workspace: {}", project_root.display()))
        .show();
    Ok(())
}

fn build_window(app: &tauri::App, address: SocketAddr, runtime_url: &str) -> Result<()> {
    let url = runtime_url.parse().context("parsing local runtime URL")?;
    WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
        .title("Rho · Scientific Runtime")
        .inner_size(1280.0, 820.0)
        .min_inner_size(900.0, 620.0)
        .center()
        .on_navigation(move |url| navigation_is_local_runtime(url, address))
        .build()?;
    Ok(())
}

fn build_tray(app: &tauri::App) -> Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Rho", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Rho", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id("rho")
        .tooltip("Rho scientific runtime")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn resolve_bridge_package(app: &tauri::App, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return canonical_directory(path, "Rho bridge package");
    }
    if let Some(path) = env::var_os("RHO_BRIDGE_PACKAGE") {
        return canonical_directory(&PathBuf::from(path), "RHO_BRIDGE_PACKAGE");
    }
    let installed = app
        .path()
        .resource_dir()
        .context("resolving Rho resource directory")?
        .join("resources/rho.bridge");
    let development = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../r/rho.bridge");
    [installed, development]
        .into_iter()
        .find(|path| path.is_dir())
        .context("rho.bridge was not found; set RHO_BRIDGE_PACKAGE")
}

fn resolve_bundled_ark(app: &tauri::App) -> Result<Option<PathBuf>> {
    let resource_dir = app
        .path()
        .resource_dir()
        .context("resolving Rho resource directory")?;
    let candidate = bundled_ark_resource_path(&resource_dir);
    if !candidate.is_file() {
        return Ok(None);
    }
    Ok(Some(candidate.canonicalize().with_context(|| {
        format!("resolving bundled Ark at {}", candidate.display())
    })?))
}

fn bundled_ark_resource_path(resource_dir: &Path) -> PathBuf {
    resource_dir
        .join("resources/runtime")
        .join(if cfg!(windows) { "ark.exe" } else { "ark" })
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .with_context(|| format!("resolving {label} at {}", path.display()))?;
    ensure!(path.is_dir(), "{label} must be a directory");
    Ok(path)
}

fn navigation_is_local_runtime(url: &tauri::Url, address: SocketAddr) -> bool {
    url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port_or_known_default() == Some(address.port())
}

fn launch_options(arguments: impl IntoIterator<Item = OsString>) -> Result<LaunchOptions> {
    let mut options = LaunchOptions::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.to_string_lossy().as_ref() {
            "--project-root" => {
                options.project_root = Some(required_path(&mut arguments, "--project-root")?);
            }
            "--kernelspec" => {
                options.kernelspec = Some(required_path(&mut arguments, "--kernelspec")?);
            }
            "--bridge-package" => {
                options.bridge_package = Some(required_path(&mut arguments, "--bridge-package")?);
            }
            value if value.starts_with('-') => {}
            _ if options.project_root.is_none() => {
                let path = PathBuf::from(argument);
                if path.is_dir() {
                    options.project_root = Some(path);
                } else if path.is_file() {
                    options.project_root = path.parent().map(Path::to_path_buf);
                }
            }
            _ => {}
        }
    }
    if options.project_root.is_none() {
        options.project_root = env::var_os("RHO_PROJECT_ROOT").map(PathBuf::from);
    }
    if options.kernelspec.is_none() {
        options.kernelspec = env::var_os("RHO_KERNELSPEC").map(PathBuf::from);
    }
    Ok(options)
}

fn required_path(arguments: &mut impl Iterator<Item = OsString>, option: &str) -> Result<PathBuf> {
    arguments
        .next()
        .map(PathBuf::from)
        .with_context(|| format!("{option} requires a path"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn file_association_uses_the_containing_project() {
        let directory = TempDir::new().unwrap();
        let project = directory.path().join("analysis.Rproj");
        std::fs::write(&project, "Version: 1.0\n").unwrap();
        let options = launch_options([project.into_os_string()]).unwrap();
        assert_eq!(options.project_root.as_deref(), Some(directory.path()));
    }

    #[test]
    fn explicit_launch_options_are_parsed_without_interpreting_platform_flags() {
        let options = launch_options([
            OsString::from("-psn_0_123"),
            OsString::from("--project-root"),
            OsString::from("/tmp/project"),
            OsString::from("--kernelspec"),
            OsString::from("/tmp/kernel.json"),
        ])
        .unwrap();
        assert_eq!(options.project_root, Some(PathBuf::from("/tmp/project")));
        assert_eq!(options.kernelspec, Some(PathBuf::from("/tmp/kernel.json")));
    }

    #[test]
    fn navigation_is_limited_to_the_embedded_local_server() {
        let address: SocketAddr = "127.0.0.1:8787".parse().unwrap();
        assert!(navigation_is_local_runtime(
            &"http://127.0.0.1:8787/runs".parse().unwrap(),
            address
        ));
        assert!(!navigation_is_local_runtime(
            &"http://127.0.0.1:9999/".parse().unwrap(),
            address
        ));
        assert!(!navigation_is_local_runtime(
            &"https://example.com/".parse().unwrap(),
            address
        ));
    }

    #[test]
    fn packaged_ark_uses_the_tauri_runtime_resource_layout() {
        let resource_dir = Path::new("/application/resources");
        let expected_name = if cfg!(windows) { "ark.exe" } else { "ark" };
        assert_eq!(
            bundled_ark_resource_path(resource_dir),
            resource_dir.join("resources/runtime").join(expected_name)
        );
    }
}
