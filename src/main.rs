use crate::fs_extra::{copy_dir_all_empty, write_file_str};
use async_ctrlc::CtrlC;
use directories::ProjectDirs;
use escape::escape_js_str;
use litcrypt::{lc, use_litcrypt};
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

mod escape;
mod fs_extra;

use_litcrypt!();

async fn patch_core(js_path: PathBuf, data_dir: PathBuf) -> Result<String, ()> {
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
    let res_dir = escape_js_str(binding.parent().ok_or(())?.to_str().ok_or(())?);
    let data_dir = escape_js_str(data_dir.to_str().ok_or(())?);
    let patched_js = format!(
        "\"{}\";{}\"{}\";{}\"{}\";{};{}",
        lc!("use strict"),
        lc!("global.__QP_RES_DIR="),
        res_dir,
        lc!("global.__QP_DATA_DIR="),
        data_dir,
        server_js,
        original_js
    );
    Ok(patched_js)
}

async fn get_app_dir() -> Option<PathBuf> {
    let app_data_dir = ProjectDirs::from(&lc!("com"), &lc!("QPlugged"), &lc!("Server"))?
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
            .join(lc!("QQ.exe")),
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

pub async fn patch() -> Result<(), String> {
    let app_data_dir = get_app_dir()
        .await
        .ok_or(lc!("Cannot get AppData directory."))?;
    let core_executable = get_core_executable()
        .await
        .ok_or(lc!("Cannot get core executable. Is the core installed?"))?;
    let core_dir = core_executable
        .parent()
        .ok_or(lc!("Cannot get core directory."))?
        .to_path_buf();
    let copied_core_dir = app_data_dir.join(lc!("core"));
    let copied_core_executable = copied_core_dir.join(
        core_executable
            .file_name()
            .ok_or(lc!("Cannot get core executable filename."))?,
    );

    #[cfg(target_os = "windows")]
    let version_json_file = core_dir
        .join(lc!("resources"))
        .join(lc!("app"))
        .join(lc!("versions"))
        .join(lc!("config.json"));
    #[cfg(not(target_os = "windows"))]
    let version_json_file = core_dir
        .join(lc!("resources"))
        .join(lc!("app"))
        .join(lc!("package.json"));
    let version_json_data = read_to_string(version_json_file.clone())
        .await
        .or(Err(format!(
            "{}{}",
            lc!("Failed to read version configuration file: "),
            version_json_file.display(),
        )))?;
    let stored_version_json_file = app_data_dir.join(lc!("core_config.json"));
    let stored_version_json_data = read_to_string(stored_version_json_file.clone())
        .await
        .unwrap_or(String::new());
    if version_json_data != stored_version_json_data {
        copy_dir_all_empty(core_dir.clone(), copied_core_dir.clone())
            .await
            .or(Err(format!(
                "{}{}{}{}",
                lc!("Failed to copy core directory, from: "),
                core_dir.display(),
                lc!(" to: "),
                copied_core_dir.display()
            )))?;
        copy(version_json_file.clone(), stored_version_json_file.clone())
            .await
            .or(Err(format!(
                "{}{}{}{}",
                lc!("Failed to copy version configuration file, from: "),
                version_json_file.display(),
                lc!(" to: "),
                stored_version_json_file.display(),
            )))?;
    }

    let js_path = core_dir
        .join(lc!("resources"))
        .join(lc!("app"))
        .join(lc!("app_launcher"))
        .join(lc!("index.js"));
    let copied_js_path = copied_core_dir
        .join(lc!("resources"))
        .join(lc!("app"))
        .join(lc!("app_launcher"))
        .join(lc!("index.js"));
    copy(js_path.clone(), copied_js_path.clone())
        .await
        .or(Err(format!(
            "{}{}{}{}",
            lc!("Failed to copy entry js, from: "),
            js_path.display(),
            lc!(" to: "),
            copied_js_path.display(),
        )))?;
    let patched_js = patch_core(copied_js_path.clone(), app_data_dir)
        .await
        .or(Err(format!(
            "{}{}",
            lc!("Failed to read entry js: "),
            js_path.display()
        )))?;

    let mut child = Command::new(copied_core_executable.clone())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .or(Err(format!(
            "{}{}",
            lc!("Failed to spawn core process: "),
            copied_core_executable.display(),
        )))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(lc!("Failed to get core process stdout handle."))?;

    async fn inject_js_file(js_path: PathBuf, js_data: String) -> Result<(), String> {
        write_file_str(js_path.clone(), &js_data)
            .await
            .or(Err(format!(
                "{}{}",
                lc!("Failed to write entry js: "),
                js_path.display(),
            )))?;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let mut is_code_injected = false;
    #[cfg(target_os = "linux")]
    inject_js_file(copied_js_path.clone(), patched_js.clone()).await?;

    let read_loop = async {
        let mut f = BufReader::new(stdout);
        loop {
            let mut buf = String::new();
            if f.read_line(&mut buf).await.is_err() {
                break;
            }
            print!("{buf}");
            #[cfg(not(target_os = "linux"))]
            if buf.contains(&lc!(
                "c48457ac24c9ad33bd2e0250f734116535df7e8caeb043a6d3164fd6ea65046c"
            )) && !is_code_injected
            {
                inject_js_file(copied_js_path.clone(), patched_js.clone()).await?;
                is_code_injected = true;
            }
        }
        Ok::<(), String>(())
    };

    let wait_core = async {
        child
            .wait()
            .await
            .or(Err(lc!("Failed to wait core process to exit.")))?;
        Ok::<(), String>(())
    };

    let ctrlc_handler = async {
        CtrlC::new()
            .or(Err(lc!("Failed to create Ctrl+C handler.")))?
            .await;
        Ok::<(), String>(())
    };

    tokio::select! {
        v = read_loop => v,
        v = wait_core => v,
        v = ctrlc_handler => v,
    }?;

    child.kill().await.unwrap_or(());

    Ok(())
}

#[tokio::main]
pub async fn main() -> Result<(), String> {
    let result = patch().await;
    #[cfg(debug_assertions)]
    result?;
    #[cfg(not(debug_assertions))]
    result.or(Err("Unknown Error"))?;
    Ok(())
}
