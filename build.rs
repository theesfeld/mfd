//! Demo/FFI build: **link pure-asm libvge**. Never implement the engine here.
//!
//! Preference order:
//! 1. `build/libvge.a` from `make static` (canonical product)
//! 2. Assemble `asm/x86_64/*.s` into OUT_DIR (same objects, for CI convenience)

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    println!("cargo:rerun-if-changed=asm/x86_64/vge.s");
    println!("cargo:rerun-if-changed=asm/x86_64/vge_extra.s");
    println!("cargo:rerun-if-changed=asm/x86_64/vge_aa.s");
    println!("cargo:rerun-if-changed=include/vge.h");
    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rerun-if-changed=build/libvge.a");
    println!("cargo:rustc-check-cfg=cfg(vge_asm)");

    if !target.starts_with("x86_64-") {
        panic!("libvge is pure x86_64 assembly. This target is not supported.");
    }

    let make_lib = manifest.join("build/libvge.a");
    if make_lib.is_file() {
        println!("cargo:rustc-link-search=native={}", manifest.join("build").display());
        println!("cargo:rustc-link-lib=static=vge");
        println!("cargo:rustc-cfg=vge_asm");
        println!("cargo:warning=demo: linking make-built build/libvge.a (pure asm)");
        return;
    }

    // Assemble the same pure-asm sources the Makefile uses.
    let srcs = [
        "asm/x86_64/vge.s",
        "asm/x86_64/vge_extra.s",
        "asm/x86_64/vge_aa.s",
    ];
    let mut objs = Vec::new();
    for src in srcs {
        let name = PathBuf::from(src)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned()
            + ".o";
        let obj = out.join(&name);
        let st = Command::new("as")
            .args(["--64", "-o"])
            .arg(&obj)
            .arg(manifest.join(src))
            .status()
            .expect("GNU as required");
        if !st.success() {
            panic!("as failed: {src}");
        }
        objs.push(obj);
    }
    let lib = out.join("libvge.a");
    let mut ar = Command::new("ar");
    ar.arg("rcs").arg(&lib);
    for o in &objs {
        ar.arg(o);
    }
    if !ar.status().expect("ar").success() {
        panic!("ar failed");
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=vge");
    println!("cargo:rustc-cfg=vge_asm");
    println!("cargo:warning=demo: assembled pure-asm libvge into OUT_DIR (run `make` for canonical build/)");
}
