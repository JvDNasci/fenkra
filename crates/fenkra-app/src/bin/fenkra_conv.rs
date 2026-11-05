// SPDX-License-Identifier: LGPL-2.1-or-later
//! Processo filho do conversor (ADR 0018). Mora neste pacote porque `cargo run -p fenkra-app` só
//! compila os binários do próprio pacote.

fn main() {
    std::process::exit(fenkra_conversor::processo::principal());
}
