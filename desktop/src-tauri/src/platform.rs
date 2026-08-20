use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DesktopPlatform {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Eq, PartialEq)]
struct CommandSpec {
    program: OsString,
    arguments: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RscriptSelectionSpec {
    display_name: &'static str,
    picker_title: &'static str,
    picker_extension: Option<&'static str>,
}

fn current_platform() -> DesktopPlatform {
    if cfg!(target_os = "windows") {
        DesktopPlatform::Windows
    } else if cfg!(target_os = "macos") {
        DesktopPlatform::Macos
    } else {
        DesktopPlatform::Linux
    }
}

fn rscript_selection_spec(platform: DesktopPlatform) -> RscriptSelectionSpec {
    match platform {
        DesktopPlatform::Windows => RscriptSelectionSpec {
            display_name: "Rscript.exe",
            picker_title: "Choose Rscript.exe",
            picker_extension: Some("exe"),
        },
        DesktopPlatform::Macos | DesktopPlatform::Linux => RscriptSelectionSpec {
            display_name: "Rscript",
            picker_title: "Choose Rscript",
            picker_extension: None,
        },
    }
}

pub fn rscript_display_name() -> &'static str {
    rscript_selection_spec(current_platform()).display_name
}

pub fn rscript_picker_title() -> &'static str {
    rscript_selection_spec(current_platform()).picker_title
}

pub fn rscript_picker_extension() -> Option<&'static str> {
    rscript_selection_spec(current_platform()).picker_extension
}

/// Short architecture requirement used in the stable `R_ARCH_MISMATCH`
/// recovery detail (LIN4).
pub fn r_architecture_requirement() -> &'static str {
    r_architecture_requirement_for(current_platform())
}

fn r_architecture_requirement_for(platform: DesktopPlatform) -> &'static str {
    match platform {
        DesktopPlatform::Macos => "Rho for Apple Silicon requires arm64 R",
        DesktopPlatform::Linux => "Rho for Linux x64 requires x86_64 R",
        DesktopPlatform::Windows => "Rho requires a supported R architecture",
    }
}

/// Full user-facing architecture requirement sentence used by the startup
/// recovery copy (LIN4).
pub fn r_architecture_requirement_message() -> &'static str {
    r_architecture_requirement_message_for(current_platform())
}

fn r_architecture_requirement_message_for(platform: DesktopPlatform) -> &'static str {
    match platform {
        DesktopPlatform::Macos => {
            "Rho for Apple Silicon requires an arm64 R 4.4 or later installation."
        }
        DesktopPlatform::Linux => {
            "Rho for Linux x64 requires an x86_64 R 4.4 or later installation."
        }
        DesktopPlatform::Windows => {
            "Rho requires an R 4.4 or later installation with a supported architecture."
        }
    }
}

fn open_url_spec(platform: DesktopPlatform, url: &str) -> CommandSpec {
    match platform {
        DesktopPlatform::Windows => CommandSpec {
            program: "explorer.exe".into(),
            arguments: vec![url.into()],
        },
        DesktopPlatform::Macos => CommandSpec {
            program: "open".into(),
            arguments: vec![url.into()],
        },
        DesktopPlatform::Linux => CommandSpec {
            program: "xdg-open".into(),
            arguments: vec![url.into()],
        },
    }
}

fn reveal_path_spec(platform: DesktopPlatform, path: &Path) -> CommandSpec {
    match platform {
        DesktopPlatform::Windows => CommandSpec {
            program: "explorer.exe".into(),
            arguments: vec!["/select,".into(), path.as_os_str().to_owned()],
        },
        DesktopPlatform::Macos => CommandSpec {
            program: "open".into(),
            arguments: vec!["-R".into(), path.as_os_str().to_owned()],
        },
        DesktopPlatform::Linux => CommandSpec {
            program: "xdg-open".into(),
            arguments: vec![path.parent().unwrap_or(path).as_os_str().to_owned()],
        },
    }
}

fn command_from_spec(spec: CommandSpec) -> Command {
    let mut command = Command::new(spec.program);
    command.args(spec.arguments);
    command
}

pub fn open_url_command(url: &str) -> Command {
    command_from_spec(open_url_spec(current_platform(), url))
}

pub fn reveal_path_command(path: &Path) -> Command {
    command_from_spec(reveal_path_spec(current_platform(), path))
}

fn merge_process_path(
    inherited: impl IntoIterator<Item = PathBuf>,
    r_bin: Option<&Path>,
    defaults: impl IntoIterator<Item = PathBuf>,
    user_local_bin: Option<PathBuf>,
) -> Result<OsString, std::env::JoinPathsError> {
    let mut paths = Vec::<PathBuf>::new();
    for path in inherited
        .into_iter()
        .chain(r_bin.map(Path::to_path_buf))
        .chain(defaults)
        .chain(user_local_bin)
    {
        if path.as_os_str().is_empty() || paths.iter().any(|known| known == &path) {
            continue;
        }
        paths.push(path);
    }
    std::env::join_paths(paths)
}

pub fn child_process_path(r_bin: Option<&Path>) -> Result<OsString, std::env::JoinPathsError> {
    let inherited = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let defaults = match current_platform() {
        DesktopPlatform::Windows => Vec::new(),
        DesktopPlatform::Macos => [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect(),
        DesktopPlatform::Linux => ["/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
    };
    let user_local_bin = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/bin"))
        .filter(|path| path.is_dir());
    merge_process_path(inherited, r_bin, defaults, user_local_bin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn macos_open_specs_preserve_urls_and_unicode_paths() {
        assert_eq!(
            open_url_spec(DesktopPlatform::Macos, "https://yulab-smu.top/Rho/"),
            CommandSpec {
                program: "open".into(),
                arguments: vec!["https://yulab-smu.top/Rho/".into()],
            }
        );
        let path = PathBuf::from("/Users/研究者/Rho logs/startup.log");
        assert_eq!(
            reveal_path_spec(DesktopPlatform::Macos, &path),
            CommandSpec {
                program: "open".into(),
                arguments: vec!["-R".into(), path.into_os_string()],
            }
        );
    }

    #[test]
    fn windows_open_specs_retain_existing_behavior() {
        let path = PathBuf::from(r"C:\Users\Example User\Rho\startup.log");
        assert_eq!(
            open_url_spec(DesktopPlatform::Windows, "https://github.com/YuLab-SMU/Rho"),
            CommandSpec {
                program: "explorer.exe".into(),
                arguments: vec!["https://github.com/YuLab-SMU/Rho".into()],
            }
        );
        assert_eq!(
            reveal_path_spec(DesktopPlatform::Windows, &path),
            CommandSpec {
                program: "explorer.exe".into(),
                arguments: vec!["/select,".into(), path.into_os_string()],
            }
        );
    }

    #[test]
    fn rscript_selection_copy_and_filters_are_platform_correct() {
        assert_eq!(
            rscript_selection_spec(DesktopPlatform::Windows),
            RscriptSelectionSpec {
                display_name: "Rscript.exe",
                picker_title: "Choose Rscript.exe",
                picker_extension: Some("exe"),
            }
        );
        for platform in [DesktopPlatform::Macos, DesktopPlatform::Linux] {
            assert_eq!(
                rscript_selection_spec(platform),
                RscriptSelectionSpec {
                    display_name: "Rscript",
                    picker_title: "Choose Rscript",
                    picker_extension: None,
                }
            );
        }
    }

    #[test]
    fn architecture_requirement_copy_is_platform_specific() {
        assert_eq!(
            r_architecture_requirement_for(DesktopPlatform::Macos),
            "Rho for Apple Silicon requires arm64 R"
        );
        assert_eq!(
            r_architecture_requirement_for(DesktopPlatform::Linux),
            "Rho for Linux x64 requires x86_64 R"
        );
        assert_eq!(
            r_architecture_requirement_message_for(DesktopPlatform::Linux),
            "Rho for Linux x64 requires an x86_64 R 4.4 or later installation."
        );
        assert_eq!(
            r_architecture_requirement_message_for(DesktopPlatform::Macos),
            "Rho for Apple Silicon requires an arm64 R 4.4 or later installation."
        );
    }

    #[test]
    fn linux_reveal_opens_the_parent_directory() {
        let path = PathBuf::from("/tmp/Rho logs/startup.log");
        assert_eq!(
            reveal_path_spec(DesktopPlatform::Linux, &path),
            CommandSpec {
                program: "xdg-open".into(),
                arguments: vec![PathBuf::from("/tmp/Rho logs").into_os_string()],
            }
        );
    }

    #[test]
    fn process_path_is_ordered_deduplicated_and_drops_empty_entries() {
        let joined = merge_process_path(
            [
                PathBuf::from("/usr/bin"),
                PathBuf::new(),
                PathBuf::from("/opt/homebrew/bin"),
            ],
            Some(Path::new("/Library/Frameworks/R.framework/Resources/bin")),
            [
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
            ],
            Some(PathBuf::from("/Users/研究者/Rho Home/.local/bin")),
        )
        .unwrap();
        assert_eq!(
            std::env::split_paths(&joined).collect::<Vec<_>>(),
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/Library/Frameworks/R.framework/Resources/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/Users/研究者/Rho Home/.local/bin"),
            ]
        );
    }
}
