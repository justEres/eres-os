use std::env;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(eres_kernel)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=boot/boot.S");
    println!("cargo:rerun-if-changed=boot/stage2.S");
    println!("cargo:rerun-if-changed=build/linker.ld");

    if matches!(env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("none")) {
        println!("cargo:rustc-cfg=eres_kernel");
    }
}
