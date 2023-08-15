use directories::ProjectDirs;
use std::{env, ffi::c_void, fs, io, path, process};
use windows::{
    w,
    Win32::{
        Foundation::{NO_ERROR, WIN32_ERROR},
        System::Registry::{
            RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6432KEY,
        },
    },
};

fn copy_dir_all(from: path::PathBuf, to: path::PathBuf) -> io::Result<()> {
    fs::create_dir_all(&to)?;
    for ecorery in fs::read_dir(from)? {
        let ecorery = ecorery?;
        if ecorery.file_type()?.is_dir() {
            copy_dir_all(ecorery.path(), to.join(ecorery.file_name()))?;
        } else {
            fs::copy(ecorery.path(), to.join(ecorery.file_name()))?;
        }
    }
    Ok(())
}

struct CoreJsData(String, String);

fn patch_core(js_path: impl AsRef<path::Path>) -> Result<CoreJsData, &'static str> {
    let original_js = fs::read_to_string(&js_path).or(Err("cannot read original js"))?;
    let server_js = include_str!("../dist/qplugged-server.js");
    let binding = env::current_exe().or(Err("cannot get current dir"))?;
    let qp_dir = binding
        .parent()
        .ok_or("cannot get current dir")?
        .to_str()
        .ok_or("cannot get current dir")?;
    let qp_dir = qp_dir
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\"", "\\\"")
        .replace("\'", "\\\'")
        .replace("\\", "\\\\");
    let patched_js =
        format!("\"use strict\";global.__QP_DIR=\"{qp_dir}\";{server_js}{original_js}");
    let js_data: CoreJsData = CoreJsData(original_js, patched_js);
    Ok(js_data)
}

fn copy_core_dir(from: path::PathBuf, to: path::PathBuf) -> io::Result<()> {
    if to.exists() {
        fs::remove_dir_all(to.clone())?;
    }

    copy_dir_all(from, to)?;
    Ok(())
}

fn get_core_dir() -> Result<path::PathBuf, WIN32_ERROR> {
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
        return Err(result);
    }

    Ok(path::PathBuf::new()
        .join(String::from_utf16(&value).or(Err(NO_ERROR))?)
        .parent()
        .ok_or(NO_ERROR)?
        .to_path_buf())
}

pub fn main() -> Result<(), &'static str> {
    let app_data_dir = ProjectDirs::from("com", "QPlugged", "Server")
        .ok_or("cannot get app_data_dir")?
        .data_local_dir()
        .to_path_buf();
    let core_dir = get_core_dir().or(Err("cannot get core dir"))?;
    let copied_core_dir = app_data_dir.join("core");
    let copied_core_executable = copied_core_dir.join("QQ.exe");
    let version_json_file = core_dir
        .join("resources")
        .join("app")
        .join("versions")
        .join("config.json");
    let version_json_data =
        fs::read_to_string(version_json_file.clone()).or(Err("cannot read version info file"))?;
    let stored_version_json_path = app_data_dir.join("core_config.json");
    let stored_version_json_data =
        fs::read_to_string(stored_version_json_path.clone()).unwrap_or(String::new());
    if version_json_data != stored_version_json_data {
        copy_core_dir(core_dir.clone(), copied_core_dir.clone())
            .or(Err("failed to copy core dir"))?;
        fs::copy(version_json_file, stored_version_json_path)
            .or(Err("failed to copy version info file"))?;
    }

    let js_path = copied_core_dir
        .join("resources")
        .join("app")
        .join("app_launcher")
        .join("index.js");
    let js_data = patch_core(&js_path).or(Err("cannot read js data"))?;
    let original_js = js_data.0;
    let patched_js = js_data.1;

    let mut child = process::Command::new(copied_core_executable)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .spawn()
        .or(Err("cannot spawn core process"))?;
    let stdout = child.stdout.take().ok_or("cannot get core stdout")?;

    const PORT_START_FLAG: &str = "[QPLUGGED_INIT_PORT]";
    const PORT_END_FLAG: &str = "[/]";
    let mut is_code_injected = false;
    let mut f = io::BufReader::new(stdout);
    loop {
        let mut buf = String::new();
        io::BufRead::read_line(&mut f, &mut buf).or(Err("cannot read stdout"))?;
        print!("{buf}");
        if buf.contains("[preload]") && !is_code_injected {
            io::Write::write_all(
                &mut fs::File::create(js_path.clone()).unwrap(),
                patched_js.as_bytes(),
            )
            .or(Err("failed to write patched js data"))?;
            is_code_injected = true;
        } else if buf.contains(PORT_START_FLAG) && buf.contains(PORT_END_FLAG) && is_code_injected {
            io::Write::write_all(
                &mut fs::File::create(js_path.clone()).unwrap(),
                original_js.as_bytes(),
            )
            .or(Err("failed to write original js data back"))?;
            break;
        }
    }
    child.wait().or(Err("failed to wait core process exit"))?;
    Ok(())
}
