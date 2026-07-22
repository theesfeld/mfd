//! Build pure assembly libvge and link it.
//! On x86_64: assemble .s only (no C in the library).
//! Elsewhere: compile portable C reference (non-asm hosts only).

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let force_c = env::var("VGE_FORCE_C").is_ok();
    let use_asm = target.starts_with("x86_64-") && !force_c;

    println!("cargo:rerun-if-changed=asm/x86_64/vge.s");
    println!("cargo:rerun-if-changed=asm/x86_64/vge_extra.s");
    println!("cargo:rerun-if-changed=c/vge_portable.c");
    println!("cargo:rerun-if-changed=include/vge.h");
    println!("cargo:rerun-if-env-changed=VGE_FORCE_C");
    println!("cargo:rustc-check-cfg=cfg(vge_asm)");
    if use_asm {
        let objs = ["vge.o", "vge_extra.o"];
        let srcs = ["asm/x86_64/vge.s", "asm/x86_64/vge_extra.s"];
        let mut paths = Vec::new();
        for (src, obj_name) in srcs.iter().zip(objs.iter()) {
            let obj = out.join(obj_name);
            let status = Command::new("as")
                .args(["--64", "-o"])
                .arg(&obj)
                .arg(manifest.join(src))
                .status()
                .expect("GNU as required for libvge");
            if !status.success() {
                panic!("as failed on {src}");
            }
            paths.push(obj);
        }
        let lib = out.join("libvge_asm.a");
        let mut ar = Command::new("ar");
        ar.arg("rcs").arg(&lib);
        for p in &paths {
            ar.arg(p);
        }
        let status = ar.status().expect("ar");
        if !status.success() {
            panic!("ar failed");
        }
        println!("cargo:rustc-link-search=native={}", out.display());
        println!("cargo:rustc-link-lib=static=vge_asm");
        println!("cargo:rustc-cfg=vge_asm");
        // Demo FFI only — library itself is pure asm, no -lm.
        println!("cargo:warning=VGE demo: linking pure-asm libvge (no C, no libm)");
    } else {
        panic!(
            "libvge is pure x86_64 assembly. Non-x86_64 is not a product target. \
             Set VGE_FORCE_C only for experiments (unsupported)."
        );
    }
}
