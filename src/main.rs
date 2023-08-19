use crate::fs_extra::{copy_dir_all_empty, write_file_str};
use async_ctrlc::CtrlC;
use directories::ProjectDirs;
#[cfg(target_os = "windows")]
use std::ffi::c_void;
use std::{env, path::PathBuf, process::Stdio};
#[cfg(target_os = "linux")]
use tokio::fs::{read_link, try_exists};
use tokio::{
    fs::{copy, create_dir_all, read_to_string},
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
#[cfg(target_os = "windows")]
use windows::{
    w,
    Win32::{Foundation::*, System::Registry::*},
};

mod fs_extra;

async fn patch_core(js_path: PathBuf) -> Result<(String, String), ()> {
    let original_js = tokio::fs::read_to_string(&js_path).await.or(Err(()))?;
    let magic = include_bytes!("../dist/qplugged-server.js.magic");
    let server_js = String::from_utf8(
        include_bytes!("../dist/qplugged-server.js.encrypted")
            .iter()
            .map(|num| num ^ magic[0])
            .collect(),
    )
    .or(Err(()))?;
    let binding = env::current_exe().or(Err(()))?;
    let qp_dir = binding.parent().ok_or(())?.to_str().ok_or(())?;
    let qp_dir = qp_dir
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\"", "\\\"")
        .replace("\'", "\\\'")
        .replace("\\", "\\\\");
    let patched_js =
        format!("\"use strict\";global.__QP_DIR=\"{qp_dir}\";{server_js}{original_js}");
    Ok((original_js, patched_js))
}

async fn get_app_dir() -> Option<PathBuf> {
    let app_data_dir = ProjectDirs::from("com", "QPlugged", "Server")?
        .data_local_dir()
        .to_path_buf();
    create_dir_all(app_data_dir.clone()).await.ok()?;
    Some(app_data_dir)
}

#[cfg(target_os = "windows")]
async fn get_core_executable() -> Option<PathBuf> {
    let mut value: Vec<u16> = vec![0; 1024];

    let mut my_size = (std::mem::size_of::<u16>() * value.len()) as u32;
    let size = &mut my_size as *mut u32;

    let result = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\QQ"),
            w!("UninstallString"),
            RRF_SUBKEY_WOW6432KEY | RRF_RT_REG_SZ,
            Some(std::ptr::null_mut()),
            Some(value.as_mut_ptr() as *mut c_void),
            Some(size),
        )
    };

    if result != NO_ERROR {
        return None;
    }

    Some(
        PathBuf::new()
            .join(String::from_utf16(&value).ok()?)
            .parent()?
            .to_path_buf()
            .join("QQ.exe"),
    )
}

#[cfg(target_os = "linux")]
async fn get_core_executable() -> Option<PathBuf> {
    let possible_files = vec![
        "/usr/bin/linuxqq",
        "/usr/bin/qq",
        "/var/lib/flatpak/app/com.qq.QQ/current/active/files/extra/QQ/qq",
    ];
    for file in possible_files {
        if try_exists(file).await.ok() == Some(true) {
            if let Some(exe) = read_link(file).await.ok() {
                return Some(exe);
            } else {
                return Some(file.into());
            }
        }
    }
    None
}

#[tokio::main]
pub async fn main() -> Result<(), String> {
    let app_data_dir = get_app_dir().await.ok_or("Cannot get AppData directory.")?;
    let core_executable = get_core_executable()
        .await
        .ok_or("Cannot get core executable. Is the core installed?")?;
    let core_dir = core_executable
        .parent()
        .ok_or("Cannot get core directory.")?
        .to_path_buf();
    let copied_core_dir = app_data_dir.join("core");
    let copied_core_executable = copied_core_dir.join(
        core_executable
            .file_name()
            .ok_or("Cannot get core executable filename.")?,
    );

    #[cfg(target_os = "windows")]
    let version_json_file = core_dir
        .join("resources")
        .join("app")
        .join("versions")
        .join("config.json");
    #[cfg(not(target_os = "windows"))]
    let version_json_file = core_dir.join("resources").join("app").join("package.json");
    let version_json_data = read_to_string(version_json_file.clone())
        .await
        .or(Err(format!(
            "Failed to read version configuration file: {}",
            version_json_file.display(),
        )))?;
    let stored_version_json_file = app_data_dir.join("core_config.json");
    let stored_version_json_data = read_to_string(stored_version_json_file.clone())
        .await
        .unwrap_or(String::new());
    if version_json_data != stored_version_json_data {
        copy_dir_all_empty(core_dir.clone(), copied_core_dir.clone())
            .await
            .or(Err(format!(
                "Failed to copy core directory, from: {} to: {}",
                core_dir.display(),
                copied_core_dir.display()
            )))?;
        copy(version_json_file.clone(), stored_version_json_file.clone())
            .await
            .or(Err(format!(
                "Failed to copy version configuration file, from: {} to: {}",
                version_json_file.display(),
                stored_version_json_file.display(),
            )))?;
    }

    let js_path = copied_core_dir
        .join("resources")
        .join("app")
        .join("app_launcher")
        .join("index.js");
    let (original_js, patched_js) = patch_core(js_path.clone()).await.or(Err(format!(
        "Failed to read entry js: {}",
        js_path.display()
    )))?;

    let mut child = Command::new(copied_core_executable.clone())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .or(Err(format!(
            "Failed to spawn core process: {}",
            copied_core_executable.display(),
        )))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Failed to get core process stdout handle.")?;

    async fn inject_js_file(js_path: PathBuf, js_data: String) -> Result<(), String> {
        write_file_str(js_path.clone(), &js_data)
            .await
            .or(Err(format!(
                "Failed to write entry js: {}",
                js_path.display(),
            )))?;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let mut is_code_injected = false;
    #[cfg(target_os = "linux")]
    let is_code_injected = true;
    #[cfg(target_os = "linux")]
    inject_js_file(js_path.clone(), patched_js.clone()).await?;

    let read_loop = async {
        let mut f = BufReader::new(stdout);
        loop {
            let mut buf = String::new();
            if f.read_line(&mut buf).await.is_err() {
                break;
            }
            print!("{buf}");
            #[cfg(not(target_os = "linux"))]
            if buf.contains("[preload]") && !is_code_injected {
                inject_js_file(js_path.clone(), patched_js.clone()).await?;
                is_code_injected = true;
            }
            if buf.contains("[QPLUGGED_STARTED_SUCCESSFULLY]") && is_code_injected {
                inject_js_file(js_path.clone(), original_js.clone()).await?;
            }
        }
        Ok::<(), String>(())
    };

    let wait_core = async {
        child
            .wait()
            .await
            .or(Err("Failed to wait core process to exit."))?;
        Ok::<(), String>(())
    };

    let ctrlc_handler = async {
        CtrlC::new()
            .or(Err("Failed to create Ctrl+C handler."))?
            .await;
        Ok::<(), String>(())
    };

    tokio::select! {
        v = read_loop => v,
        v = wait_core => v,
        v = ctrlc_handler => v,
    }?;

    inject_js_file(js_path.clone(), original_js.clone()).await?;

    child.kill().await.unwrap_or(());

    Ok(())
}
