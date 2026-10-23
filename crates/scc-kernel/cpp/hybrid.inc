// Bancada de Superfícies: superfícies primitivas, varredura/multi-seções abertas, operações
// (juntar, curar, dividir, aparar, contorno, desmontar, desaparar, extrapolar) e híbridos
// (engrossar, fechar, células de volume, malha → B-rep).
// Incluído no fim de kernel.cpp, depois de surfaces.inc (usa face_near, spine_wire, extended_faces).

namespace scc {

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
        if (len <= 0) throw std::runtime_error("o comprimento precisa ser positivo");
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
        GeomLib::ExtendSurfByLength(ext, len, std::clamp(cont, 1, 2), sides[best].inU, sides[best].after);
        BRepBuilderAPI_MakeFace mf(Handle(Geom_Surface)::DownCast(ext), Precision::Confusion());
        if (!mf.IsDone()) throw std::runtime_error("não foi possível extrapolar");
        return wrap(mf.Face());
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

}  // namespace scc
