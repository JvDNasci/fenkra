// Superfícies de referência, consultas (medição) e término "até o próximo".
// Incluído no fim de kernel.cpp, depois de features.inc (usa face_near, spine_wire, P3).

namespace fenkra {

namespace {

// Curva (polilinha xyz) como aresta suave ou polilinha, igual à curva guia.
TopoDS_Wire curve_wire(rust::Slice<const double> pts) { return spine_wire(pts); }

// Fatia `flat` em curvas: `ends[i]` = índice final (em pontos) da curva i.
std::vector<TopoDS_Wire> curves_of(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends) {
    std::vector<TopoDS_Wire> out;
    size_t start = 0;
    for (uint32_t e : ends) {
        std::vector<double> part(flat.begin() + 3 * start, flat.begin() + 3 * e);
        out.push_back(spine_wire(rust::Slice<const double>(part.data(), part.size())));
        start = e;
    }
    return out;
}

}  // namespace

std::unique_ptr<Shape> face_copy(const Shape& s, double px, double py, double pz) {
    return guard("extrair face", [&] { return wrap(BRepBuilderAPI_Copy(face_near(s.s, gp_Pnt(px, py, pz), kFaceLimit)).Shape()); });
}

std::unique_ptr<Shape> offset_surface(const Shape& s, double d) {
    return guard("superfície deslocada", [&] {
        BRepOffsetAPI_MakeOffsetShape mk;
        mk.PerformByJoin(s.s, d, 1e-4, BRepOffset_Skin, Standard_False, Standard_False, GeomAbs_Arc);
        if (!mk.IsDone()) throw std::runtime_error("não foi possível deslocar a superfície");
        return wrap(mk.Shape());
    });
}

// join: 0 automático (arco; se falhar, interseção), 1 arco, 2 interseção (cantos vivos)
std::unique_ptr<Shape> offset_surface_j(const Shape& s, double d, uint8_t join) {
    return guard("superfície deslocada", [&] {
        auto run = [&](GeomAbs_JoinType j) -> TopoDS_Shape {
            BRepOffsetAPI_MakeOffsetShape mk;
            mk.PerformByJoin(s.s, d, 1e-4, BRepOffset_Skin, j == GeomAbs_Intersection, Standard_False, j);
            if (!mk.IsDone()) return TopoDS_Shape();
            return mk.Shape();
        };
        TopoDS_Shape r = run(join == 2 ? GeomAbs_Intersection : GeomAbs_Arc);
        if (r.IsNull() && join == 0) r = run(GeomAbs_Intersection);
        if (r.IsNull()) throw std::runtime_error("não foi possível deslocar a superfície com essa ligação");
        return wrap(r);
    });
}

std::unique_ptr<Shape> fill_surface(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends) {
    return guard("preenchimento", [&] {
        BRepOffsetAPI_MakeFilling mk;
        int n = 0;
        for (auto& w : curves_of(flat, ends))
            for (TopExp_Explorer e(w, TopAbs_EDGE); e.More(); e.Next()) {
                mk.Add(TopoDS::Edge(e.Current()), GeomAbs_C0);
                ++n;
            }
        if (n == 0) throw std::runtime_error("escolha as curvas do contorno");
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("as curvas não formam um contorno fechado");
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> extrude_curve(rust::Slice<const double> pts, double dx, double dy, double dz, double len) {
    return guard("superfície extrudada", [&] {
        if (std::abs(len) < 1e-9) throw std::runtime_error("comprimento zero");
        gp_Vec d(dx, dy, dz);
        d.Normalize();
        return wrap(BRepPrimAPI_MakePrism(curve_wire(pts), d * len).Shape());
    });
}

std::unique_ptr<Shape> revolve_curve(rust::Slice<const double> pts, double ox, double oy, double oz, double dx, double dy, double dz, double angle_deg) {
    return guard("superfície de revolução", [&] {
        gp_Ax1 ax(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz));
        if (std::abs(angle_deg) >= 360.0 - 1e-9) return wrap(BRepPrimAPI_MakeRevol(curve_wire(pts), ax).Shape());
        return wrap(BRepPrimAPI_MakeRevol(curve_wire(pts), ax, angle_deg * M_PI / 180.0).Shape());
    });
}

// Ponto mais próximo sobre a forma e a normal da face nesse ponto: [qx, qy, qz, nx, ny, nz].
rust::Vec<double> surface_near(const Shape& s, double px, double py, double pz) {
    return guard("superfície", [&] {
        TopoDS_Face f = face_near(s.s, gp_Pnt(px, py, pz));
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz)).Vertex();
        BRepExtrema_DistShapeShape d(f, v);
        if (!d.IsDone() || d.NbSolution() == 0) throw std::runtime_error("não foi possível projetar na superfície");
        gp_Pnt q = d.PointOnShape1(1);
        double u = 0, w = 0;
        if (d.SupportTypeShape1(1) == BRepExtrema_IsInFace) d.ParOnFaceS1(1, u, w);
        else {
            Handle(Geom_Surface) gs = BRep_Tool::Surface(f);
            GeomAPI_ProjectPointOnSurf pr(q, gs);
            if (pr.NbPoints() > 0) pr.LowerDistanceParameters(u, w);
        }
        gp_Pnt pp;
        gp_Vec n;
        BRepGProp_Face(f).Normal(u, w, pp, n);
        if (n.Magnitude() < 1e-12) n = gp_Vec(0, 0, 1);
        n.Normalize();
        rust::Vec<double> out;
        for (double x : {q.X(), q.Y(), q.Z(), n.X(), n.Y(), n.Z()}) out.push_back(x);
        return out;
    });
}

// Medidas da face perto do ponto: [área, tipo (0 plano, 1 cilindro, 2 cone, 3 esfera, 4 toro, 5 livre),
//  nx, ny, nz (plano: normal; cilindro/cone: eixo), ox, oy, oz (ponto do plano/eixo), raio].
rust::Vec<double> face_props(const Shape& s, double px, double py, double pz) {
    return guard("medição", [&] {
        TopoDS_Face f = face_near(s.s, gp_Pnt(px, py, pz));
        GProp_GProps g;
        BRepGProp::SurfaceProperties(f, g);
        BRepAdaptor_Surface su(f);
        double t = 5, r = 0;
        gp_Dir n(0, 0, 1);
        gp_Pnt o = g.CentreOfMass();
        switch (su.GetType()) {
            case GeomAbs_Plane:
                t = 0;
                n = su.Plane().Axis().Direction();
                if (f.Orientation() == TopAbs_REVERSED) n.Reverse();
                break;
            case GeomAbs_Cylinder:
                t = 1;
                n = su.Cylinder().Axis().Direction();
                o = su.Cylinder().Axis().Location();
                r = su.Cylinder().Radius();
                break;
            case GeomAbs_Cone:
                t = 2;
                n = su.Cone().Axis().Direction();
                o = su.Cone().Axis().Location();
                r = su.Cone().RefRadius();
                break;
            case GeomAbs_Sphere:
                t = 3;
                o = su.Sphere().Location();
                r = su.Sphere().Radius();
                break;
            case GeomAbs_Torus:
                t = 4;
                n = su.Torus().Axis().Direction();
                o = su.Torus().Axis().Location();
                r = su.Torus().MinorRadius();
                break;
            default:
                break;
        }
        rust::Vec<double> out;
        for (double x : {std::abs(g.Mass()), t, n.X(), n.Y(), n.Z(), o.X(), o.Y(), o.Z(), r}) out.push_back(x);
        return out;
    });
}

// "Até o próximo": o prisma do perfil (comprimento máximo `len`) recortado pelo corpo; fica só o
// pedaço que encosta no perfil, que termina na primeira parede do corpo.
std::unique_ptr<Shape> extrude_to_next(const Shape& face, double dx, double dy, double dz, double len, const Shape& body) {
    return guard("até o próximo", [&] {
        gp_Vec d(dx, dy, dz);
        d.Normalize();
        TopoDS_Shape prism = BRepPrimAPI_MakePrism(face.s, d * len).Shape();
        TopoDS_Shape rest = BRepAlgoAPI_Cut(prism, body.s).Shape();
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        int kept = 0;
        for (TopExp_Explorer e(rest, TopAbs_SOLID); e.More(); e.Next()) {
            BRepExtrema_DistShapeShape dist(e.Current(), face.s);
            if (dist.IsDone() && dist.Value() < 1e-4) {
                bb.Add(c, e.Current());
                ++kept;
            }
        }
        if (kept == 0) throw std::runtime_error("nada entre o perfil e o corpo nessa direção");
        return wrap(c);
    });
}

// Superfície "estendida": cada face vira a superfície-suporte com limites naturais (as infinitas,
// como plano e cilindro, ficam limitadas a ±L), para cortar a ferramenta inteira.
static TopoDS_Shape extended_faces(const TopoDS_Shape& s, double L) {
    TopoDS_Compound c;
    BRep_Builder bb;
    bb.MakeCompound(c);
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) {
        Handle(Geom_Surface) g = BRep_Tool::Surface(TopoDS::Face(e.Current()));
        if (auto t = Handle(Geom_RectangularTrimmedSurface)::DownCast(g)) g = t->BasisSurface();
        double u1, u2, v1, v2;
        g->Bounds(u1, u2, v1, v2);
        u1 = std::max(u1, -L), u2 = std::min(u2, L), v1 = std::max(v1, -L), v2 = std::min(v2, L);
        BRepBuilderAPI_MakeFace mf(g, u1, u2, v1, v2, Precision::Confusion());
        if (mf.IsDone()) bb.Add(c, mf.Face());
    }
    return c;
}

// Término "até a superfície / até o próximo / até plano" genérico: a ferramenta longa é dividida pela
// superfície (estendida se `extend`) e fica só o pedaço que passa pelo ponto inicial (perfil, boca do
// furo). Vale para extrusão, recorte, furo e rosca.
std::unique_ptr<Shape> clip_until(const Shape& tool, const Shape& cutter, double px, double py, double pz, bool extend) {
    return guard("até a superfície", [&] {
        Bnd_Box bx;
        BRepBndLib::Add(tool.s, bx);
        BRepBndLib::Add(cutter.s, bx);
        double L = std::sqrt(bx.SquareExtent()) * 2 + 10;
        TopoDS_Shape cut = extend ? extended_faces(cutter.s, L) : cutter.s;
        BRepAlgoAPI_Splitter sp;
        TopTools_ListOfShape args, tools;
        args.Append(tool.s);
        tools.Append(cut);
        sp.SetArguments(args);
        sp.SetTools(tools);
        sp.Build();
        if (!sp.IsDone()) throw std::runtime_error("não foi possível dividir pela superfície");
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz));
        std::vector<std::pair<double, TopoDS_Shape>> parts;
        for (TopExp_Explorer e(sp.Shape(), TopAbs_SOLID); e.More(); e.Next()) {
            BRepExtrema_DistShapeShape d(e.Current(), v);
            parts.push_back({d.IsDone() ? d.Value() : 1e30, e.Current()});
        }
        if (parts.size() < 2) throw std::runtime_error("a superfície não atravessa a ferramenta nessa direção (inverta ou escolha outra)");
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        double best = 1e30;
        for (auto& p : parts) best = std::min(best, p.first);
        for (auto& p : parts)
            if (p.first < best + 1e-4) bb.Add(c, p.second);
        return wrap(c);
    });
}

// Face plana quadrada (lado 2·half) centrada em o com normal n (plano como superfície de corte).
std::unique_ptr<Shape> plane_face(double ox, double oy, double oz, double nx, double ny, double nz, double half) {
    return guard("plano", [&] {
        gp_Pln pl(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz));
        return wrap(BRepBuilderAPI_MakeFace(pl, -half, half, -half, half).Face());
    });
}

// Face mais próxima tem que ter um ponto a no máximo `tol` (usado para alternar seleção de faces).
bool same_face(const Shape& s, double ax, double ay, double az, double bx, double by, double bz) {
    return guard("faces", [&] { return face_near(s.s, gp_Pnt(ax, ay, az)).IsSame(face_near(s.s, gp_Pnt(bx, by, bz))); });
}

// Filete que consome a face do meio (tritangente / "completo"): raio = metade da largura entre as
// faces laterais, aplicado nas duas arestas que a face do meio divide com elas.
std::unique_ptr<Shape> full_round_t(const Shape& s, double mx, double my, double mz, double width, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("filete completo", [&] {
        TopoDS_Face mid = face_near(s.s, gp_Pnt(mx, my, mz), kFaceLimit);
        BRepFilletAPI_MakeFillet mk(s.s);
        int n = 0;
        for (TopExp_Explorer e(mid, TopAbs_EDGE); e.More(); e.Next()) {
            const TopoDS_Edge& ed = TopoDS::Edge(e.Current());
            if (BRep_Tool::Degenerated(ed)) continue;
            mk.Add(width / 2 * 0.995, ed);
            ++n;
        }
        if (n == 0) throw std::runtime_error("a face do meio não tem arestas");
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("não foi possível arredondar a face inteira (confira a largura)");
        return tagged_result(mk, s.s, tags, out);
    });
}

// Filete face-face: arredonda as arestas comuns às duas faces escolhidas.
std::unique_ptr<Shape> face_face_fillet_t(const Shape& s, double ax, double ay, double az, double bx, double by, double bz, double r,
                                          rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("filete face-face", [&] {
        TopoDS_Face fa = face_near(s.s, gp_Pnt(ax, ay, az), kFaceLimit), fb = face_near(s.s, gp_Pnt(bx, by, bz), kFaceLimit);
        TopTools_IndexedMapOfShape ea, eb;
        TopExp::MapShapes(fa, TopAbs_EDGE, ea);
        TopExp::MapShapes(fb, TopAbs_EDGE, eb);
        BRepFilletAPI_MakeFillet mk(s.s);
        int n = 0;
        for (int i = 1; i <= ea.Extent(); ++i)
            if (eb.Contains(ea(i))) {
                mk.Add(r, TopoDS::Edge(ea(i)));
                ++n;
            }
        if (n == 0) throw std::runtime_error("as duas faces não se encontram numa aresta");
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("raio grande demais entre as faces");
        return tagged_result(mk, s.s, tags, out);
    });
}

}  // namespace fenkra
