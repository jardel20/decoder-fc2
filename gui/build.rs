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
    println!("cargo:rerun-if-changed=../assets/app_icon.ico");
    println!("cargo:rerun-if-env-changed=ProgramFiles(x86)");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_icon();
        return;
    }
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

fn windows_icon() {
    let target = std::env::var("TARGET").expect("Cargo TARGET");
    assert!(
        target.ends_with("-msvc"),
        "The portable Windows build requires the MSVC target"
    );
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let icon = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../assets/app_icon.ico")
        .canonicalize()
        .expect("application icon");
    let resource = out.join("app_icon.rc");
    let compiled = out.join("app_icon.res");
    let icon_path = icon.display().to_string();
    let icon_path = icon_path.strip_prefix("\\\\?\\").unwrap_or(&icon_path);
    std::fs::write(
        &resource,
        format!(
            "#pragma code_page(65001)\n1 ICON \"{}\"\n",
            icon_path.replace('\\', "\\\\")
        ),
    )
    .expect("write icon resource");
    let compiler = resource_compiler().expect("Windows resource compiler rc.exe not found. Install the Windows SDK with Visual Studio C++ Build Tools.");
    let result = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&compiled)
        .arg(&resource)
        .status()
        .expect("run Windows resource compiler");
    assert!(
        result.success(),
        "Windows application icon could not be embedded"
    );
    println!(
        "cargo:rustc-link-arg-bin=fc2-explorer={}",
        compiled.display()
    );
}

fn resource_compiler() -> Option<PathBuf> {
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some("rc.exe".into());
    }
    let kits = PathBuf::from(std::env::var_os("ProgramFiles(x86)")?).join("Windows Kits/10/bin");
    let mut versions: Vec<_> = std::fs::read_dir(kits)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    let architecture = if std::env::var("HOST").ok()?.starts_with("aarch64") {
        "arm64"
    } else {
        "x64"
    };
    versions
        .into_iter()
        .rev()
        .map(|path| path.join(architecture).join("rc.exe"))
        .find(|path| path.is_file())
}
