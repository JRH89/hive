//! Best-effort detection in the launching client, never in the daemon.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub(crate) fn launcher() -> Option<&'static Path> {
    static LAUNCHER: OnceLock<Option<PathBuf>> = OnceLock::new();
    LAUNCHER.get_or_init(detect).as_deref()
}

fn program_name(executable: &Path) -> Option<String> {
    let name = executable.file_name()?.to_str()?.to_ascii_lowercase();
    Some(name.strip_suffix(".exe").unwrap_or(&name).to_string())
}

fn nearest_shell(
    client: u32,
    mut process: impl FnMut(u32) -> Option<(u32, PathBuf)>,
) -> Option<(u32, PathBuf)> {
    let mut pid = process(client)?.0;
    for _ in 0..8 {
        let (parent, program) = process(pid)?;
        match program_name(&program)?.as_str() {
            "bash" | "zsh" | "fish" | "sh" | "dash" | "ksh" | "mksh" | "ash" | "csh" | "tcsh"
            | "nu" | "xonsh" | "elvish" | "pwsh" | "powershell" | "cmd" => {
                return Some((pid, program));
            }
            // Package-manager shims can put a runtime between Hive and its shell.
            "node" | "nodejs" | "bun" | "bunx" | "npm" | "npx" | "pnpm" | "yarn" | "env" => {}
            _ => return None,
        }
        if parent == 0 || parent == pid {
            break;
        }
        pid = parent;
    }
    None
}

#[cfg(target_os = "linux")]
fn detect() -> Option<PathBuf> {
    nearest_shell(std::process::id(), |pid| {
        let proc = PathBuf::from(format!("/proc/{pid}"));
        let status = std::fs::read_to_string(proc.join("status")).ok()?;
        let parent = status.lines().find_map(|line| line.strip_prefix("PPid:"))?;
        Some((
            parent.trim().parse().ok()?,
            std::fs::read_link(proc.join("exe")).ok()?,
        ))
    })
    .map(|(_, program)| program)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn detect() -> Option<PathBuf> {
    let output = std::process::Command::new("/bin/ps")
        .args(["-axww", "-o", "pid=,ppid=,comm="])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let rows = String::from_utf8(output.stdout).ok()?;
    nearest_shell(std::process::id(), |pid| {
        rows.lines().find_map(|line| {
            let (id, rest) = line.trim_start().split_once(char::is_whitespace)?;
            if id.parse::<u32>().ok()? != pid {
                return None;
            }
            let (parent, command) = rest.trim_start().split_once(char::is_whitespace)?;
            Some((parent.parse().ok()?, PathBuf::from(command.trim_start())))
        })
    })
    .map(|(_, program)| program)
}

#[cfg(windows)]
fn detect() -> Option<PathBuf> {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    let mut processes = HashMap::new();
    // The snapshot and process handles are local to these calls and closed below.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = Process32FirstW(snapshot, &mut entry);
        while found != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|&ch| ch == 0)
                .unwrap_or(entry.szExeFile.len());
            let executable = OsString::from_wide(&entry.szExeFile[..length]);
            processes.insert(
                entry.th32ProcessID,
                (entry.th32ParentProcessID, PathBuf::from(executable)),
            );
            found = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);

        let (pid, _) = nearest_shell(std::process::id(), |pid| processes.remove(&pid))?;
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buffer = vec![0; 32_768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut length,
        );
        CloseHandle(handle);
        if result == 0 {
            return None;
        }

        let path = PathBuf::from(OsString::from_wide(&buffer[..length as usize]));
        let system_directory = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str());
        // Windows' own bash.exe starts WSL. A native Bash has a different path.
        if program_name(&path).as_deref() == Some("bash")
            && system_directory.is_some_and(|name| {
                name.eq_ignore_ascii_case("System32") || name.eq_ignore_ascii_case("SysWOW64")
            })
        {
            return None;
        }
        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_nearest_recognized_shell_through_a_package_launcher() {
        let launcher = nearest_shell(4, |pid| {
            let (parent, program) = match pid {
                4 => (3, "hive.exe"),
                3 => (2, "node.exe"),
                2 => (1, "pwsh.exe"),
                1 => (0, "bash.exe"),
                _ => return None,
            };
            Some((parent, PathBuf::from(program)))
        });
        assert_eq!(launcher, Some((2, PathBuf::from("pwsh.exe"))));
    }

    #[test]
    fn stops_at_an_unrelated_parent() {
        assert!(
            nearest_shell(3, |pid| {
                match pid {
                    3 => Some((2, "hive")),
                    2 => Some((1, "ghostty")),
                    1 => Some((0, "bash")),
                    _ => None,
                }
                .map(|(parent, name)| (parent, PathBuf::from(name)))
            })
            .is_none()
        );
    }
}
