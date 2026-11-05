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

// Varredura aberta: perfil (curva) ao longo da guia, sem fechar em sólido.
std::unique_ptr<Shape> sweep_surface(rust::Slice<const double> profile, rust::Slice<const double> spine, uint8_t mode) {
    return guard("varredura de superfície", [&] {
        TopoDS_Wire p = spine_wire(profile), w = spine_wire(spine);
        BRepOffsetAPI_MakePipeShell mk(w);
        if (mode == 1) mk.SetMode(gp_Dir(0, 0, 1));
        else mk.SetMode(mode == 2 ? Standard_True : Standard_False);  // 2 = Frenet; 0 = Frenet corrigido
        mk.Add(p, Standard_False, Standard_False);
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("não foi possível varrer o perfil ao longo da guia");
        return wrap(mk.Shape());
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

// Engrossar: superfície vira sólido com espessura t1 para o lado da normal e t2 para o outro.
std::unique_ptr<Shape> thicken(const Shape& s, double t1, double t2) {
    return guard("engrossar", [&] {
        if (t1 + t2 <= 1e-9) throw std::runtime_error("a espessura total precisa ser positiva");
        TopoDS_Shape base = sewn(s.s, 1e-4);
        if (std::abs(t2) > 1e-9) {
            BRepOffsetAPI_MakeOffsetShape off;
            off.PerformBySimple(base, -t2);
            if (!off.IsDone()) throw std::runtime_error("não foi possível deslocar a superfície");
            base = off.Shape();
        }
        BRepOffsetAPI_MakeThickSolid mk;
        mk.MakeThickSolidBySimple(base, t1 + t2);
        if (!mk.IsDone()) throw std::runtime_error("não foi possível engrossar (raio de curvatura menor que a espessura?)");
        TopoDS_Shape r = mk.Shape();
        GProp_GProps g;
        BRepGProp::VolumeProperties(r, g);
        if (g.Mass() < 0) r.Reverse();  // casco com a normal para dentro
        return wrap(r);
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

}  // namespace fenkra
