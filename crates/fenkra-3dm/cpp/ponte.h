// SPDX-License-Identifier: LGPL-2.1-or-later
// Ponte do Fenkra com o openNURBS (B211, ADR 0090). Nenhuma exceção sai sem virar std::runtime_error.
#pragma once
#include "rust/cxx.h"

namespace fenkra3dm {

struct ArquivoBruto;

ArquivoBruto ler_3dm(rust::Str caminho);
rust::Vec<rust::String> gravar_3dm(rust::Str caminho, const ArquivoBruto& arquivo);
rust::String versao_opennurbs();

}  // namespace fenkra3dm
