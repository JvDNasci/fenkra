//! Compila a fachada C++ se o OpenCASCADE for encontrado.
//! Procura em `OCCT_ROOT` ou no vcpkg (`C:\dev\vcpkg\installed\x64-windows-release`).
//! Sem OCCT, o crate compila em modo "sem kernel" (as operações retornam erro).

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rustc-check-cfg=cfg(occt)");
    println!("cargo:rerun-if-env-changed=OCCT_ROOT");
    println!("cargo:rerun-if-changed=cpp/kernel.cpp");
    println!("cargo:rerun-if-changed=cpp/kernel.h");
    println!("cargo:rerun-if-changed=src/ffi.rs");

    let root = std::env::var("OCCT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\dev\vcpkg\installed\x64-windows-release"));
    let include = root.join("include").join("opencascade");
    let lib = root.join("lib");
    if !include.join("TopoDS_Shape.hxx").exists() {
        println!("cargo:warning=OpenCASCADE não encontrado em {} — compilando sem kernel", root.display());
        println!("cargo:rerun-if-changed={}", include.display());
        return;
    }
    println!("cargo:rustc-cfg=occt");

    cxx_build::bridge("src/ffi.rs")
        .file("cpp/kernel.cpp")
        .include(&include)
        .include("cpp")
        .std("c++17")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/utf-8")
        .define("_USE_MATH_DEFINES", None)
        .compile("scc_kernel_cpp");

    // liga todas as bibliotecas TK* (os nomes mudam entre versões do OCCT)
    println!("cargo:rustc-link-search=native={}", lib.display());
    for e in std::fs::read_dir(&lib).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with("TK") && n.ends_with(".lib") {
            println!("cargo:rustc-link-lib=dylib={}", n.trim_end_matches(".lib"));
        }
    }
    copy_dlls(&root.join("bin"));
}

/// Copia as DLLs do OCCT e dependências para a pasta do executável.
fn copy_dlls(bin: &Path) {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // OUT_DIR = target/<perfil>/build/<crate>/out
    let Some(target) = out.ancestors().nth(3) else { return };
    for dir in [target.to_path_buf(), target.join("deps")] {
        for e in std::fs::read_dir(bin).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "dll") {
                let dst = dir.join(p.file_name().unwrap());
                if !dst.exists() {
                    let _ = std::fs::copy(&p, &dst);
                }
            }
        }
    }
}
