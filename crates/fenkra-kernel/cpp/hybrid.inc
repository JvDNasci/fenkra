// SPDX-License-Identifier: LGPL-2.1-or-later
// Bancada de Superfícies: superfícies primitivas, varredura/multi-seções abertas, operações
// (juntar, curar, dividir, aparar, contorno, desmontar, desaparar, extrapolar) e híbridos
// (engrossar, fechar, células de volume, malha → B-rep).
// Incluído no fim de kernel.cpp, depois de surfaces.inc (usa face_near, spine_wire, extended_faces).

namespace fenkra {

namespace {

// Ponto mais próximo `q` na forma e normal `n` da face ali.
void near_normal(const TopoDS_Shape& s, const gp_Pnt& p, gp_Pnt& q, gp_Vec& n) {
    TopoDS_Face f = face_near(s, p);
    BRepExtrema_DistShapeShape d(f, BRepBuilderAPI_MakeVertex(p).Vertex());
    if (!d.IsDone() || d.NbSolution() == 0) throw std::runtime_error("não foi possível projetar na superfície");
    q = d.PointOnShape1(1);
    double u = 0, w = 0;
    if (d.SupportTypeShape1(1) == BRepExtrema_IsInFace) d.ParOnFaceS1(1, u, w);
    else {
        GeomAPI_ProjectPointOnSurf pr(q, BRep_Tool::Surface(f));
        if (pr.NbPoints() > 0) pr.LowerDistanceParameters(u, w);
    }
    gp_Pnt pp;
    BRepGProp_Face(f).Normal(u, w, pp, n);
    if (n.Magnitude() < 1e-12) n = gp_Vec(0, 0, 1);
    n.Normalize();
}

gp_Pnt center_of(const TopoDS_Shape& s) {
    GProp_GProps g;
    if (TopExp_Explorer(s, TopAbs_SOLID).More()) BRepGProp::VolumeProperties(s, g);
    else BRepGProp::SurfaceProperties(s, g);
    return g.CentreOfMass();
}

double size_of(const TopoDS_Shape& a, const TopoDS_Shape& b) {
    Bnd_Box bx;
    BRepBndLib::Add(a, bx);
    if (!b.IsNull()) BRepBndLib::Add(b, bx);
    return std::sqrt(bx.SquareExtent()) * 2 + 10;
}

TopoDS_Shape sewn(const TopoDS_Shape& s, double tol) {
    BRepBuilderAPI_Sewing sw(tol);
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) sw.Add(e.Current());
    sw.Perform();
    return sw.SewedShape();
}

// Polilinhas (achatadas) das arestas, encadeadas por fio; `ends` = índice final de cada polilinha.
void wires_to_pts(const TopoDS_Shape& wires, rust::Vec<double>& out, rust::Vec<uint32_t>& ends) {
    uint32_t n = 0;
    for (TopExp_Explorer we(wires, TopAbs_WIRE); we.More(); we.Next()) {
        bool any = false;
        for (BRepTools_WireExplorer ex(TopoDS::Wire(we.Current())); ex.More(); ex.Next()) {
            BRepAdaptor_Curve c(ex.Current());
            GCPnts_TangentialDeflection td(c, 0.05, 0.1, 2);
            int m = td.NbPoints();
            bool rev = ex.Current().Orientation() == TopAbs_REVERSED;
            for (int i = 1; i <= m; ++i) {
                int k = rev ? m + 1 - i : i;
                if (any && i == 1) continue;  // ponto compartilhado com a aresta anterior
                gp_Pnt p = td.Value(k);
                out.push_back(p.X()), out.push_back(p.Y()), out.push_back(p.Z());
                ++n;
            }
            any = true;
        }
        if (any) ends.push_back(n);
    }
}

// Bordas livres como fios: arestas usadas por uma face só (a costura de faces periódicas,
// como a do cilindro, aparece duas vezes na mesma face e não conta).
TopoDS_Compound free_wires(const TopoDS_Shape& s) {
    TopTools_IndexedDataMapOfShapeListOfShape m;
    TopExp::MapShapesAndAncestors(s, TopAbs_EDGE, TopAbs_FACE, m);
    Handle(TopTools_HSequenceOfShape) seq = new TopTools_HSequenceOfShape, wires;
    for (int i = 1; i <= m.Extent(); ++i) {
        const TopoDS_Edge& e = TopoDS::Edge(m.FindKey(i));
        if (BRep_Tool::Degenerated(e)) continue;
        const TopTools_ListOfShape& fs = m.FindFromIndex(i);
        if (fs.Extent() != 1 || BRep_Tool::IsClosed(e, TopoDS::Face(fs.First()))) continue;
        seq->Append(e);
    }
    TopoDS_Compound c;
    BRep_Builder bb;
    bb.MakeCompound(c);
    if (seq->Length() > 0) {
        ShapeAnalysis_FreeBounds::ConnectEdgesToWires(seq, 1e-4, Standard_False, wires);
        for (int i = 1; i <= wires->Length(); ++i) bb.Add(c, wires->Value(i));
    }
    return c;
}

}  // namespace

std::unique_ptr<Shape> sphere_surface(double cx, double cy, double cz, double r) {
    return guard("esfera", [&] {
        if (r <= 0) throw std::runtime_error("raio precisa ser positivo");
        TopoDS_Shape s = BRepPrimAPI_MakeSphere(gp_Pnt(cx, cy, cz), r).Shape();
        TopExp_Explorer e(s, TopAbs_SHELL);
        return wrap(e.More() ? e.Current() : s);
    });
}

std::unique_ptr<Shape> cylinder_surface(double ox, double oy, double oz, double dx, double dy, double dz, double r, double h0, double h1) {
    return guard("cilindro", [&] {
        if (r <= 0) throw std::runtime_error("raio precisa ser positivo");
        if (h1 - h0 < 1e-9) throw std::runtime_error("o comprimento precisa ser positivo");
        Handle(Geom_CylindricalSurface) cs = new Geom_CylindricalSurface(gp_Ax3(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz)), r);
        return wrap(BRepBuilderAPI_MakeFace(cs, 0, 2 * M_PI, h0, h1, Precision::Confusion()).Face());
    });
}

namespace {

// Uma tentativa de varredura aberta. `discreto` troca o triedro pelo discreto (guia suave e longa, em
// que o Frenet corrigido salta entre vizinhos). Falha de qualquer tipo só devolve false com o motivo.
bool varre_superficie(const TopoDS_Wire& w, const TopoDS_Wire& p, uint8_t mode, bool discreto, TopoDS_Shape& out, const char*& motivo) {
    bool ok = false;
    try {
        protegido("", [&] {
            BRepOffsetAPI_MakePipeShell mk(w);
            if (discreto && mode != 2) mk.SetDiscreteMode();
            else if (mode == 1) mk.SetMode(gp_Dir(0, 0, 1));
            else mk.SetMode(mode == 2 ? Standard_True : Standard_False);  // 2 = Frenet; 0 = Frenet corrigido
            mk.Add(p, Standard_False, Standard_False);
            mk.Build();
            if (!mk.IsDone() || mk.Shape().IsNull()) {
                motivo = "não foi possível varrer o perfil ao longo da guia";
                return;
            }
            // `IsDone` não garante o resultado: a aproximação pode sair longe da varredura exata
            if (mk.ErrorOnSurface() > kErroVarredura || !BRepCheck_Analyzer(mk.Shape()).IsValid()) {
                motivo = "varredura de superfície: o resultado saiu inválido; simplifique a guia ou divida em trechos";
                return;
            }
            out = mk.Shape();
            ok = true;
        });
    } catch (const std::runtime_error&) {
        if (!motivo) motivo = "não foi possível varrer o perfil ao longo da guia";
    } catch (const Standard_Failure&) {
        if (!motivo) motivo = "não foi possível varrer o perfil ao longo da guia";
    }
    return ok;
}

}  // namespace

// Varredura aberta: perfil (curva) ao longo da guia, sem fechar em sólido. Mesma escada da varredura
// sólida (K-08): o caminho de sempre primeiro; guia suave que não varre com o Frenet corrigido é refeita
// com o triedro discreto, inteira e depois dividida em trechos de uma mesma B-spline (sem criar canto).
std::unique_ptr<Shape> sweep_surface(rust::Slice<const double> profile, rust::Slice<const double> spine, uint8_t mode) {
    return guard("varredura de superfície", [&] {
        TopoDS_Wire p = spine_wire(profile), w = spine_wire(spine);
        TopoDS_Shape out;
        const char* motivo = nullptr;
        if (varre_superficie(w, p, mode, false, out, motivo)) return wrap(out);
        Handle(Geom_BSplineCurve) c = guia_suave(spine);
        if (!c.IsNull()) {
            if (varre_superficie(w, p, mode, true, out, motivo)) return wrap(out);
            for (int k : {4, 8, 16}) {
                falhar_se_cancelado();
                if (varre_superficie(guia_dividida(c, k), p, mode, true, out, motivo)) return wrap(out);
            }
        }
        // Frenet puro não tem direção onde a guia perde a curvatura (inflexão ou trecho reto): a escolha da
        // pessoa fica respeitada, e o erro diz o que fazer
        if (mode == 2) throw std::runtime_error("não foi possível varrer o perfil com o triedro de Frenet: a guia tem inflexão ou trecho quase reto, onde o Frenet gira sem controle; use o modo Corrigido");
        throw std::runtime_error(motivo ? motivo : "não foi possível varrer o perfil ao longo da guia");
    });
}

// Multi-seções de superfície: com guia, as seções percorrem a guia (pipe shell); sem guia, ThruSections.
std::unique_ptr<Shape> loft_surface(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends, bool ruled, bool closed, rust::Slice<const double> guide) {
    return guard("multi-seções de superfície", [&] {
        std::vector<TopoDS_Wire> ws = curves_of(flat, ends);
        if (ws.size() < 2) throw std::runtime_error("são precisas pelo menos duas seções");
        if (guide.size() >= 6) {
            BRepOffsetAPI_MakePipeShell mk(spine_wire(guide));
            mk.SetMode(Standard_False);
            for (auto& w : ws) mk.Add(w, Standard_False, Standard_False);
            mk.Build();
            if (!mk.IsDone()) throw std::runtime_error("as seções não puderam seguir a guia (seções devem cruzar a guia, em ordem)");
            return wrap(mk.Shape());
        }
        BRepOffsetAPI_ThruSections ts(Standard_False, ruled ? Standard_True : Standard_False);
        for (auto& w : ws) ts.AddWire(w);
        if (closed) ts.AddWire(ws[0]);
        ts.CheckCompatibility(Standard_True);
        ts.Build();
        if (!ts.IsDone()) throw std::runtime_error("as seções não puderam ser ligadas (confira a ordem e o sentido)");
        return wrap(ts.Shape());
    });
}

namespace {

// Cúbica pelos pontos, em passos iguais de parâmetro.
Handle(Geom_BSplineCurve) cubica_pelos_pontos(const std::vector<gp_Pnt>& p) {
    const int n = int(p.size());
    occ::handle<NCollection_HArray1<gp_Pnt>> pts = new NCollection_HArray1<gp_Pnt>(1, n);
    occ::handle<NCollection_HArray1<double>> par = new NCollection_HArray1<double>(1, n);
    for (int i = 0; i < n; ++i) {
        pts->SetValue(i + 1, p[size_t(i)]);
        par->SetValue(i + 1, double(i) / (n - 1));
    }
    GeomAPI_Interpolate g(pts, par, false, 1e-9);
    g.Perform();
    if (!g.IsDone()) throw std::runtime_error("pontos coincidentes na grade da superfície");
    return g.Curve();
}

}  // namespace

// Superfície B-spline cúbica que passa por toda a grade, por produto tensorial: interpola cada linha e
// depois cada coluna de polos, todas com os mesmos parâmetros e portanto os mesmos nós. A interpolação
// de grade pronta do kernel ondula a borda perto dos cantos, mesmo com os pontos todos no mesmo plano.
std::unique_ptr<Shape> grid_surface(rust::Slice<const double> flat, uint32_t nu, uint32_t nv) {
    return guard("superfície por grade", [&] {
        if (nu < 4 || nv < 4 || flat.size() != size_t(nu) * nv * 3) throw std::runtime_error("grade de pontos incompleta");
        std::vector<Handle(Geom_BSplineCurve)> linhas;
        for (uint32_t i = 0; i < nu; ++i) {
            std::vector<gp_Pnt> p;
            for (uint32_t j = 0; j < nv; ++j) {
                const double* q = flat.data() + 3 * (size_t(i) * nv + j);
                p.emplace_back(q[0], q[1], q[2]);
            }
            linhas.push_back(cubica_pelos_pontos(p));
        }
        const int np = linhas[0]->NbPoles();
        std::vector<Handle(Geom_BSplineCurve)> colunas;
        for (int j = 1; j <= np; ++j) {
            std::vector<gp_Pnt> p;
            for (auto& l : linhas) p.push_back(l->Pole(j));
            colunas.push_back(cubica_pelos_pontos(p));
        }
        const int mp = colunas[0]->NbPoles();
        NCollection_Array2<gp_Pnt> polos(1, mp, 1, np);
        for (int j = 1; j <= np; ++j)
            for (int i = 1; i <= mp; ++i) polos(i, j) = colunas[size_t(j - 1)]->Pole(i);
        Handle(Geom_BSplineSurface) s = new Geom_BSplineSurface(polos, colunas[0]->Knots(), linhas[0]->Knots(), colunas[0]->Multiplicities(), linhas[0]->Multiplicities(), colunas[0]->Degree(), linhas[0]->Degree());
        return wrap(BRepBuilderAPI_MakeFace(s, 1e-6).Face());
    });
}

namespace {

// Um lado da junção no ponto `p`: normal, direção que entra na face e curvatura nessa direção.
struct LadoDaJuncao {
    gp_Vec normal, entra;
    double curvatura = 0;
};

LadoDaJuncao lado_da_juncao(const TopoDS_Face& f, const gp_Pnt& p, const gp_Vec& tangente) {
    Handle(Geom_Surface) gs = BRep_Tool::Surface(f);
    GeomAPI_ProjectPointOnSurf pr(p, gs);
    if (pr.NbPoints() == 0) throw std::runtime_error("não foi possível projetar a borda na superfície");
    double u, v;
    pr.LowerDistanceParameters(u, v);
    GeomLProp_SLProps sl(gs, u, v, 2, 1e-7);
    LadoDaJuncao r;
    r.normal = sl.IsNormalDefined() ? gp_Vec(sl.Normal()) : gp_Vec(0, 0, 1);
    r.entra = r.normal.Crossed(tangente);
    if (r.entra.Magnitude() < 1e-12) throw std::runtime_error("a borda é perpendicular à superfície");
    r.entra.Normalize();
    if (r.entra.Dot(gp_Vec(p, center_of(f))) < 0) r.entra.Reverse();
    if (sl.IsCurvatureDefined()) {
        if (sl.IsUmbilic()) r.curvatura = sl.MaxCurvature();
        else {
            // Euler: a curvatura numa direção vem das principais
            gp_Dir d1, d2;
            sl.CurvatureDirections(d1, d2);
            const double c = r.entra.Dot(gp_Vec(d1)), s = r.entra.Dot(gp_Vec(d2));
            r.curvatura = sl.MaxCurvature() * c * c + sl.MinCurvature() * s * s;
        }
    }
    return r;
}

}  // namespace

// Junção entre as bordas livres de `a` e a superfície `b`. O ângulo vem das direções que entram em cada
// face, e não das normais: as duas superfícies podem ter normais opostas sem haver dobra nenhuma.
rust::Vec<double> connect_check(const Shape& a, const Shape& b, uint32_t samples, double max_gap) {
    return guard("análise de conexão", [&] {
        rust::Vec<double> out;
        TopTools_IndexedDataMapOfShapeListOfShape e2f;
        TopExp::MapShapesAndAncestors(a.s, TopAbs_EDGE, TopAbs_FACE, e2f);
        std::vector<TopoDS_Face> faces_b;
        for (TopExp_Explorer x(b.s, TopAbs_FACE); x.More(); x.Next()) faces_b.push_back(TopoDS::Face(x.Current()));
        const int n = int(std::max(samples, 2u));
        for (int i = 1; i <= e2f.Extent(); ++i) {
            const TopoDS_Edge& e = TopoDS::Edge(e2f.FindKey(i));
            if (e2f(i).Extent() != 1 || BRep_Tool::Degenerated(e)) continue;
            BRepExtrema_DistShapeShape perto(e, b.s);
            if (!perto.IsDone() || perto.Value() > max_gap) continue;
            const TopoDS_Face fa = TopoDS::Face(e2f(i).First());
            BRepAdaptor_Curve c(e);
            for (int k = 0; k < n; ++k) {
                const double t = c.FirstParameter() + (c.LastParameter() - c.FirstParameter()) * (k + 0.5) / n;
                gp_Pnt p;
                gp_Vec tan;
                c.D1(t, p, tan);
                if (tan.Magnitude() < 1e-12) continue;
                const TopoDS_Vertex vx = BRepBuilderAPI_MakeVertex(p).Vertex();
                double gap = 1e300;
                gp_Pnt q;
                TopoDS_Face fb;
                for (const TopoDS_Face& f : faces_b) {
                    BRepExtrema_DistShapeShape d(f, vx);
                    if (d.IsDone() && d.NbSolution() > 0 && d.Value() < gap) gap = d.Value(), q = d.PointOnShape1(1), fb = f;
                }
                if (fb.IsNull() || gap > max_gap) continue;
                const LadoDaJuncao la = lado_da_juncao(fa, p, tan), lb = lado_da_juncao(fb, q, tan);
                const double ang = std::acos(std::clamp(-la.entra.Dot(lb.entra), -1.0, 1.0)) * 180.0 / M_PI;
                const double sinal = la.normal.Dot(lb.normal) < 0 ? -1 : 1;
                for (double x : {p.X(), p.Y(), p.Z(), gap, ang, la.curvatura, lb.curvatura * sinal}) out.push_back(x);
            }
        }
        return out;
    });
}

rust::Vec<double> signed_distances(const Shape& s, rust::Slice<const double> flat) {
    return guard("mapa de desvio", [&] {
        rust::Vec<double> out;
        for (size_t i = 0; i + 2 < flat.size(); i += 3) {
            const gp_Pnt p(flat[i], flat[i + 1], flat[i + 2]);
            gp_Pnt q;
            gp_Vec n;
            near_normal(s.s, p, q, n);
            const double d = p.Distance(q);
            out.push_back(gp_Vec(q, p).Dot(n) < 0 ? -d : d);
        }
        return out;
    });
}

// Linhas de u e v constantes de cada face, cortadas onde saem da parte aparada.
rust::Vec<double> iso_curves(const Shape& s, uint32_t n, rust::Vec<uint32_t>& ends) {
    return guard("isocurvas", [&] {
        rust::Vec<double> out;
        const int passos = 40;
        for (TopExp_Explorer x(s.s, TopAbs_FACE); x.More(); x.Next()) {
            const TopoDS_Face f = TopoDS::Face(x.Current());
            double u0, u1, v0, v1;
            BRepTools::UVBounds(f, u0, u1, v0, v1);
            BRepAdaptor_Surface su(f);
            BRepTopAdaptor_FClass2d dentro(f, 1e-7);
            std::vector<gp_Pnt> linha;
            auto fecha = [&]() {
                // ponto solto não é linha
                if (linha.size() > 1) {
                    for (const gp_Pnt& p : linha)
                        for (double c : {p.X(), p.Y(), p.Z()}) out.push_back(c);
                    ends.push_back(uint32_t(out.size() / 3));
                }
                linha.clear();
            };
            for (int dir = 0; dir < 2; ++dir)
                for (uint32_t i = 1; i <= n; ++i) {
                    const double fixo = double(i) / (n + 1);
                    for (int k = 0; k <= passos; ++k) {
                        const double corre = double(k) / passos;
                        const double u = u0 + (u1 - u0) * (dir == 0 ? fixo : corre), v = v0 + (v1 - v0) * (dir == 0 ? corre : fixo);
                        if (dentro.Perform(gp_Pnt2d(u, v)) == TopAbs_OUT) {
                            fecha();
                            continue;
                        }
                        linha.push_back(su.Value(u, v));
                    }
                    fecha();
                }
        }
        return out;
    });
}

std::unique_ptr<Shape> sew_shapes(const Shape& s, double tol) {
    return guard("juntar", [&] { return wrap(sewn(s.s, std::max(tol, 1e-6))); });
}

// Curar: costura com tolerância maior, corrige a topologia e une faces do mesmo suporte.
std::unique_ptr<Shape> heal_shape(const Shape& s, double tol) {
    return guard("curar", [&] {
        TopoDS_Shape r = sewn(s.s, std::max(tol, 1e-6));
        Handle(ShapeFix_Shape) fx = new ShapeFix_Shape(r);
        fx->SetPrecision(tol);
        fx->SetMaxTolerance(tol * 10);
        fx->Perform();
        ShapeUpgrade_UnifySameDomain u(fx->Shape(), Standard_True, Standard_True, Standard_True);
        u.Build();
        return wrap(u.Shape());
    });
}

// Divide `s` pela ferramenta (estendida se `extend`) e fica com os pedaços do lado da normal
// (`keep_pos`) ou do outro lado. Sólidos ficam sólidos; superfícies voltam costuradas.
std::unique_ptr<Shape> split_side(const Shape& s, const Shape& tool, bool keep_pos, bool extend) {
    return guard("dividir", [&] {
        double L = size_of(s.s, tool.s);
        TopoDS_Shape cut = extend ? extended_faces(tool.s, L) : tool.s;
        BRepAlgoAPI_Splitter sp;
        TopTools_ListOfShape args, tools;
        args.Append(s.s);
        tools.Append(cut);
        sp.SetArguments(args);
        sp.SetTools(tools);
        sp.Build();
        if (!sp.IsDone()) throw std::runtime_error("não foi possível dividir");
        bool solids = TopExp_Explorer(s.s, TopAbs_SOLID).More();
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        int n = 0, kept = 0;
        for (TopExp_Explorer e(sp.Shape(), solids ? TopAbs_SOLID : TopAbs_FACE); e.More(); e.Next(), ++n) {
            gp_Pnt m = center_of(e.Current()), q;
            gp_Vec nn;
            near_normal(cut, m, q, nn);
            bool pos = gp_Vec(q, m).Dot(nn) > 0;
            if (pos == keep_pos) bb.Add(c, e.Current()), ++kept;
        }
        if (n < 2) throw std::runtime_error("o elemento de corte não atravessa (nada a dividir)");
        if (kept == 0) throw std::runtime_error("nada sobra desse lado — inverta o lado");
        return wrap(solids ? TopoDS_Shape(c) : sewn(c, 1e-4));
    });
}

// Formas de verdade dentro de compostos aninhados. A ferramenta vai ao corte uma a uma: num composto
// só, duas ferramentas que se cruzam contam como uma forma que se corta, e a operação exige o contrário.
static void leaves(const TopoDS_Shape& s, TopTools_ListOfShape& out) {
    if (s.ShapeType() != TopAbs_COMPOUND) {
        out.Append(s);
        return;
    }
    for (TopoDS_Iterator it(s); it.More(); it.Next()) leaves(it.Value(), out);
}

// Pedaços de `s` depois do corte. O corte devolve faces, e uma folha dobrada já tem duas antes de ser
// cortada: o que separa um pedaço do outro é a aresta sobre a ferramenta. Faces ligadas por qualquer
// outra aresta continuam juntas. Devolve um composto com um filho por pedaço.
std::unique_ptr<Shape> split_regions(const Shape& s, const Shape& tool, bool extend) {
    return guard("regiões", [&] {
        double L = size_of(s.s, tool.s);
        TopTools_ListOfShape args, tools, given;
        leaves(tool.s, given);
        for (TopTools_ListIteratorOfListOfShape it(given); it.More(); it.Next())
            leaves(extend ? extended_faces(it.Value(), L) : it.Value(), tools);
        args.Append(s.s);
        BRepAlgoAPI_Splitter sp;
        sp.SetArguments(args);
        sp.SetTools(tools);
        sp.Build();
        if (!sp.IsDone()) throw std::runtime_error("não foi possível dividir");
        TopTools_IndexedMapOfShape faces;
        TopExp::MapShapes(sp.Shape(), TopAbs_FACE, faces);
        TopTools_IndexedDataMapOfShapeListOfShape vizinhas;
        TopExp::MapShapesAndAncestors(sp.Shape(), TopAbs_EDGE, TopAbs_FACE, vizinhas);
        std::vector<int> pai(faces.Extent() + 1);
        for (int i = 0; i <= faces.Extent(); ++i) pai[i] = i;
        std::function<int(int)> raiz = [&](int i) { return pai[i] == i ? i : pai[i] = raiz(pai[i]); };
        auto no_corte = [&](const TopoDS_Edge& e) {
            BRepAdaptor_Curve c(e);
            gp_Pnt m = c.Value((c.FirstParameter() + c.LastParameter()) / 2);
            TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(m).Vertex();
            for (TopTools_ListIteratorOfListOfShape it(tools); it.More(); it.Next()) {
                BRepExtrema_DistShapeShape d(it.Value(), v);
                if (d.IsDone() && d.Value() < 1e-4) return true;
            }
            return false;
        };
        for (int i = 1; i <= vizinhas.Extent(); ++i) {
            const TopTools_ListOfShape& l = vizinhas.FindFromIndex(i);
            const TopoDS_Edge& e = TopoDS::Edge(vizinhas.FindKey(i));
            if (l.Extent() < 2 || BRep_Tool::Degenerated(e) || no_corte(e)) continue;
            int primeira = faces.FindIndex(l.First());
            for (TopTools_ListIteratorOfListOfShape it(l); it.More(); it.Next()) pai[raiz(faces.FindIndex(it.Value()))] = raiz(primeira);
        }
        BRep_Builder bb;
        TopoDS_Compound out;
        bb.MakeCompound(out);
        for (int r = 1; r <= faces.Extent(); ++r) {
            if (raiz(r) != r) continue;
            TopoDS_Compound g;
            bb.MakeCompound(g);
            int n = 0;
            for (int i = 1; i <= faces.Extent(); ++i)
                if (raiz(i) == r) bb.Add(g, faces(i)), ++n;
            bb.Add(out, n == 1 ? TopoDS_Shape(TopoDS_Iterator(g).Value()) : sewn(g, 1e-4));
        }
        return wrap(out);
    });
}

uint32_t child_count(const Shape& s) {
    uint32_t n = 0;
    for (TopoDS_Iterator it(s.s); it.More(); it.Next()) ++n;
    return n;
}

std::unique_ptr<Shape> nth_child(const Shape& s, uint32_t i) {
    return guard("desmontar", [&] {
        uint32_t k = 0;
        for (TopoDS_Iterator it(s.s); it.More(); it.Next(), ++k)
            if (k == i) return wrap(it.Value());
        throw std::runtime_error("a forma tem menos partes");
    });
}

std::unique_ptr<Shape> nth_face(const Shape& s, uint32_t i) {
    return guard("desmontar", [&] {
        uint32_t k = 0;
        for (TopExp_Explorer e(s.s, TopAbs_FACE); e.More(); e.Next(), ++k)
            if (k == i) return wrap(e.Current());
        throw std::runtime_error("a superfície tem menos faces");
    });
}

// Desaparar: as faces voltam aos limites naturais do suporte (as infinitas, limitadas ao tamanho da peça).
std::unique_ptr<Shape> untrim_face(const Shape& s) {
    return guard("desaparar", [&] {
        Bnd_Box bx;
        BRepBndLib::Add(s.s, bx);
        return wrap(sewn(extended_faces(s.s, std::sqrt(bx.SquareExtent())), 1e-4));
    });
}

// Extrapolar: prolonga a face pelo lado (borda u/v) mais próximo do ponto, com continuidade de tangência.
std::unique_ptr<Shape> extrapolate_face(const Shape& s, double px, double py, double pz, double len, int32_t cont) {
    return guard("extrapolar", [&] {
        if (std::abs(len) < 1e-9) throw std::runtime_error("o comprimento não pode ser zero");
        gp_Pnt p(px, py, pz);
        TopoDS_Face f = face_near(s.s, p);
        double u1, u2, v1, v2;
        BRepTools::UVBounds(f, u1, u2, v1, v2);
        Handle(Geom_Surface) g = BRep_Tool::Surface(f);
        Handle(Geom_BoundedSurface) tr = new Geom_RectangularTrimmedSurface(g, u1, u2, v1, v2);
        Handle(Geom_BSplineSurface) bs = GeomConvert::SurfaceToBSplineSurface(tr);
        double um = (u1 + u2) / 2, vm = (v1 + v2) / 2;
        struct Side { double u, v; bool inU, after; };
        Side sides[4] = {{u1, vm, true, false}, {u2, vm, true, true}, {um, v1, false, false}, {um, v2, false, true}};
        int best = 0;
        double bd = 1e300;
        for (int i = 0; i < 4; ++i) {
            double d = g->Value(sides[i].u, sides[i].v).Distance(p);
            if (d < bd) bd = d, best = i;
        }
        Handle(Geom_BoundedSurface) ext = bs;
        TopoDS_Face made;
        if (len > 0) {
            GeomLib::ExtendSurfByLength(ext, len, std::clamp(cont, 1, 2), sides[best].inU, sides[best].after);
            BRepBuilderAPI_MakeFace mf(Handle(Geom_Surface)::DownCast(ext), Precision::Confusion());
            if (!mf.IsDone()) throw std::runtime_error("não foi possível extrapolar");
            made = mf.Face();
        } else {
            // distância negativa apara: anda o comprimento pedido sobre a isolinha do meio, a partir
            // da borda, e fecha o domínio nesse parâmetro
            const Side& sd = sides[best];
            Handle(Geom_Curve) iso = sd.inU ? g->VIso(vm) : g->UIso(um);
            GeomAdaptor_Curve ac(iso);
            const double from = sd.inU ? sd.u : sd.v, sign = sd.after ? -1.0 : 1.0;
            GCPnts_AbscissaPoint ap(ac, sign * std::abs(len), from);
            if (!ap.IsDone()) throw std::runtime_error("a distância de aparo é maior que a superfície");
            const double t = ap.Parameter();
            double a1 = u1, a2 = u2, b1 = v1, b2 = v2;
            (sd.inU ? (sd.after ? a2 : a1) : (sd.after ? b2 : b1)) = t;
            if (a2 - a1 < 1e-9 || b2 - b1 < 1e-9) throw std::runtime_error("a distância de aparo é maior que a superfície");
            BRepBuilderAPI_MakeFace mf(g, a1, a2, b1, b2, Precision::Confusion());
            if (!mf.IsDone()) throw std::runtime_error("não foi possível aparar a borda");
            made = mf.Face();
        }
        // as outras faces da superfície continuam como estavam
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        int others = 0;
        for (TopExp_Explorer x(s.s, TopAbs_FACE); x.More(); x.Next())
            if (!x.Current().IsSame(f)) {
                bb.Add(c, x.Current());
                ++others;
            }
        if (others == 0) return wrap(made);
        bb.Add(c, made);
        return wrap(c);
    });
}

// Contorno: arestas livres (bordas) da superfície como polilinhas.
rust::Vec<double> boundary_pts(const Shape& s, rust::Vec<uint32_t>& ends) {
    return guard("contorno", [&] {
        TopoDS_Compound c = free_wires(s.s);
        rust::Vec<double> out;
        wires_to_pts(c, out, ends);
        if (ends.empty()) throw std::runtime_error("a superfície não tem bordas livres (é fechada)");
        return out;
    });
}

// Bordas livres aresta por aresta: é a aresta, e não o fio inteiro, que a propagação escolhe.
rust::Vec<double> free_edge_pts(const Shape& s, rust::Vec<uint32_t>& ends) {
    return guard("contorno", [&] {
        rust::Vec<double> out;
        uint32_t n = 0;
        TopoDS_Compound c = free_wires(s.s);
        for (TopExp_Explorer x(c, TopAbs_EDGE); x.More(); x.Next()) {
            BRepAdaptor_Curve bc(TopoDS::Edge(x.Current()));
            GCPnts_TangentialDeflection td(bc, 0.05, 0.1, 2);
            for (int i = 1; i <= td.NbPoints(); ++i) {
                gp_Pnt p = td.Value(i);
                for (double v : {p.X(), p.Y(), p.Z()}) out.push_back(v);
                ++n;
            }
            ends.push_back(n);
        }
        if (ends.empty()) throw std::runtime_error("a superfície não tem bordas livres (é fechada)");
        return out;
    });
}

// Corta a superfície por outra (estendida se `extend`) e fica com as faces que passam mais perto do
// ponto: é o término "até elemento" das superfícies, em que quem decide o lado é o ponto de partida.
std::unique_ptr<Shape> clip_surface(const Shape& s, const Shape& cutter, double px, double py, double pz, bool extend) {
    return guard("até elemento", [&] {
        const double L = size_of(s.s, cutter.s);
        TopoDS_Shape cut = extend ? extended_faces(cutter.s, L) : cutter.s;
        BRepAlgoAPI_Splitter sp;
        TopTools_ListOfShape args, tools;
        args.Append(s.s);
        tools.Append(cut);
        sp.SetArguments(args);
        sp.SetTools(tools);
        sp.Build();
        if (!sp.IsDone()) throw std::runtime_error("não foi possível cortar pelo elemento");
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz));
        std::vector<std::pair<double, TopoDS_Shape>> parts;
        for (TopExp_Explorer e(sp.Shape(), TopAbs_FACE); e.More(); e.Next()) {
            BRepExtrema_DistShapeShape d(e.Current(), v);
            parts.push_back({d.IsDone() ? d.Value() : 1e30, e.Current()});
        }
        if (parts.size() < 2) throw std::runtime_error("o elemento limite não atravessa a superfície");
        double best = 1e30;
        for (auto& p : parts) best = std::min(best, p.first);
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        int n = 0;
        TopoDS_Shape one;
        for (auto& p : parts)
            if (p.first < best + 1e-4) {
                bb.Add(c, p.second);
                one = p.second;
                ++n;
            }
        return wrap(n == 1 ? one : TopoDS_Shape(c));
    });
}

// Polilinha da aresta da forma mais perto do ponto.
rust::Vec<double> edge_pts_at(const Shape& s, double px, double py, double pz) {
    return guard("aresta", [&] {
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz));
        TopoDS_Edge best;
        double bd = 1e300;
        for (TopExp_Explorer x(s.s, TopAbs_EDGE); x.More(); x.Next()) {
            if (BRep_Tool::Degenerated(TopoDS::Edge(x.Current()))) continue;
            BRepExtrema_DistShapeShape d(x.Current(), v);
            if (d.IsDone() && d.Value() < bd) bd = d.Value(), best = TopoDS::Edge(x.Current());
        }
        if (best.IsNull()) throw std::runtime_error("a forma não tem arestas");
        BRepAdaptor_Curve c(best);
        GCPnts_TangentialDeflection td(c, 0.05, 0.05, 2);
        rust::Vec<double> out;
        for (int i = 1; i <= td.NbPoints(); ++i) {
            gp_Pnt p = td.Value(i);
            for (double w : {p.X(), p.Y(), p.Z()}) out.push_back(w);
        }
        return out;
    });
}

// Pontos em que a aresta mais perto de p atravessa o plano, calculados na curva da aresta.
rust::Vec<double> edge_plane_pts(const Shape& s, double px, double py, double pz, double ox, double oy, double oz, double nx, double ny, double nz) {
    return guard("aresta", [&] {
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz));
        TopoDS_Edge best;
        double bd = 1e300;
        for (TopExp_Explorer x(s.s, TopAbs_EDGE); x.More(); x.Next()) {
            if (BRep_Tool::Degenerated(TopoDS::Edge(x.Current()))) continue;
            BRepExtrema_DistShapeShape d(x.Current(), v);
            if (d.IsDone() && d.Value() < bd) bd = d.Value(), best = TopoDS::Edge(x.Current());
        }
        if (best.IsNull()) throw std::runtime_error("a forma não tem arestas");
        double f, l;
        Handle(Geom_Curve) c = BRep_Tool::Curve(best, f, l);
        if (c.IsNull()) throw std::runtime_error("a aresta não tem curva 3D");
        const TopLoc_Location loc = best.Location();
        if (!loc.IsIdentity()) c = Handle(Geom_Curve)::DownCast(c->Transformed(loc.Transformation()));
        Handle(Geom_Plane) pl = new Geom_Plane(gp_Pln(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz)));
        GeomAPI_IntCS it(new Geom_TrimmedCurve(c, f, l), pl);
        if (!it.IsDone()) throw std::runtime_error("interseção da aresta com o plano falhou");
        rust::Vec<double> out;
        for (int i = 1; i <= it.NbPoints(); ++i) {
            gp_Pnt p = it.Point(i);
            for (double w : {p.X(), p.Y(), p.Z()}) out.push_back(w);
        }
        return out;
    });
}

// A aresta é lisa entre as duas faces? Normais comparadas em três pontos, com 1 grau de folga.
static bool smooth_across(const TopoDS_Edge& e, const TopoDS_Face& f, const TopoDS_Face& g) {
    double a1, b1, a2, b2;
    Handle(Geom2d_Curve) c1 = BRep_Tool::CurveOnSurface(e, f, a1, b1), c2 = BRep_Tool::CurveOnSurface(e, g, a2, b2);
    if (c1.IsNull() || c2.IsNull()) return false;
    for (double t : {0.25, 0.5, 0.75}) {
        gp_Pnt2d p = c1->Value(a1 + (b1 - a1) * t), q = c2->Value(a2 + (b2 - a2) * t);
        gp_Pnt x, y;
        gp_Vec n, m;
        BRepGProp_Face(f).Normal(p.X(), p.Y(), x, n);
        BRepGProp_Face(g).Normal(q.X(), q.Y(), y, m);
        if (n.Magnitude() < 1e-12 || m.Magnitude() < 1e-12 || n.Angle(m) > 1.0 * M_PI / 180.0) return false;
    }
    return true;
}

// Extração com propagação, refeita a cada recálculo a partir da face clicada. prop: 0 nenhuma,
// 1 ponto (faces que tocam por vértice), 2 tangente (faces lisas entre si), 3 completo (a forma
// inteira). `complement` devolve o resto. `counts` = [faces, arestas].
std::unique_ptr<Shape> face_propagate(const Shape& s, double px, double py, double pz, int32_t prop, bool complement, rust::Vec<uint32_t>& counts) {
    return guard("extrair", [&] {
        TopoDS_Face start = face_near(s.s, gp_Pnt(px, py, pz), kFaceLimit);
        TopTools_IndexedMapOfShape faces;
        TopExp::MapShapes(s.s, TopAbs_FACE, faces);
        TopTools_IndexedDataMapOfShapeListOfShape e2f, v2f;
        TopExp::MapShapesAndAncestors(s.s, TopAbs_EDGE, TopAbs_FACE, e2f);
        TopExp::MapShapesAndAncestors(s.s, TopAbs_VERTEX, TopAbs_FACE, v2f);
        std::vector<char> in(faces.Extent() + 1, prop == 3 ? 1 : 0);
        std::vector<int> todo{faces.FindIndex(start)};
        in[todo[0]] = 1;
        while (!todo.empty() && (prop == 1 || prop == 2)) {
            TopoDS_Face f = TopoDS::Face(faces(todo.back()));
            todo.pop_back();
            auto visit = [&](const TopoDS_Shape& g) {
                int k = faces.FindIndex(g);
                if (k > 0 && !in[k]) {
                    in[k] = 1;
                    todo.push_back(k);
                }
            };
            if (prop == 1) {
                for (TopExp_Explorer x(f, TopAbs_VERTEX); x.More(); x.Next())
                    for (TopTools_ListIteratorOfListOfShape it(v2f.FindFromKey(x.Current())); it.More(); it.Next()) visit(it.Value());
            } else {
                for (TopExp_Explorer x(f, TopAbs_EDGE); x.More(); x.Next()) {
                    const TopoDS_Edge& e = TopoDS::Edge(x.Current());
                    if (BRep_Tool::Degenerated(e)) continue;
                    for (TopTools_ListIteratorOfListOfShape it(e2f.FindFromKey(e)); it.More(); it.Next())
                        if (!it.Value().IsSame(f) && smooth_across(e, f, TopoDS::Face(it.Value()))) visit(it.Value());
                }
            }
        }
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        TopTools_IndexedMapOfShape edges;
        uint32_t nf = 0;
        for (int i = 1; i <= faces.Extent(); ++i) {
            if (bool(in[i]) == complement) continue;
            bb.Add(c, BRepBuilderAPI_Copy(faces(i)).Shape());
            TopExp::MapShapes(faces(i), TopAbs_EDGE, edges);
            ++nf;
        }
        if (nf == 0) throw std::runtime_error("a propagação pegou a forma inteira: não sobra complemento");
        counts.push_back(nf);
        counts.push_back(edges.Extent());
        if (nf == 1) return wrap(TopoDS_Iterator(c).Value());
        return wrap(c);
    });
}

// Curvas de interseção entre duas formas (superfícies, faces, planos).
rust::Vec<double> section_pts(const Shape& a, const Shape& b, rust::Vec<uint32_t>& ends) {
    return guard("interseção", [&] {
        BRepAlgoAPI_Section sc(a.s, b.s, Standard_False);
        sc.Approximation(Standard_True);
        sc.Build();
        if (!sc.IsDone()) throw std::runtime_error("não foi possível intersectar");
        TopTools_ListOfShape edges;
        for (TopExp_Explorer e(sc.Shape(), TopAbs_EDGE); e.More(); e.Next()) edges.Append(e.Current());
        if (edges.IsEmpty()) throw std::runtime_error("os elementos não se cruzam");
        Handle(TopTools_HSequenceOfShape) seq = new TopTools_HSequenceOfShape, wires;
        for (auto& e : edges) seq->Append(e);
        ShapeAnalysis_FreeBounds::ConnectEdgesToWires(seq, 1e-4, Standard_False, wires);
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        for (int i = 1; i <= wires->Length(); ++i) bb.Add(c, wires->Value(i));
        rust::Vec<double> out;
        wires_to_pts(c, out, ends);
        return out;
    });
}

namespace {

// Um sólido só, válido e com volume coerente com a área vezes a espessura. A faixa larga cobre a
// curvatura (a parede de dentro de uma curva é mais curta que a de fora) e os cantos vivos; o que sai
// dela é parede dobrada ou duplicada, que a conferência de forma às vezes deixa passar.
bool solido_coerente(TopoDS_Shape& r, double area, double t) {
    if (r.IsNull()) return false;
    int n = 0;
    TopoDS_Shape so;
    for (TopExp_Explorer e(r, TopAbs_SOLID); e.More(); e.Next(), ++n) so = e.Current();
    if (n != 1) return false;
    if (!forma_valida(so)) return false;
    GProp_GProps g;
    BRepGProp::VolumeProperties(so, g);
    double v = g.Mass();
    if (v < 0) so.Reverse(), v = -v;  // casco com a normal para dentro
    const double esperado = area * t;
    if (!(v > 0.4 * esperado && v < 2.5 * esperado)) return false;
    r = so;
    return true;
}

// Uma peça (casco ou face) engrossada de `t` para o lado da normal. Primeiro pelo deslocamento com
// ligação, que refaz as vizinhas e fecha as bordas com paredes; depois pelo simples, que só serve para
// casco liso. Nulo quando nenhum dá um sólido coerente.
TopoDS_Shape engrossa_peca(const TopoDS_Shape& p, double t, double tol) {
    GProp_GProps g;
    BRepGProp::SurfaceProperties(p, g);
    const double area = g.Mass();
    for (GeomAbs_JoinType j : {GeomAbs_Intersection, GeomAbs_Arc}) {
        try {
            BRepOffset_MakeOffset mo;
            mo.Initialize(p, t, tol, BRepOffset_Skin, j == GeomAbs_Intersection, Standard_False, j, Standard_True);
            mo.MakeOffsetShape();
            if (mo.IsDone()) {
                TopoDS_Shape r = mo.Shape();
                if (solido_coerente(r, area, t)) return r;
            }
        } catch (const Standard_Failure&) {
        }
    }
    try {
        BRepOffsetAPI_MakeThickSolid mk;
        mk.MakeThickSolidBySimple(p, t);
        if (mk.IsDone()) {
            // B386: no tubo de varredura em guia 3D a tampa anular sai com as curvas da borda fora da
            // tolerância da face; a cura de forma dentro de `solido_coerente` refaz curvas e tolerâncias
            TopoDS_Shape r = mk.Shape();
            if (solido_coerente(r, area, t)) return r;
        }
    } catch (const Standard_Failure&) {
    }
    return TopoDS_Shape();
}

}  // namespace

// Engrossar: superfície vira sólido com espessura t1 para o lado da normal e t2 para o outro.
// B518: as faces viram um casco costurado, que é deslocado e fechado inteiro (sem paredes que passam
// umas das outras na concordância); espessura maior que o raio de curvatura daquele lado é recusada
// com o raio mínimo, e o resultado só sai se for um sólido válido por peça conexa.
std::unique_ptr<Shape> thicken(const Shape& s, double t1, double t2) {
    return guard("engrossar", [&] {
        if (t1 + t2 <= 1e-9) throw std::runtime_error("a espessura total precisa ser positiva");
        double tol = 1e-4;
        int n = 0;
        TopoDS_Shape base = casco_costurado(s.s, tol, n);
        confere_raio(base, t1, "a espessura 1 (lado da normal)");
        confere_raio(base, -t2, "a espessura 2 (lado oposto)");
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        int feitos = 0;
        TopoDS_Shape um;
        for (const TopoDS_Shape& p0 : pecas_da_costura(base)) {
            TopoDS_Shape p = p0;
            if (t2 > 1e-9) {
                for (int m : {1, 2, 3})
                    if (!(p = desloca_peca(p0, -t2, std::max(tol, 1e-4), m)).IsNull()) break;
                if (p.IsNull()) throw std::runtime_error("não foi possível deslocar a superfície para o lado da espessura 2");
            }
            TopoDS_Shape r = engrossa_peca(p, t1 + t2, std::max(tol, 1e-4));
            if (r.IsNull()) throw std::runtime_error("não foi possível engrossar num sólido válido: confira se as faces se tocam e se a espessura cabe nas curvas");
            bb.Add(c, r);
            um = r;
            ++feitos;
        }
        return wrap(feitos == 1 ? um : TopoDS_Shape(c));
    });
}

namespace {

// Ponto de dentro da face (o centro de massa de uma face curva pode cair fora dela).
gp_Pnt ponto_dentro(const TopoDS_Face& f) {
    double u0, u1, v0, v1;
    BRepTools::UVBounds(f, u0, u1, v0, v1);
    BRepTopAdaptor_FClass2d cl(f, 1e-7);
    BRepAdaptor_Surface a(f, Standard_True);
    for (int k = 1; k <= 9; k += 2)
        for (int i = 1; i < 2 * k; i += 2)
            for (int j = 1; j < 2 * k; j += 2) {
                const double u = u0 + (u1 - u0) * i / (2.0 * k), v = v0 + (v1 - v0) * j / (2.0 * k);
                if (cl.Perform(gp_Pnt2d(u, v)) == TopAbs_IN) return a.Value(u, v);
            }
    return center_of(f);
}

// Superfície estendida para o corte do filete (B517). Os lados infinitos (plano, cilindro, extrusão)
// ficam limitados a ±L, como em `extended_faces`; os lados finitos de uma superfície de forma livre
// (a parede vinda de uma curva mais curta que a base) são prolongados em tangência por L. Sem isso a
// parede não atravessava a base inteira, a base não se dividia e a junta não aparecia.
TopoDS_Shape estendida_para_corte(const TopoDS_Shape& s, double L) {
    TopoDS_Compound c;
    BRep_Builder bb;
    bb.MakeCompound(c);
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) {
        Handle(Geom_Surface) g = BRep_Tool::Surface(TopoDS::Face(e.Current()));
        if (auto t = Handle(Geom_RectangularTrimmedSurface)::DownCast(g)) g = t->BasisSurface();
        double u1, u2, v1, v2;
        g->Bounds(u1, u2, v1, v2);
        const bool fu = !Precision::IsInfinite(u1) && !Precision::IsInfinite(u2);
        const bool fv = !Precision::IsInfinite(v1) && !Precision::IsInfinite(v2);
        const GeomAbs_SurfaceType tipo = GeomAdaptor_Surface(g).GetType();
        const bool analitica = tipo == GeomAbs_Plane || tipo == GeomAbs_Cylinder || tipo == GeomAbs_Cone || tipo == GeomAbs_Sphere || tipo == GeomAbs_Torus;
        u1 = std::max(u1, -L), u2 = std::min(u2, L), v1 = std::max(v1, -L), v2 = std::min(v2, L);
        if (!analitica && (fu || fv) && !g->IsUPeriodic() && !g->IsVPeriodic()) {
            try {
                Handle(Geom_BoundedSurface) b = GeomConvert::SurfaceToBSplineSurface(new Geom_RectangularTrimmedSurface(g, u1, u2, v1, v2));
                for (bool depois : {false, true}) {
                    if (fu) GeomLib::ExtendSurfByLength(b, L, 1, Standard_True, depois);
                    if (fv) GeomLib::ExtendSurfByLength(b, L, 1, Standard_False, depois);
                }
                g = b;
                g->Bounds(u1, u2, v1, v2);
            } catch (const Standard_Failure&) {
            }
        }
        BRepBuilderAPI_MakeFace mf(g, u1, u2, v1, v2, Precision::Confusion());
        if (mf.IsDone()) bb.Add(c, mf.Face());
    }
    return c;
}

// O lado escolhido de `s` cortada pela outra superfície estendida `outra` (`pos`: lado da normal dela).
// A superfície que só encosta na outra, sem atravessar, não se divide: fica inteira se estiver do lado
// escolhido. Lado de cada pedaço medido num ponto de dentro dele.
TopoDS_Shape parte_do_lado(const TopoDS_Shape& s, const TopoDS_Shape& outra, bool pos, const char* qual) {
    BRepAlgoAPI_Splitter sp;
    TopTools_ListOfShape args, tools;
    args.Append(s);
    tools.Append(outra);
    sp.SetArguments(args);
    sp.SetTools(tools);
    sp.Build();
    if (sp.HasErrors()) throw std::runtime_error("não foi possível cortar uma superfície pela outra");
    TopoDS_Compound c;
    BRep_Builder bb;
    bb.MakeCompound(c);
    int kept = 0;
    for (TopExp_Explorer e(sp.Shape(), TopAbs_FACE); e.More(); e.Next()) {
        const gp_Pnt m = ponto_dentro(TopoDS::Face(e.Current()));
        gp_Pnt q;
        gp_Vec nn;
        near_normal(outra, m, q, nn);
        const double h = gp_Vec(q, m).Dot(nn);
        if (std::abs(h) < 1e-7) continue;  // pedaço deitado sobre a outra
        if ((h > 0) == pos) bb.Add(c, e.Current()), ++kept;
    }
    if (kept == 0) {
        char buf[256];
        std::snprintf(buf, sizeof buf, "nada sobra da %s desse lado: inverta a seta dela", qual);
        throw std::runtime_error(buf);
    }
    return c;
}

// A face está sobre a forma (a distância de um ponto de dentro dela até a forma cabe na tolerância)?
bool face_sobre(const TopoDS_Face& f, const TopoDS_Shape& s, double tol) {
    BRepExtrema_DistShapeShape d(s, BRepBuilderAPI_MakeVertex(ponto_dentro(f)).Vertex());
    return d.IsDone() && d.NbSolution() > 0 && d.Value() <= tol;
}

// Filete nas arestas da junta: o casco do jeito que veio e, se a bola não couber daquele lado, o casco
// invertido (o filete de casco aberto rola do lado oposto à normal das faces).
TopoDS_Shape filete_na_junta(const TopoDS_Shape& casco, const std::vector<gp_Pnt>& meios, double r, double r2) {
    for (bool inv : {false, true}) {
        TopoDS_Shape sh = inv ? casco.Reversed() : casco;
        try {
            BRepFilletAPI_MakeFillet mf(sh);
            TopTools_IndexedMapOfShape es;
            TopExp::MapShapes(sh, TopAbs_EDGE, es);
            int n = 0;
            for (const gp_Pnt& m : meios)
                for (int i = 1; i <= es.Extent(); ++i) {
                    const TopoDS_Edge& e = TopoDS::Edge(es(i));
                    if (BRep_Tool::Degenerated(e)) continue;
                    BRepAdaptor_Curve cv(e);
                    if (cv.Value((cv.FirstParameter() + cv.LastParameter()) / 2).Distance(m) > 1e-6) continue;
                    if (mf.Contour(e) == 0) {
                        if (r2 > 0) mf.Add(r, r2, e);
                        else mf.Add(r, e);
                        ++n;
                    }
                }
            if (n == 0) continue;
            mf.Build();
            if (!mf.IsDone()) continue;
            TopoDS_Shape out = mf.Shape();
            if (forma_valida(out) && dominios(out) == 1) return inv ? out.Reversed() : out;
        } catch (const Standard_Failure&) {
        }
    }
    return TopoDS_Shape();
}

}  // namespace

// B517: filete entre duas superfícies. Cada uma é cortada pela outra estendida e fica o lado que a seta
// escolhe (`keep_*_pos`: lado da normal da outra); a que só encosta na outra, sem atravessar, fica
// inteira (antes o corte dizia "não atravessa" e o filete não saía). As partes são costuradas com a
// tolerância que cresce até 0,01 mm: as duas curvas da junta saem de interseções separadas e não
// coincidem exatamente, e com a costura de 0,001 mm fixa a junta ficava aberta numa superfície curva
// (sem aresta entre duas faces, o filete não tinha onde rolar). As arestas da junta são as que ficam
// entre uma face de cada entrada; o filete de raio `r` (até `r2`, se > 0) roda nelas e o resultado é um
// casco só.
std::unique_ptr<Shape> surf_fillet(const Shape& a, const Shape& b, bool keep_a_pos, bool keep_b_pos, double r, double r2) {
    return guard("filete entre superfícies", [&] {
        if (r <= 0) throw std::runtime_error("o raio precisa ser positivo");
        const double L = size_of(a.s, b.s);
        const TopoDS_Shape pa = parte_do_lado(a.s, estendida_para_corte(b.s, L), keep_a_pos, "superfície 1");
        const TopoDS_Shape pb = parte_do_lado(b.s, estendida_para_corte(a.s, L), keep_b_pos, "superfície 2");
        TopoDS_Compound kept;
        BRep_Builder bb;
        bb.MakeCompound(kept);
        bb.Add(kept, pa);
        bb.Add(kept, pb);
        double tol = 1e-4;
        int pecas = 0;
        const TopoDS_Shape casco = casco_costurado(kept, tol, pecas);
        if (pecas != 1)
            throw std::runtime_error("as superfícies não se encontram: prolongue uma até a outra (Extrapolar) ou mova-as até se cruzarem");
        // arestas da junta: entre uma face sobre a entrada 1 e outra sobre a entrada 2
        TopTools_IndexedDataMapOfShapeListOfShape e2f;
        TopExp::MapShapesAndAncestors(casco, TopAbs_EDGE, TopAbs_FACE, e2f);
        const double perto = std::max(10 * tol, 1e-3);
        std::vector<gp_Pnt> meios;
        for (int i = 1; i <= e2f.Extent(); ++i) {
            const TopoDS_Edge& e = TopoDS::Edge(e2f.FindKey(i));
            if (BRep_Tool::Degenerated(e) || e2f(i).Extent() != 2) continue;
            const TopoDS_Face& f1 = TopoDS::Face(e2f(i).First());
            const TopoDS_Face& f2 = TopoDS::Face(e2f(i).Last());
            const bool a1 = face_sobre(f1, a.s, perto), b1 = face_sobre(f1, b.s, perto);
            const bool a2 = face_sobre(f2, a.s, perto), b2 = face_sobre(f2, b.s, perto);
            if (!((a1 && b2 && !b1 && !a2) || (b1 && a2 && !a1 && !b2))) continue;
            BRepAdaptor_Curve cv(e);
            meios.push_back(cv.Value((cv.FirstParameter() + cv.LastParameter()) / 2));
        }
        if (meios.empty())
            throw std::runtime_error("as superfícies não se encontram: prolongue uma até a outra (Extrapolar) ou mova-as até se cruzarem");
        TopoDS_Shape out = filete_na_junta(casco, meios, r, r2);
        if (out.IsNull()) {
            char buf[256];
            std::snprintf(buf, sizeof buf, "o raio %.4g mm não cabe entre as partes que ficam: diminua o raio ou troque o lado pelas setas", r);
            throw std::runtime_error(buf);
        }
        return wrap(out);
    });
}

// Fechar superfície: costura; bordas livres planas viram tampas; o casco fechado vira sólido.
std::unique_ptr<Shape> close_surface(const Shape& s, double tol) {
    return guard("fechar superfície", [&] {
        tol = std::max(tol, 1e-6);
        TopoDS_Shape sh = sewn(s.s, tol);
        BRepBuilderAPI_Sewing sw(tol);
        for (TopExp_Explorer e(sh, TopAbs_FACE); e.More(); e.Next()) sw.Add(e.Current());
        for (TopExp_Explorer e(free_wires(sh), TopAbs_WIRE); e.More(); e.Next()) {
            const TopoDS_Wire& w = TopoDS::Wire(e.Current());
            if (!BRep_Tool::IsClosed(w)) throw std::runtime_error("há bordas abertas que não formam contorno fechado — junte/cure antes");
            BRepBuilderAPI_MakeFace mf(w, Standard_True);
            if (!mf.IsDone()) {
                // borda não plana: preenchimento
                BRepOffsetAPI_MakeFilling fl;
                for (TopExp_Explorer ed(w, TopAbs_EDGE); ed.More(); ed.Next()) fl.Add(TopoDS::Edge(ed.Current()), GeomAbs_C0);
                fl.Build();
                if (!fl.IsDone()) throw std::runtime_error("uma borda livre não pôde ser tampada");
                sw.Add(fl.Shape());
            } else sw.Add(mf.Face());
        }
        sw.Perform();
        TopoDS_Shape closed = sw.SewedShape();
        TopExp_Explorer se(closed, TopAbs_SHELL);
        if (!se.More()) throw std::runtime_error("a costura não formou casco");
        BRepBuilderAPI_MakeSolid ms(TopoDS::Shell(se.Current()));
        if (!ms.IsDone()) throw std::runtime_error("o casco não fecha um volume");
        Handle(ShapeFix_Solid) fx = new ShapeFix_Solid(ms.Solid());
        fx->Perform();
        TopoDS_Shape r = fx->Solid();
        GProp_GProps g;
        BRepGProp::VolumeProperties(r, g);
        if (g.Mass() < 0) r.Reverse();
        return wrap(r);
    });
}

// Células de volume (Boundary Fill): todos os volumes fechados formados pelas formas; tira as
// células que contêm algum ponto de `exclude`.
std::unique_ptr<Shape> volume_cells(const Shape& s, rust::Slice<const double> exclude) {
    return guard("preencher fronteiras", [&] {
        BOPAlgo_MakerVolume mv;
        TopTools_ListOfShape args;
        for (TopoDS_Iterator it(s.s); it.More(); it.Next()) args.Append(it.Value());
        if (args.IsEmpty()) args.Append(s.s);
        mv.SetArguments(args);
        mv.SetIntersect(Standard_True);
        mv.Perform();
        if (mv.HasErrors()) throw std::runtime_error("não foi possível montar os volumes");
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        int kept = 0;
        for (TopExp_Explorer e(mv.Shape(), TopAbs_SOLID); e.More(); e.Next()) {
            bool drop = false;
            for (size_t i = 0; i + 2 < exclude.size(); i += 3) {
                BRepClass3d_SolidClassifier cl(e.Current(), gp_Pnt(exclude[i], exclude[i + 1], exclude[i + 2]), 1e-6);
                if (cl.State() == TopAbs_IN) drop = true;
            }
            if (!drop) bb.Add(c, e.Current()), ++kept;
        }
        if (kept == 0) throw std::runtime_error("as superfícies não fecham nenhum volume");
        return wrap(c);
    });
}

// Malha (triângulos achatados, 9 números cada) → B-rep: faces planas costuradas; sólido se fechar.
std::unique_ptr<Shape> mesh_shape(rust::Slice<const double> tris, bool solid) {
    return guard("malha → B-rep", [&] {
        BRepBuilderAPI_Sewing sw(1e-6);
        size_t n = 0;
        for (size_t i = 0; i + 8 < tris.size(); i += 9) {
            gp_Pnt a(tris[i], tris[i + 1], tris[i + 2]), b(tris[i + 3], tris[i + 4], tris[i + 5]), c(tris[i + 6], tris[i + 7], tris[i + 8]);
            if (a.Distance(b) < 1e-9 || b.Distance(c) < 1e-9 || a.Distance(c) < 1e-9) continue;
            if (gp_Vec(a, b).Crossed(gp_Vec(a, c)).Magnitude() < 1e-12) continue;
            BRepBuilderAPI_MakePolygon pg(a, b, c, Standard_True);
            BRepBuilderAPI_MakeFace mf(pg.Wire(), Standard_True);
            if (mf.IsDone()) sw.Add(mf.Face()), ++n;
        }
        if (n == 0) throw std::runtime_error("a malha não tem triângulos válidos");
        sw.Perform();
        TopoDS_Shape r = sw.SewedShape();
        if (solid) {
            TopExp_Explorer se(r, TopAbs_SHELL);
            if (se.More() && BRep_Tool::IsClosed(se.Current())) {
                BRepBuilderAPI_MakeSolid ms(TopoDS::Shell(se.Current()));
                if (ms.IsDone()) {
                    Handle(ShapeFix_Solid) fx = new ShapeFix_Solid(ms.Solid());
                    fx->Perform();
                    r = fx->Solid();
                    GProp_GProps g;
                    BRepGProp::VolumeProperties(r, g);
                    if (g.Mass() < 0) r.Reverse();
                }
            }
        }
        return wrap(r);
    });
}

bool is_solid(const Shape& s) { return TopExp_Explorer(s.s, TopAbs_SOLID).More(); }

// Afinidade: escala independente em X, Y, Z do sistema local (origem o, eixo x, normal n).
std::unique_ptr<Shape> affinity(const Shape& s, double ox, double oy, double oz, double xx, double xy, double xz, double nx, double ny, double nz,
                                double fx, double fy, double fz) {
    return guard("afinidade", [&] {
        if (std::abs(fx * fy * fz) < 1e-12) throw std::runtime_error("os fatores não podem ser zero");
        gp_Ax3 ax(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz), gp_Dir(xx, xy, xz));
        gp_Trsf to_local;
        to_local.SetTransformation(ax);  // global → local
        gp_GTrsf scale;
        scale.SetVectorialPart(gp_Mat(fx, 0, 0, 0, fy, 0, 0, 0, fz));
        gp_GTrsf g(to_local.Inverted());
        g.Multiply(scale);
        g.Multiply(gp_GTrsf(to_local));
        BRepBuilderAPI_GTransform mk(s.s, g, Standard_True);
        if (!mk.IsDone()) throw std::runtime_error("não foi possível aplicar a afinidade");
        return wrap(mk.Shape());
    });
}

// Forma em texto B-rep (para guardar a cópia de uma superfície publicada na peça que a importa).
rust::String to_brep(const Shape& s) {
    return guard("B-rep", [&] {
        std::ostringstream o;
        BRepTools::Write(s.s, o);
        return rust::String(o.str());
    });
}

std::unique_ptr<Shape> from_brep(rust::Str text) {
    return guard("B-rep", [&] {
        std::istringstream i(std::string(text.data(), text.size()));
        TopoDS_Shape s;
        BRep_Builder b;
        BRepTools::Read(s, i, b);
        if (s.IsNull()) throw std::runtime_error("cópia B-rep inválida");
        return wrap(s);
    });
}

// Furo completo: corpo cilíndrico (ou cônico, `taper` = meio-ângulo em graus), fundo plano ou em V
// (`tip` = ângulo da ponta da broca; 0 = plano) e cabeça: 0 simples, 1 rebaixo (d2, p2),
// 2 escareado (d2, ângulo p2), 3 contrafurado: rebaixo (d2, p2) com fundo cônico de ângulo a3.
std::unique_ptr<Shape> hole_tool_ext(double ox, double oy, double oz, double dx, double dy, double dz, double diameter, double depth,
                                     uint8_t kind, double d2, double p2, double a3, double tip, double taper) {
    return guard("furo", [&] {
        if (diameter <= 0 || depth <= 0) throw std::runtime_error("diâmetro e profundidade devem ser positivos");
        gp_Dir d(dx, dy, dz);
        const double lead = 0.05;
        gp_Pnt top(ox, oy, oz);
        gp_Pnt o(ox - d.X() * lead, oy - d.Y() * lead, oz - d.Z() * lead);
        gp_Ax2 ax(o, d);
        double r = diameter / 2;
        // corpo
        double r_bot = r;
        TopoDS_Shape tool;
        if (std::abs(taper) > 1e-9) {
            r_bot = r - (depth + lead) * std::tan(taper * M_PI / 180.0);
            if (r_bot <= 0.01) throw std::runtime_error("conicidade grande demais para a profundidade");
            tool = BRepPrimAPI_MakeCone(ax, r, r_bot, depth + lead).Shape();
        } else {
            tool = BRepPrimAPI_MakeCylinder(ax, r, depth + lead).Shape();
        }
        // fundo em V (ponta da broca)
        if (tip > 1e-6 && tip < 179.9) {
            double h = r_bot / std::tan(tip / 2 * M_PI / 180.0);
            gp_Pnt b(top.X() + d.X() * depth, top.Y() + d.Y() * depth, top.Z() + d.Z() * depth);
            tool = BRepAlgoAPI_Fuse(tool, BRepPrimAPI_MakeCone(gp_Ax2(b, d), r_bot, 1e-4, h).Shape()).Shape();
        }
        // cabeça
        if ((kind == 1 || kind == 3) && d2 > diameter && p2 > 0) {
            tool = BRepAlgoAPI_Fuse(tool, BRepPrimAPI_MakeCylinder(ax, d2 / 2, p2 + lead).Shape()).Shape();
            if (kind == 3 && a3 > 1e-6 && a3 < 179.9) {
                double h = (d2 - diameter) / 2 / std::tan(a3 / 2 * M_PI / 180.0);
                gp_Pnt b(top.X() + d.X() * p2, top.Y() + d.Y() * p2, top.Z() + d.Z() * p2);
                tool = BRepAlgoAPI_Fuse(tool, BRepPrimAPI_MakeCone(gp_Ax2(b, d), d2 / 2, r, h).Shape()).Shape();
            }
        } else if (kind == 2 && d2 > diameter && p2 > 0 && p2 < 180) {
            double h = (d2 - diameter) / 2 / std::tan(p2 / 2 * M_PI / 180.0);
            TopoDS_Shape cone = BRepPrimAPI_MakeCone(gp_Ax2(top, d), d2 / 2, r, h).Shape();
            TopoDS_Shape cap = BRepPrimAPI_MakeCylinder(ax, d2 / 2, lead).Shape();
            tool = BRepAlgoAPI_Fuse(BRepAlgoAPI_Fuse(tool, cone).Shape(), cap).Shape();
        }
        return wrap(tool);
    });
}

// Primeiro ponto onde a reta (p, d) entra na forma (t > 0): [x, y, z, t] ou vazio.
rust::Vec<double> ray_hit(const Shape& s, double px, double py, double pz, double dx, double dy, double dz) {
    return guard("projeção", [&] {
        IntCurvesFace_ShapeIntersector it;
        it.Load(s.s, 1e-6);
        it.Perform(gp_Lin(gp_Pnt(px, py, pz), gp_Dir(dx, dy, dz)), -1e9, 1e9);
        rust::Vec<double> out;
        if (!it.IsDone() || it.NbPnt() == 0) return out;
        int best = 0;
        double bt = 1e300;
        for (int i = 1; i <= it.NbPnt(); ++i)
            if (std::abs(it.WParameter(i)) < bt) bt = std::abs(it.WParameter(i)), best = i;
        gp_Pnt q = it.Pnt(best);
        for (double x : {q.X(), q.Y(), q.Z(), it.WParameter(best)}) out.push_back(x);
        return out;
    });
}

/// Todos os parâmetros em que a reta cruza a forma, em ordem: entradas e saídas se alternam.
rust::Vec<double> ray_hits(const Shape& s, double px, double py, double pz, double dx, double dy, double dz) {
    return guard("projeção", [&] {
        IntCurvesFace_ShapeIntersector it;
        it.Load(s.s, 1e-6);
        it.Perform(gp_Lin(gp_Pnt(px, py, pz), gp_Dir(dx, dy, dz)), -1e9, 1e9);
        std::vector<double> t;
        if (it.IsDone())
            for (int i = 1; i <= it.NbPnt(); ++i) t.push_back(it.WParameter(i));
        std::sort(t.begin(), t.end());
        rust::Vec<double> out;
        for (double x : t) out.push_back(x);
        return out;
    });
}

// ---------- B169: malha → B-rep híbrido (faces analíticas onde houve ajuste, triangulares onde não)
//
// O Rust reconhece as regiões e encaixa os vértices (`malha_primitivas.rs`). Aqui:
// - cada vértice da malha vira um TopoDS_Vertex compartilhado, para os laços fecharem;
// - cada trecho de borda entre duas regiões analíticas tenta a interseção das duas superfícies
//   (GeomAPI_IntSS): vale a curva que passa por todos os vértices do trecho; senão, segmentos retos;
// - a face analítica nasce da superfície e dos laços; o ShapeFix_Face cria as curvas paramétricas,
//   decide o laço externo e põe a costura que falta no cilindro, no cone e no toro;
// - triângulo sem ajuste vira face plana;
// - tudo é costurado, vira sólido quando fecha, e as faces coplanares do mesmo domínio se unem.

namespace {

Handle(Geom_Surface) superficie_da_regiao(const double* r) {
    const int tipo = int(r[0] + 0.5);
    const gp_Pnt o(r[1], r[2], r[3]);
    const gp_Dir ax(r[4], r[5], r[6]);
    gp_Vec xv(r[7], r[8], r[9]);
    xv = xv - gp_Vec(ax) * xv.Dot(gp_Vec(ax));
    if (xv.Magnitude() < 1e-9) throw std::runtime_error("direção de referência nula");
    const gp_Ax3 a3(o, ax, gp_Dir(xv));
    switch (tipo) {
        case 0: return new Geom_Plane(a3);
        case 1: return new Geom_CylindricalSurface(a3, r[10]);
        case 2: return new Geom_ConicalSurface(a3, r[10], 0.0);  // raio zero na origem: a origem é o ápice
        case 3: return new Geom_SphericalSurface(a3, r[10]);
        case 4: return new Geom_ToroidalSurface(a3, r[10], r[11]);
    }
    throw std::runtime_error("tipo de superfície desconhecido");
}

// Curva da interseção que passa por todos os pontos dentro de `tol`.
Handle(Geom_Curve) curva_comum(const Handle(Geom_Surface)& a, const Handle(Geom_Surface)& b, const std::vector<gp_Pnt>& pts, double tol) {
    try {
        GeomAPI_IntSS in(a, b, 1e-7);
        if (!in.IsDone()) return nullptr;
        for (int i = 1; i <= in.NbLines(); ++i) {
            Handle(Geom_Curve) c = in.Line(i);
            bool todos = true;
            for (const gp_Pnt& p : pts) {
                GeomAPI_ProjectPointOnCurve pr(p, c);
                if (pr.NbPoints() == 0 || pr.LowerDistance() > tol) {
                    todos = false;
                    break;
                }
            }
            if (todos) return c;
        }
    } catch (const Standard_Failure&) {
    }
    return nullptr;
}

// As duas superfícies são tangentes em `p`? Ali a interseção é instável (o filete que encosta no
// plano): a aresta fica pelos vértices da malha.
bool tangentes_em(const Handle(Geom_Surface)& a, const Handle(Geom_Surface)& b, const gp_Pnt& p) {
    auto normal = [&](const Handle(Geom_Surface)& s, gp_Vec& n) {
        GeomAPI_ProjectPointOnSurf pr(p, s);
        if (pr.NbPoints() == 0) return false;
        double u, v;
        pr.LowerDistanceParameters(u, v);
        GeomLProp_SLProps sl(s, u, v, 1, 1e-9);
        if (!sl.IsNormalDefined()) return false;
        n = gp_Vec(sl.Normal());
        return true;
    };
    gp_Vec na, nb;
    if (!normal(a, na) || !normal(b, nb)) return true;
    return std::abs(na.Dot(nb)) > std::cos(5.0 * M_PI / 180.0);
}

double parametro(const Handle(Geom_Curve)& c, const gp_Pnt& p) {
    GeomAPI_ProjectPointOnCurve pr(p, c);
    if (pr.NbPoints() == 0) throw std::runtime_error("ponto fora da curva de interseção");
    return pr.LowerDistanceParameter();
}

// Aresta sobre a curva de `va` a `vb`, passando perto de `meio`. Fechada quando va == vb.
TopoDS_Edge aresta_na_curva(const Handle(Geom_Curve)& c, const TopoDS_Vertex& va, const TopoDS_Vertex& vb, const gp_Pnt& meio, bool fechada, double tol) {
    const gp_Pnt pa = BRep_Tool::Pnt(va), pb = BRep_Tool::Pnt(vb);
    double ua = parametro(c, pa), ub = parametro(c, pb), um = parametro(c, meio);
    bool reverso = false;
    if (c->IsPeriodic()) {
        const double T = c->Period();
        auto norm = [&](double u) {
            while (u < ua) u += T;
            while (u >= ua + T) u -= T;
            return u;
        };
        um = norm(um);
        if (fechada) {
            ub = ua + T;
            reverso = um - ua > T / 2;
        } else {
            ub = norm(ub);
            if (um > ub) {
                // o trecho vai para trás: de ub − T até ua, e a aresta é usada ao contrário
                std::swap(ua, ub);
                ua -= T;
                reverso = true;
            }
        }
    } else {
        if (fechada) throw std::runtime_error("curva aberta num laço fechado");
        if (ua > ub) {
            std::swap(ua, ub);
            reverso = true;
        }
    }
    BRep_Builder bb;
    TopoDS_Edge e;
    bb.MakeEdge(e, c, tol);
    TopoDS_Vertex v1 = reverso ? vb : va, v2 = reverso ? va : vb;
    bb.Add(e, TopoDS::Vertex(v1.Oriented(TopAbs_FORWARD)));
    bb.Add(e, TopoDS::Vertex(v2.Oriented(TopAbs_REVERSED)));
    bb.Range(e, ua, ub);
    BRepLib::UpdateTolerances(e);
    if (reverso) e.Reverse();
    return e;
}

TopoDS_Face face_triangular(const TopoDS_Vertex& a, const TopoDS_Vertex& b, const TopoDS_Vertex& c) {
    BRepBuilderAPI_MakePolygon pg(a, b, c, Standard_True);
    if (!pg.IsDone()) return TopoDS_Face();
    BRepBuilderAPI_MakeFace mf(pg.Wire(), Standard_True);
    return mf.IsDone() ? mf.Face() : TopoDS_Face();
}

TopoDS_Shape fechar_e_validar(const TopoDS_Shape& costurada, bool solid) {
    // tolerâncias de vértice e aresta e o mesmo parâmetro entre curva 3D e paramétrica
    ShapeFix_Shape sfs(costurada);
    sfs.Perform();
    TopoDS_Shape r = sfs.Shape();
    if (solid) {
        TopExp_Explorer se(r, TopAbs_SHELL);
        TopoDS_Shell sh;
        if (!se.More()) {
            // uma face só, fechada em si (esfera ou toro inteiro): a costura não faz casca
            BRep_Builder bb;
            bb.MakeShell(sh);
            int nf = 0;
            for (TopExp_Explorer fe(r, TopAbs_FACE); fe.More(); fe.Next()) bb.Add(sh, fe.Current()), ++nf;
            if (nf == 0) throw std::runtime_error("a costura não formou casca");
            sh.Closed(BRep_Tool::IsClosed(sh));
        } else {
            sh = TopoDS::Shell(se.Current());
            se.Next();
        }
        if (se.More()) throw std::runtime_error("a costura deixou mais de uma casca");
        if (!BRep_Tool::IsClosed(sh)) throw std::runtime_error("a casca costurada não fecha");
        BRepBuilderAPI_MakeSolid ms(sh);
        if (!ms.IsDone()) throw std::runtime_error("não foi possível fechar o sólido");
        Handle(ShapeFix_Solid) fx = new ShapeFix_Solid(ms.Solid());
        fx->Perform();
        r = fx->Solid();
        GProp_GProps g;
        BRepGProp::VolumeProperties(r, g);
        if (g.Mass() < 0) r.Reverse();
    }
    BRepCheck_Analyzer an(r);
    if (!an.IsValid()) {
        // diz quais faces quebram, por tipo de superfície, para o relatório
        int ruins[7] = {0, 0, 0, 0, 0, 0, 0};
        for (TopExp_Explorer fe(r, TopAbs_FACE); fe.More(); fe.Next()) {
            BRepCheck_Analyzer af(fe.Current());
            if (af.IsValid()) continue;
            switch (BRepAdaptor_Surface(TopoDS::Face(fe.Current())).GetType()) {
                case GeomAbs_Plane: ruins[0]++; break;
                case GeomAbs_Cylinder: ruins[1]++; break;
                case GeomAbs_Cone: ruins[2]++; break;
                case GeomAbs_Sphere: ruins[3]++; break;
                case GeomAbs_Torus: ruins[4]++; break;
                default: ruins[5]++;
            }
        }
        std::ostringstream o;
        o << "o B-rep montado não é válido (faces ruins: " << ruins[0] << " planas, " << ruins[1] << " cilíndricas, " << ruins[2] << " cônicas, " << ruins[3] << " esféricas, " << ruins[4] << " tóricas, " << ruins[5] << " outras)";
        throw std::runtime_error(o.str());
    }
    return r;
}

TopoDS_Shape unir_coplanares(const TopoDS_Shape& s, bool solid) {
    try {
        ShapeUpgrade_UnifySameDomain u(s, Standard_True, Standard_True, Standard_True);
        u.Build();
        TopoDS_Shape r = u.Shape();
        BRepCheck_Analyzer an(r);
        if (!an.IsValid()) return s;
        if (solid && !TopExp_Explorer(r, TopAbs_SOLID).More()) return s;
        return r;
    } catch (const Standard_Failure&) {
        return s;
    }
}

}  // namespace

// vertices: 3 por vértice; facetas: 3 índices por triângulo sem ajuste; regioes: 12 números por região
// (tipo, origem, eixo, referência, dois números); lacos: por região, nº de laços e, por laço, nº de
// trechos seguido de pares (vértice, região do outro lado ou −1).
std::unique_ptr<Shape> mesh_hybrid(rust::Slice<const double> vertices, rust::Slice<const uint32_t> facetas, rust::Slice<const double> regioes,
                                   rust::Slice<const int64_t> lacos, double tol, bool solid) {
    return guard("malha → B-rep híbrido", [&] {
        const size_t nv = vertices.size() / 3, nr = regioes.size() / 12;
        std::vector<TopoDS_Vertex> vs(nv);
        BRep_Builder bb;
        for (size_t i = 0; i < nv; ++i) bb.MakeVertex(vs[i], gp_Pnt(vertices[3 * i], vertices[3 * i + 1], vertices[3 * i + 2]), tol);
        auto ponto = [&](int64_t i) {
            if (i < 0 || size_t(i) >= nv) throw std::runtime_error("vértice fora da malha");
            return gp_Pnt(vertices[3 * i], vertices[3 * i + 1], vertices[3 * i + 2]);
        };
        std::vector<Handle(Geom_Surface)> sup(nr);
        for (size_t k = 0; k < nr; ++k) sup[k] = superficie_da_regiao(&regioes[12 * k]);

        BRepBuilderAPI_Sewing sw(std::max(tol * 20, 1e-5));
        size_t faces = 0, pos = 0;
        auto ler = [&]() {
            if (pos >= lacos.size()) throw std::runtime_error("descrição dos laços incompleta");
            return lacos[pos++];
        };
        for (size_t k = 0; k < nr; ++k) {
            const int64_t nl = ler();
            TopoDS_Face f;
            bb.MakeFace(f, sup[k], tol);
            if (nl == 0) {
                // superfície fechada sem borda (esfera ou toro inteiro)
                f = BRepBuilderAPI_MakeFace(sup[k], Precision::Confusion()).Face();
            }
            for (int64_t l = 0; l < nl; ++l) {
                const int64_t n = ler();
                std::vector<int64_t> v(static_cast<size_t>(n)), viz(static_cast<size_t>(n));
                for (int64_t i = 0; i < n; ++i) v[size_t(i)] = ler(), viz[size_t(i)] = ler();
                // trechos: onde a região do outro lado muda
                std::vector<size_t> cortes;
                for (size_t i = 0; i < v.size(); ++i)
                    if (viz[i] != viz[(i + v.size() - 1) % v.size()]) cortes.push_back(i);
                TopoDS_Wire w;
                bb.MakeWire(w);
                auto segmentos = [&](size_t i0, size_t cont) {
                    for (size_t j = 0; j < cont; ++j) {
                        const size_t a = (i0 + j) % v.size(), b = (i0 + j + 1) % v.size();
                        if (v[a] == v[b]) continue;
                        BRepBuilderAPI_MakeEdge me(vs[size_t(v[a])], vs[size_t(v[b])]);
                        if (me.IsDone()) bb.Add(w, me.Edge());
                    }
                };
                auto trecho = [&](size_t i0, size_t cont, bool fechado) {
                    const int64_t nb = viz[i0];
                    bool feito = false;
                    if (nb >= 0 && size_t(nb) < nr && cont >= 1) {
                        std::vector<gp_Pnt> pts;
                        for (size_t j = 0; j <= cont; ++j) pts.push_back(ponto(v[(i0 + j) % v.size()]));
                        Handle(Geom_Curve) c;
                        if (!tangentes_em(sup[k], sup[size_t(nb)], pts[pts.size() / 2])) c = curva_comum(sup[k], sup[size_t(nb)], pts, tol * 10);
                        if (!c.IsNull()) {
                            try {
                                const size_t a = i0 % v.size(), b = (i0 + cont) % v.size();
                                const gp_Pnt meio = pts[pts.size() / 2];
                                bb.Add(w, aresta_na_curva(c, vs[size_t(v[a])], vs[size_t(v[b])], meio, fechado, tol));
                                feito = true;
                            } catch (const std::exception&) {
                            } catch (const Standard_Failure&) {
                            }
                        }
                    }
                    if (!feito) segmentos(i0, cont);
                };
                if (cortes.empty()) {
                    trecho(0, v.size(), true);
                } else {
                    for (size_t c = 0; c < cortes.size(); ++c) {
                        const size_t i0 = cortes[c], i1 = cortes[(c + 1) % cortes.size()];
                        trecho(i0, (i1 + v.size() - i0 - 1) % v.size() + 1, false);
                    }
                }
                bb.Add(f, w);
            }
            if (nl > 0) {
                ShapeFix_Face sff(f);
                sff.SetPrecision(tol);
                sff.SetMaxTolerance(std::max(tol * 1000, 0.1));
                sff.Perform();
                f = sff.Face();
            }
            sw.Add(f);
            ++faces;
        }
        for (size_t i = 0; i + 2 < facetas.size(); i += 3) {
            const uint32_t a = facetas[i], b = facetas[i + 1], c = facetas[i + 2];
            if (a >= nv || b >= nv || c >= nv) throw std::runtime_error("triângulo fora da malha");
            TopoDS_Face f = face_triangular(vs[a], vs[b], vs[c]);
            if (!f.IsNull()) sw.Add(f), ++faces;
        }
        if (faces == 0) throw std::runtime_error("nada para montar");
        sw.Perform();
        TopoDS_Shape r = fechar_e_validar(sw.SewedShape(), solid);
        return wrap(unir_coplanares(r, solid));
    });
}

// Faces por tipo de superfície: plano, cilindro, cone, esfera, toro, outras.
rust::Vec<uint32_t> face_types(const Shape& s) {
    rust::Vec<uint32_t> out;
    for (int i = 0; i < 6; ++i) out.push_back(0);
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(s.s, TopAbs_FACE, faces);
    for (int i = 1; i <= faces.Extent(); ++i) {
        BRepAdaptor_Surface a(TopoDS::Face(faces(i)));
        switch (a.GetType()) {
            case GeomAbs_Plane: out[0]++; break;
            case GeomAbs_Cylinder: out[1]++; break;
            case GeomAbs_Cone: out[2]++; break;
            case GeomAbs_Sphere: out[3]++; break;
            case GeomAbs_Torus: out[4]++; break;
            default: out[5]++;
        }
    }
    return out;
}

}  // namespace fenkra
