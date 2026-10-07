use std::{env, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        return;
    }

    let emsdk = PathBuf::from(
        env::var_os("FWOK_EMSDK_ROOT")
            .expect("FWOK_EMSDK_ROOT must point to the Emscripten SDK for WASM builds"),
    );
    let emscripten = emsdk.join("upstream/emscripten");
    let compiler = emsdk.join("upstream/bin/wasm32-clang.exe");
    let helper = emscripten.join("system/lib/compiler-rt/emscripten_setjmp.c");

    println!("cargo:rerun-if-env-changed=FWOK_EMSDK_ROOT");
    println!("cargo:rerun-if-changed={}", helper.display());

    cc::Build::new()
        .target("wasm32-unknown-unknown")
        .compiler(compiler)
        .file(helper)
        .include(emscripten.join("cache/sysroot/include"))
        .include(emscripten.join("system/lib/libc"))
        .define("__WASM_SJLJ__", None)
        .define("NDEBUG", None)
        .flag("-mllvm")
        .flag("-wasm-enable-sjlj")
        .flag("-mllvm")
        .flag("-wasm-use-legacy-eh=false")
        .compile("fwok_wasm_setjmp");
}
