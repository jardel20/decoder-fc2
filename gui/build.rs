//! rfd loads libdbus with dlopen; retain its resolved runtime directory in ELF builds.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn dbus_directory() -> Option<PathBuf> {
    if let Ok(output) = Command::new("pkg-config")
        .args(["--variable=libdir", "dbus-1"])
        .output()
        && output.status.success()
    {
        let directory = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
        if directory.is_absolute() && directory.join("libdbus-1.so.3").is_file() {
            return Some(directory);
        }
    }
    // NixOS exposes the daemon in the system profile while its library lives in
    // a separate output. Resolve from the installed daemon, without hardcoding a store hash.
    let daemon = Path::new("/run/current-system/sw/bin/dbus-daemon");
    if daemon.is_file() {
        let output = Command::new("ldd").arg(daemon).output().ok()?;
        if !output.status.success() {
            return None;
        }
        for line in String::from_utf8(output.stdout).ok()?.lines() {
            let mut words = line.split_whitespace();
            if words.next() == Some("libdbus-1.so.3") && words.next() == Some("=>") {
                let library = Path::new(words.next()?);
                if library.is_absolute() && library.is_file() {
                    return library.parent().map(Path::to_path_buf);
                }
            }
        }
    }
    None
}
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    if std::env::var("HOST").ok() != std::env::var("TARGET").ok() {
        return;
    }
    println!("cargo:rerun-if-changed=/run/current-system/sw/bin/dbus-daemon");
    if let Some(directory) = dbus_directory() {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", directory.display());
    }
}
