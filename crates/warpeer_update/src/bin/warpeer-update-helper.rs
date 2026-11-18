//! Runs outside the installation so Windows can replace all DLLs.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    use std::{path::PathBuf, process::Command};
    use windows::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        return;
    }
    let Some(pid) = args[1].to_str().and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let Some(tag) = args[2].to_str() else {
        return;
    };
    let exe = PathBuf::from(&args[3]);
    let exited = unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(handle) => {
                let result = WaitForSingleObject(handle, 60_000);
                let _ = CloseHandle(handle);
                result == WAIT_OBJECT_0
            }
            Err(error) => error.code() == windows::core::HRESULT::from_win32(87),
        }
    };
    if !exited {
        return;
    }
    if let Err(error) = warpeer_update::install_at(tag, &exe) {
        if let Some(root) = exe.parent() {
            let _ = std::fs::write(root.join(".warpeer-update-error"), format!("{error:#}"));
        }
    }
    let _ = Command::new(exe).spawn();
    let _ = self_replace::self_delete();
}

#[cfg(not(windows))]
fn main() {}
