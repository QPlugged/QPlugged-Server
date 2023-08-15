use std::{env, fs, path, process};

fn main() {
    println!("cargo:rerun-if-changed=prebuild.js");
    println!("cargo:rerun-if-changed=server");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=yarn.lock");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(target_os = "windows")]
    let pkg_manager = "yarn.cmd";
    #[cfg(not(target_os = "windows"))]
    let pkg_manager = "yarn";
    let profile = env::var("PROFILE").unwrap();
    let target = env::var("TARGET").unwrap();
    let mut out_dir = path::Path::new("target").join("release");
    if !out_dir.exists() {
        out_dir = out_dir.join(target.clone());
    }
    if !out_dir.exists() {
        panic!("output directory not found");
    }
    let paths = match target.as_str() {
        "x86_64-pc-windows-msvc" => ("silk_codec-windows-static-x64.exe", "silk-codec.exe"),
        "i586-pc-windows-msvc" => ("silk_codec-windows-static-x86.exe", "silk-codec.exe"),
        "i686-pc-windows-msvc" => ("silk_codec-windows-static-x86.exe", "silk-codec.exe"),
        "i686-pc-windows-gnu" => ("silk_codec-windows-static-x86.exe", "silk-codec.exe"),
        "x86_64-unknown-linux-gnu" => ("silk_codec-linux-x64", "silk-codec"),
        "x86_64-unknown-linux-musl" => ("silk_codec-linux-x64", "silk-codec"),
        _ => panic!("unsupported target!"),
    };
    fs::copy(
        path::Path::new("helper").join(paths.0),
        out_dir.join(paths.1),
    )
    .unwrap();
    process::Command::new(pkg_manager)
        .args(["node", "./prebuild.js"])
        .env(
            "NODE_ENV",
            match profile.as_str() {
                "debug" => "development",
                _ => "production",
            },
        )
        .output()
        .unwrap();
}
