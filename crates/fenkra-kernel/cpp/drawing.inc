// Detalhamento 2D: projeção com linhas ocultas (HLR) e cortes para vistas de seção.
// Incluído no fim de kernel.cpp.

namespace fenkra {

namespace {

// Discretiza as arestas da composição em polilinhas 2D (x, y do plano da vista) com o tipo dado.
void hlr_add(const TopoDS_Shape& c, uint8_t kind, rust::Vec<double>& out, rust::Vec<uint32_t>& ends, rust::Vec<uint8_t>& kinds) {
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
