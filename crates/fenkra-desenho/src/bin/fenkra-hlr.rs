// SPDX-License-Identifier: LGPL-2.1-or-later
//! Projeta uma vista com linhas ocultas e sai (B393). O Fenkra faz o mesmo com `--fenkra-hlr`;
//! este executável serve aos testes do crate, que não têm o aplicativo.

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some(fenkra_desenho::hlr_processos::ARG) {
        args.insert(0, fenkra_desenho::hlr_processos::ARG.into());
    }
    std::process::exit(fenkra_desenho::hlr_processos::principal(&args).unwrap_or(2));
}
