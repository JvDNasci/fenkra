// SPDX-License-Identifier: LGPL-2.1-or-later
// Detalhamento 2D: projeção com linhas ocultas (HLR) e cortes para vistas de seção.
// Incluído no fim de kernel.cpp.

namespace fenkra {

namespace {

// Discretiza as arestas da composição em polilinhas 2D (x, y do plano da vista) com o tipo dado.
template <class VD, class VU, class VK>
void hlr_add(const TopoDS_Shape& c, uint8_t kind, VD& out, VU& ends, VK& kinds) {
    if (c.IsNull()) return;
    for (TopExp_Explorer e(c, TopAbs_EDGE); e.More(); e.Next()) {
        TopoDS_Edge ed = TopoDS::Edge(e.Current());
        BRepLib::BuildCurves3d(ed);
        BRepAdaptor_Curve cv(ed);
        GCPnts_TangentialDeflection td(cv, 0.08, 0.02, 2);
        if (td.NbPoints() < 2) continue;
        for (int i = 1; i <= td.NbPoints(); ++i) {
            gp_Pnt p = td.Value(i);
            out.push_back(p.X());
            out.push_back(p.Y());
        }
        ends.push_back(static_cast<uint32_t>(out.size() / 2));
        kinds.push_back(kind);
    }
}

// Linhas da projeção por malha: cada aresta é um segmento reto. Segmentos seguidos que se tocam
// viram uma polilinha só, para a folha não receber dezenas de milhares de pedaços.
template <class VD, class VU, class VK>
void hlr_add_poly(const TopoDS_Shape& c, uint8_t kind, VD& out, VU& ends, VK& kinds) {
    if (c.IsNull()) return;
    bool aberta = false;
    double ux = 0, uy = 0;
    for (TopExp_Explorer e(c, TopAbs_EDGE); e.More(); e.Next()) {
        TopoDS_Edge ed = TopoDS::Edge(e.Current());
        TopoDS_Vertex v1, v2;
        TopExp::Vertices(ed, v1, v2, true);
        if (v1.IsNull() || v2.IsNull()) continue;
        gp_Pnt a = BRep_Tool::Pnt(v1), b = BRep_Tool::Pnt(v2);
        bool emenda = aberta && std::abs(a.X() - ux) < 1e-9 && std::abs(a.Y() - uy) < 1e-9;
        if (!emenda) {
            if (aberta) {
                ends.push_back(static_cast<uint32_t>(out.size() / 2));
                kinds.push_back(kind);
            }
            out.push_back(a.X());
            out.push_back(a.Y());
        }
        out.push_back(b.X());
        out.push_back(b.Y());
        ux = b.X();
        uy = b.Y();
        aberta = true;
    }
    if (aberta) {
        ends.push_back(static_cast<uint32_t>(out.size() / 2));
        kinds.push_back(kind);
    }
}

}  // namespace

// Vista: direção para o observador (dx, dy, dz) e eixo x da folha (xx, xy, xz). Tipos: 0 aresta
// visível, 1 tangente visível, 2 contorno (silhueta), 3 oculta.
rust::Vec<double> hlr_project(const Shape& s, double dx, double dy, double dz, double xx, double xy, double xz, bool hidden,
                              rust::Vec<uint32_t>& ends, rust::Vec<uint8_t>& kinds) {
    return guard("projeção da vista", [&] {
        gp_Ax2 ax(gp_Pnt(0, 0, 0), gp_Dir(dx, dy, dz), gp_Dir(xx, xy, xz));
        HLRAlgo_Projector proj(ax);
        Handle(HLRBRep_Algo) algo = new HLRBRep_Algo;
        algo->Add(s.s);
        algo->Projector(proj);
        algo->Update();
        algo->Hide();
        HLRBRep_HLRToShape h(algo);
        rust::Vec<double> out;
        hlr_add(h.VCompound(), 0, out, ends, kinds);
        hlr_add(h.Rg1LineVCompound(), 1, out, ends, kinds);
        hlr_add(h.OutLineVCompound(), 2, out, ends, kinds);
        if (hidden) {
            hlr_add(h.HCompound(), 3, out, ends, kinds);
            hlr_add(h.OutLineHCompound(), 3, out, ends, kinds);
        }
        return out;
    });
}

struct HlrTrabalho::Impl {
    TopoDS_Shape s;
    gp_Ax2 ax;
    bool hidden = false;
    bool poly = false;
    std::vector<double> pts;
    std::vector<uint32_t> ends;
    std::vector<uint8_t> kinds;
    std::string erro;
};

// Prepara uma vista para `hlr_trabalho_rodar` em outra thread (B393). `copiar` faz uma cópia
// profunda da forma: duas vistas não leem a mesma topologia ao mesmo tempo. `poligonal > 0` usa a
// projeção por malha (rápida, para a prévia), com a malha desse desvio feita aqui, antes da cópia.
std::unique_ptr<HlrTrabalho> hlr_trabalho_novo(const Shape& s, double dx, double dy, double dz, double xx, double xy, double xz,
                                               bool hidden, bool copiar, double poligonal) {
    return guard("projeção da vista", [&] {
        auto t = std::make_unique<HlrTrabalho>();
        t->p = std::make_shared<HlrTrabalho::Impl>();
        HlrTrabalho::Impl& x = *t->p;
        x.ax = gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(dx, dy, dz), gp_Dir(xx, xy, xz));
        x.hidden = hidden;
        x.poly = poligonal > 0;
        if (x.poly) BRepMesh_IncrementalMesh(s.s, poligonal, false, 0.5, false);
        x.s = copiar ? BRepBuilderAPI_Copy(s.s, true, x.poly).Shape() : s.s;
        return t;
    });
}

// Roda a vista. Não lança: o erro fica guardado e sai em `hlr_trabalho_resultado`.
void hlr_trabalho_rodar(HlrTrabalho& t) {
    HlrTrabalho::Impl& x = *t.p;
    try {
        HLRAlgo_Projector proj(x.ax);
        if (x.poly) {
            Handle(HLRBRep_PolyAlgo) algo = new HLRBRep_PolyAlgo;
            algo->Load(x.s);
            algo->Projector(proj);
            algo->Update();
            HLRBRep_PolyHLRToShape h;
            h.Update(algo);
            hlr_add_poly(h.VCompound(), 0, x.pts, x.ends, x.kinds);
            hlr_add_poly(h.Rg1LineVCompound(), 1, x.pts, x.ends, x.kinds);
            hlr_add_poly(h.OutLineVCompound(), 2, x.pts, x.ends, x.kinds);
            if (x.hidden) {
                hlr_add_poly(h.HCompound(), 3, x.pts, x.ends, x.kinds);
                hlr_add_poly(h.OutLineHCompound(), 3, x.pts, x.ends, x.kinds);
            }
        } else {
            Handle(HLRBRep_Algo) algo = new HLRBRep_Algo;
            algo->Add(x.s);
            algo->Projector(proj);
            algo->Update();
            algo->Hide();
            HLRBRep_HLRToShape h(algo);
            hlr_add(h.VCompound(), 0, x.pts, x.ends, x.kinds);
            hlr_add(h.Rg1LineVCompound(), 1, x.pts, x.ends, x.kinds);
            hlr_add(h.OutLineVCompound(), 2, x.pts, x.ends, x.kinds);
            if (x.hidden) {
                hlr_add(h.HCompound(), 3, x.pts, x.ends, x.kinds);
                hlr_add(h.OutLineHCompound(), 3, x.pts, x.ends, x.kinds);
            }
        }
    } catch (const Standard_Failure& e) {
        std::string m = e.GetMessageString() ? e.GetMessageString() : "";
        x.erro = "projeção da vista: " + (m.empty() ? std::string("erro geométrico do kernel") : m);
    } catch (const std::exception& e) {
        x.erro = std::string("projeção da vista: ") + e.what();
    } catch (...) {
        x.erro = "projeção da vista: erro desconhecido";
    }
    x.s.Nullify();
}

rust::Vec<double> hlr_trabalho_resultado(const HlrTrabalho& t, rust::Vec<uint32_t>& ends, rust::Vec<uint8_t>& kinds) {
    const HlrTrabalho::Impl& x = *t.p;
    if (!x.erro.empty()) throw std::runtime_error(x.erro);
    rust::Vec<double> out;
    out.reserve(x.pts.size());
    for (double v : x.pts) out.push_back(v);
    for (uint32_t e : x.ends) ends.push_back(e);
    for (uint8_t k : x.kinds) kinds.push_back(k);
    return out;
}

// Forma em binário (BinTools), para outro processo ler (B393: linhas ocultas em processos).
rust::Vec<uint8_t> forma_para_bin(const Shape& s) {
    return guard("gravar forma", [&] {
        std::ostringstream o(std::ios::binary);
        BinTools::Write(s.s, o, false, false, BinTools_FormatVersion_CURRENT);
        std::string t = o.str();
        rust::Vec<uint8_t> out;
        out.reserve(t.size());
        for (char c : t) out.push_back(static_cast<uint8_t>(c));
        return out;
    });
}

std::unique_ptr<Shape> forma_de_bin(rust::Slice<const uint8_t> dados) {
    return guard("ler forma", [&] {
        std::istringstream i(std::string(reinterpret_cast<const char*>(dados.data()), dados.size()), std::ios::binary);
        TopoDS_Shape s;
        BinTools::Read(s, i);
        if (s.IsNull()) throw std::runtime_error("forma binária inválida");
        return wrap(s);
    });
}

// Mesma topologia (o `Shape::clone` do Rust divide a mesma forma).
bool forma_mesma(const Shape& a, const Shape& b) {
    return a.s.IsSame(b.s);
}

// Contornos da seção da forma pelo plano (o, n): polilinhas 3D fechadas (para a hachura).
rust::Vec<double> section_loops(const Shape& s, double ox, double oy, double oz, double nx, double ny, double nz, double half, rust::Vec<uint32_t>& ends) {
    return guard("seção", [&] {
        gp_Pln pl(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz));
        TopoDS_Face f = BRepBuilderAPI_MakeFace(pl, -half, half, -half, half).Face();
        TopoDS_Shape c = BRepAlgoAPI_Common(s.s, f).Shape();
        rust::Vec<double> out;
        uint32_t n = 0;
        for (TopExp_Explorer fe(c, TopAbs_FACE); fe.More(); fe.Next()) {
            for (TopExp_Explorer we(fe.Current(), TopAbs_WIRE); we.More(); we.Next()) {
                bool any = false;
                for (BRepTools_WireExplorer ex(TopoDS::Wire(we.Current())); ex.More(); ex.Next()) {
                    BRepAdaptor_Curve cv(ex.Current());
                    GCPnts_TangentialDeflection td(cv, 0.05, 0.1, 2);
                    int m = td.NbPoints();
                    bool rev = ex.Current().Orientation() == TopAbs_REVERSED;
                    for (int i = 1; i <= m; ++i) {
                        if (any && i == 1) continue;
                        gp_Pnt p = td.Value(rev ? m + 1 - i : i);
                        out.push_back(p.X()), out.push_back(p.Y()), out.push_back(p.Z());
                        ++n;
                    }
                    any = true;
                }
                if (any) ends.push_back(n);
            }
        }
        return out;
    });
}

}  // namespace fenkra
