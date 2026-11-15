// SPDX-License-Identifier: LGPL-2.1-or-later
//! Compila o openNURBS (biblioteca e o zlib que vem com ele) e a ponte, se o fonte estiver na máquina.
//!
//! O fonte não fica no repositório (ADR 0090): `scripts/baixar_opennurbs.py` baixa a versão fixada para
//! `third_party/opennurbs`. Procura em `FENKRA_OPENNURBS` e depois em `third_party/opennurbs` desta pasta
//! ou de qualquer pasta acima, para um worktree dentro do repositório achar a cópia da raiz. Sem o fonte,
//! o crate compila sem a ponte e as funções respondem como obter o openNURBS.

use std::path::{Path, PathBuf};

fn achar() -> Option<PathBuf> {
    let ok = |p: &Path| p.join("opennurbs.h").exists() && p.join("CMakeLists.txt").exists();
    if let Ok(p) = std::env::var("FENKRA_OPENNURBS") {
        let p = PathBuf::from(p);
        // apontada e sem o fonte: não procura em outro lugar, para não compilar uma cópia que ninguém escolheu
        return ok(&p).then_some(p);
    }
    let manifesto = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    manifesto.ancestors().map(|a| a.join("third_party").join("opennurbs")).find(|p| ok(p))
}

/// Fontes da biblioteca pública, na lista do próprio CMakeLists.txt (sem os comentados).
fn fontes(raiz: &Path) -> Vec<PathBuf> {
    let texto = std::fs::read_to_string(raiz.join("CMakeLists.txt")).expect("CMakeLists.txt do openNURBS");
    let inicio = texto.find("set( OPENNURBS_PUBLIC_SOURCES").expect("lista OPENNURBS_PUBLIC_SOURCES");
    let resto = &texto[inicio + "set( OPENNURBS_PUBLIC_SOURCES".len()..];
    let fim = resto.find(')').expect("fim da lista de fontes");
    let mut v: Vec<PathBuf> = resto[..fim]
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(".cpp") && !l.starts_with('#'))
        .map(|l| raiz.join(l))
        .collect();
    // a gerência de memória fica fora da lista no CMake, num conjunto à parte
    v.push(raiz.join("opennurbs_public_memory.cpp"));
    v
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(opennurbs)");
    println!("cargo:rerun-if-env-changed=FENKRA_OPENNURBS");
    println!("cargo:rerun-if-changed=cpp/ponte.cpp");
    println!("cargo:rerun-if-changed=cpp/ponte.h");
    println!("cargo:rerun-if-changed=src/ffi.rs");
    let Some(raiz) = achar() else {
        println!("cargo:warning=openNURBS não encontrado: rode scripts/baixar_opennurbs.py para ler e gravar 3DM");
        return;
    };
    println!("cargo:rerun-if-changed={}", raiz.join("FENKRA_VERSAO").display());
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");

    // a ponte primeiro: no Linux a ordem das bibliotecas estáticas conta (ponte → openNURBS → zlib)
    let mut ponte = cxx_build::bridge("src/ffi.rs");
    ponte.file("cpp/ponte.cpp").include(&raiz).include("cpp").std("c++17").warnings(false);
    for (k, v) in [("UNICODE", None), ("_UNICODE", None), ("ON_CMAKE_BUILD", None), ("NOMINMAX", None), ("Z_PREFIX", None), ("MY_ZCALLOC", None)] {
        ponte.define(k, v);
    }
    if msvc {
        ponte.flag("/EHsc").flag("/utf-8").flag("/bigobj");
    }
    ponte.compile("fenkra_3dm_ponte");

    // O openNURBS leva minutos para compilar e o `cc` não guarda nada entre execuções: mudar a ponte
    // reroda este script. Com a mesma versão e as mesmas opções, reaproveita as bibliotecas da vez anterior.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let marca = format!(
        "{}|{}|{}|{}",
        std::fs::read_to_string(raiz.join("FENKRA_VERSAO")).unwrap_or_default().trim(),
        raiz.display(),
        std::env::var("TARGET").unwrap_or_default(),
        std::env::var("OPT_LEVEL").unwrap_or_default()
    );
    let carimbo = out.join("fenkra_opennurbs.carimbo");
    let libs: &[&str] = if windows { &["fenkra_opennurbs", "fenkra_on_zlib"] } else { &["fenkra_opennurbs", "fenkra_on_uuid", "fenkra_on_zlib"] };
    let arquivo = |n: &str| out.join(if msvc { format!("{n}.lib") } else { format!("lib{n}.a") });
    if std::fs::read_to_string(&carimbo).ok().as_deref() == Some(marca.as_str()) && libs.iter().all(|n| arquivo(n).exists()) {
        println!("cargo:rustc-link-search=native={}", out.display());
        for n in libs {
            println!("cargo:rustc-link-lib=static={n}");
        }
    } else {
        compilar_opennurbs(&raiz, windows, msvc);
        let _ = std::fs::write(&carimbo, &marca);
    }

    if windows {
        for l in ["shlwapi", "shell32", "usp10", "rpcrt4", "ole32", "user32", "gdi32", "advapi32"] {
            println!("cargo:rustc-link-lib=dylib={l}");
        }
    }
    println!("cargo:rustc-cfg=opennurbs");
}

fn compilar_opennurbs(raiz: &Path, windows: bool, msvc: bool) {
    let mut on = cc::Build::new();
    on.cpp(true).std("c++17").include(raiz).warnings(false).files(fontes(raiz));
    for k in ["ON_COMPILING_OPENNURBS", "Z_PREFIX", "MY_ZCALLOC", "UNICODE", "_UNICODE", "ON_CMAKE_BUILD"] {
        on.define(k, None);
    }
    if msvc {
        on.flag("/EHsc").flag("/utf-8").flag("/bigobj");
    }
    if !windows {
        // o Windows gera UUID pelo sistema; fora dele, o openNURBS usa o android_uuid que vem junto
        let u = raiz.join("android_uuid");
        on.include(&u);
        let mut uuid = cc::Build::new();
        uuid.include(&u).warnings(false);
        for e in std::fs::read_dir(&u).into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "c") {
                uuid.file(e.path());
            }
        }
        uuid.compile("fenkra_on_uuid");
    }
    on.compile("fenkra_opennurbs");

    let z = raiz.join("zlib");
    let mut zlib = cc::Build::new();
    zlib.include(&z).warnings(false).define("Z_PREFIX", None).define("MY_ZCALLOC", None);
    for f in ["adler32.c", "compress.c", "crc32.c", "deflate.c", "infback.c", "inffast.c", "inflate.c", "inftrees.c", "trees.c", "uncompr.c", "zutil.c"] {
        zlib.file(z.join(f));
    }
    zlib.compile("fenkra_on_zlib");
}
