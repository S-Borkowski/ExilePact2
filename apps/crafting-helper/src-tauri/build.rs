use std::{env, fs, path::PathBuf};

fn main() {
    // tauri-build links the Visual C++ runtime statically by default, and
    // to do it puts a stand-in `msvcrt.lib` in this crate's OUT_DIR and adds
    // that folder to the native search path. Cargo hands every build
    // script's search path to every doctest of a workspace run, so
    // `cargo test --workspace` then links another crate's doctest (the
    // ch-blueprint one) against the stand-in and fails with unresolved
    // runtime symbols. The same static link is done below, scoped to this
    // crate's binary alone.
    let windows = tauri_build::WindowsAttributes::new().static_vc_runtime(false);
    if let Err(error) = tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)) {
        println!("{error:#}");
        std::process::exit(1);
    }
    static_vc_runtime_for_the_binary();
}

/// The static Visual C++ runtime for the app's binary only: an empty
/// `msvcrt.lib` in a folder of its own, found through `/LIBPATH` on the
/// binary's link line (not a search path every target inherits), and the
/// runtime libraries swapped for their static versions. The library bytes
/// are tauri-build's own (`static_vcruntime.rs`, 2.7), as is the set of
/// libraries; the release exe imports no `VCRUNTIME140.dll`.
fn static_vc_runtime_for_the_binary() {
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let machine: &[u8] = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => &[0x64, 0x86],
        Ok("x86") => &[0x4C, 0x01],
        _ => return,
    };
    let body: &[u8] = &[
        1, 0, 94, 3, 96, 98, 60, 0, 0, 0, 1, 0, 0, 0, 0, 0, 132, 1, 46, 100, 114, 101, 99, 116,
        118, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 10, 16, 0, 46, 100, 114, 101, 99, 116, 118, 101, 0, 0, 0, 0, 1, 0, 0, 0, 3, 0, 4,
        0, 0, 0,
    ];
    let dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("static-vc-runtime");
    fs::create_dir_all(&dir).expect("create the static runtime folder");
    fs::write(dir.join("msvcrt.lib"), [machine, body].concat()).expect("write msvcrt.lib");

    println!("cargo:rustc-link-arg-bins=/LIBPATH:{}", dir.display());
    for lib in [
        "libvcruntimed.lib",
        "vcruntime.lib",
        "vcruntimed.lib",
        "libcmtd.lib",
        "msvcrt.lib",
        "msvcrtd.lib",
        "libucrt.lib",
        "libucrtd.lib",
    ] {
        println!("cargo:rustc-link-arg-bins=/NODEFAULTLIB:{lib}");
    }
    for lib in ["libcmt.lib", "libvcruntime.lib", "ucrt.lib"] {
        println!("cargo:rustc-link-arg-bins=/DEFAULTLIB:{lib}");
    }
}
