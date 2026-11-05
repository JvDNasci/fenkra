// SPDX-License-Identifier: LGPL-2.1-or-later
// Superfícies de referência, consultas (medição) e término "até o próximo".
// Incluído no fim de kernel.cpp, depois de features.inc (usa face_near, spine_wire, P3).

#include <TColStd_Array1OfInteger.hxx>
#include <TColStd_Array1OfReal.hxx>
#include <TColgp_Array2OfPnt.hxx>
#include <BRepTopAdaptor_FClass2d.hxx>

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

// Distância de um ponto à polilinha (n pontos achatados).
static double dist_to_polyline(const gp_Pnt& p, const double* v, size_t n) {
    double best = 1e300;
    for (size_t i = 0; i + 1 < n; ++i) {
        gp_Pnt a(v[3 * i], v[3 * i + 1], v[3 * i + 2]), b(v[3 * i + 3], v[3 * i + 4], v[3 * i + 5]);
        gp_Vec ab(a, b);
        double l2 = ab.SquareMagnitude();
        double t = l2 > 1e-24 ? std::clamp(gp_Vec(a, p).Dot(ab) / l2, 0.0, 1.0) : 0.0;
        best = std::min(best, a.Translated(ab * t).Distance(p));
    }
    return best;
}

// Arestas de `support` que ficam sobre a polilinha, cada uma com a face vizinha. Só vale se elas
// cobrirem a curva inteira: pedaço de aresta não é contorno, e o MakeFilling precisa da aresta real
// (é nela que está a curva paramétrica sobre a face, de onde sai a tangente da condição G1/G2).
static std::vector<std::pair<TopoDS_Edge, TopoDS_Face>> support_edges(const TopoDS_Shape& support, const double* v, size_t n, double tol) {
    std::vector<std::pair<TopoDS_Edge, TopoDS_Face>> out;
    if (support.IsNull()) return out;
    TopTools_IndexedDataMapOfShapeListOfShape e2f;
    TopExp::MapShapesAndAncestors(support, TopAbs_EDGE, TopAbs_FACE, e2f);
    double covered = 0, len = 0;
    for (size_t i = 0; i + 1 < n; ++i)
        len += gp_Pnt(v[3 * i], v[3 * i + 1], v[3 * i + 2]).Distance(gp_Pnt(v[3 * i + 3], v[3 * i + 4], v[3 * i + 5]));
    for (int i = 1; i <= e2f.Extent(); ++i) {
        const TopoDS_Edge& e = TopoDS::Edge(e2f.FindKey(i));
        if (BRep_Tool::Degenerated(e) || e2f(i).IsEmpty()) continue;
        BRepAdaptor_Curve c(e);
        const double a = c.FirstParameter(), b = c.LastParameter();
        bool on = true;
        for (double t : {0.0, 0.25, 0.5, 0.75, 1.0}) on = on && dist_to_polyline(c.Value(a + (b - a) * t), v, n) < tol;
        if (!on) continue;
        GProp_GProps g;
        BRepGProp::LinearProperties(e, g);
        covered += g.Mass();
        out.push_back({e, TopoDS::Face(e2f(i).First())});
    }
    if (covered < 0.98 * len) out.clear();
    return out;
}

// Preenchimento com condição por aresta. `orders[i]` é a continuidade da curva i (0 G0, 1 G1, 2 G2),
// `weights[i]` o alcance da tangente (1 = neutro) e `support` as faces de apoio. `pts` são pontos de
// passagem. `errs` devolve o erro medido pelo construtor: [G0 em mm, G1 em rad].
std::unique_ptr<Shape> fill_surface_x(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends, rust::Slice<const int32_t> orders, rust::Slice<const double> weights,
                                      const Shape& support, rust::Slice<const double> pts, rust::Vec<double>& errs) {
    return guard("preenchimento", [&] {
        bool tangent = false;
        for (int32_t o : orders) tangent = tangent || o > 0;
        // a tolerância angular padrão (0,01 rad) deixa passar 0,57 grau; G1 de verdade pede menos
        BRepOffsetAPI_MakeFilling mk(3, tangent ? 30 : 15, tangent ? 4 : 2, Standard_False, 1e-5, 1e-4, tangent ? 2e-3 : 1e-2, 0.1, 8, tangent ? 20 : 9);
        gp_XYZ sum(0, 0, 0);
        for (size_t i = 0; i + 2 < flat.size(); i += 3) sum += gp_XYZ(flat[i], flat[i + 1], flat[i + 2]);
        const gp_Pnt centre(sum / std::max<double>(1.0, flat.size() / 3.0));
        std::vector<gp_Pnt> reach;
        size_t start = 0;
        int n = 0;
        for (size_t k = 0; k < ends.size(); ++k) {
            const size_t e = ends[k];
            const double* v = flat.data() + 3 * start;
            const int order = k < orders.size() ? orders[k] : 0;
            const double w = k < weights.size() ? weights[k] : 1.0;
            if (order > 0) {
                // a polilinha da curva vem da malha ou de amostragem com flecha de até 0,1 mm
                auto edges = support_edges(support.s, v, e - start, 0.15);
                if (edges.empty()) throw std::runtime_error("a curva " + std::to_string(k + 1) + " não é borda da face de apoio: G1 e G2 precisam de uma face vizinha");
                for (auto& [ed, face] : edges) {
                    // o preenchimento converte o enum em número de derivadas: GeomAbs_G2 vale 3 e é
                    // recusado, e quem vale 2 é GeomAbs_C1. A condição aplicada é a de curvatura.
                    mk.Add(ed, face, order >= 2 ? GeomAbs_C1 : GeomAbs_G1);
                    ++n;
                    if (w <= 1.0 + 1e-9) continue;
                    // peso: a superfície passa por pontos no plano tangente da vizinha, dentro do
                    // buraco, o que obriga a tangente a valer por mais tempo antes de curvar
                    BRepAdaptor_Curve c(ed);
                    double f2, l2;
                    Handle(Geom2d_Curve) pc = BRep_Tool::CurveOnSurface(ed, face, f2, l2);
                    if (pc.IsNull()) continue;
                    for (double t : {0.2, 0.5, 0.8}) {
                        gp_Pnt p, q;
                        gp_Vec d, nr;
                        c.D1(c.FirstParameter() + (c.LastParameter() - c.FirstParameter()) * t, p, d);
                        gp_Pnt2d uv = pc->Value(f2 + (l2 - f2) * t);
                        BRepGProp_Face(face).Normal(uv.X(), uv.Y(), q, nr);
                        gp_Vec in = nr.Crossed(d);
                        if (in.Magnitude() < 1e-12) continue;
                        in.Normalize();
                        if (in.Dot(gp_Vec(p, centre)) < 0) in.Reverse();
                        reach.push_back(p.Translated(in * (w - 1.0) * 0.1 * p.Distance(centre)));
                    }
                }
            } else {
                std::vector<double> part(v, v + 3 * (e - start));
                for (TopExp_Explorer x(spine_wire(rust::Slice<const double>(part.data(), part.size())), TopAbs_EDGE); x.More(); x.Next()) {
                    mk.Add(TopoDS::Edge(x.Current()), GeomAbs_C0);
                    ++n;
                }
            }
            start = e;
        }
        if (n == 0) throw std::runtime_error("escolha as curvas do contorno");
        for (auto& p : reach) mk.Add(p);
        for (size_t i = 0; i + 2 < pts.size(); i += 3) mk.Add(gp_Pnt(pts[i], pts[i + 1], pts[i + 2]));
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("as curvas não formam um contorno fechado ou as condições não têm solução");
        errs.push_back(mk.G0Error());
        errs.push_back(tangent ? mk.G1Error() : 0.0);
        bool curvatura = false;
        for (int32_t o : orders) curvatura = curvatura || o > 1;
        errs.push_back(curvatura ? mk.G2Error() : 0.0);
        // o plano médio do preenchimento sai de autovetores, com sinal arbitrário; com aresta de apoio
        // o fio pode ficar ao contrário da superfície, e a face fica sem interior: o infinito cai
        // dentro dela e o malhador não gera nada
        TopoDS_Shape r = mk.Shape();
        if (r.ShapeType() == TopAbs_FACE) {
            TopoDS_Face f = TopoDS::Face(r.Oriented(TopAbs_FORWARD));
            BRepTopAdaptor_FClass2d dentro(f, Precision::PConfusion());
            if (dentro.PerformInfinitePoint() == TopAbs_IN) {
                TopoDS_Face g = TopoDS::Face(f.EmptyCopied());
                BRep_Builder b;
                for (TopoDS_Iterator w(f); w.More(); w.Next()) b.Add(g, w.Value().Reversed());
                r = g;
            }
        }
        return wrap(r);
    });
}

// Remendos bicúbicos: 48 números por remendo, 16 polos com o primeiro índice por fora. Cada um vira
// uma face inteira, em B-spline de um vão só, que é a forma que filete e casca tratam melhor.
std::unique_ptr<Shape> bezier_faces(rust::Slice<const double> poles) {
    return guard("remendos", [&] {
        TopoDS_Compound c;
        BRep_Builder b;
        b.MakeCompound(c);
        TColStd_Array1OfReal knots(1, 2);
        knots(1) = 0.;
        knots(2) = 1.;
        TColStd_Array1OfInteger mult(1, 2);
        mult(1) = 4;
        mult(2) = 4;
        const size_t n = poles.size() / 48;
        for (size_t k = 0; k < n; ++k) {
            TColgp_Array2OfPnt p(1, 4, 1, 4);
            for (int i = 0; i < 4; ++i)
                for (int j = 0; j < 4; ++j) {
                    const double* q = poles.data() + 48 * k + 3 * (4 * i + j);
                    p(i + 1, j + 1) = gp_Pnt(q[0], q[1], q[2]);
                }
            Handle(Geom_BSplineSurface) s = new Geom_BSplineSurface(p, knots, knots, mult, mult, 3, 3);
            BRepBuilderAPI_MakeFace mk(s, 1e-7);
            if (!mk.IsDone()) throw std::runtime_error("remendo " + std::to_string(k + 1) + " degenerado");
            b.Add(c, mk.Face());
        }
        return wrap(c);
    });
}

// Curva solta como forma (entra numa junção com faces).
std::unique_ptr<Shape> curve_shape(rust::Slice<const double> pts) {
    return guard("curva", [&] { return wrap(curve_wire(pts)); });
}

// Relatório da junção: [domínios conexos, arestas livres, maior folga fechada, maior folga aberta].
// A folga fechada vem da tolerância que a costura deu às arestas; a aberta é a distância de cada
// domínio ao vizinho mais perto (o pior caso é o que impede de virar uma peça só).
rust::Vec<double> sew_report(const Shape& s) {
    return guard("juntar", [&] {
        TopTools_IndexedMapOfShape faces;
        TopExp::MapShapes(s.s, TopAbs_FACE, faces);
        const int nf = faces.Extent();
        std::vector<int> up(nf + 1);
        for (int i = 0; i <= nf; ++i) up[i] = i;
        std::function<int(int)> root = [&](int x) { return up[x] == x ? x : up[x] = root(up[x]); };
        TopTools_IndexedDataMapOfShapeListOfShape e2f;
        TopExp::MapShapesAndAncestors(s.s, TopAbs_EDGE, TopAbs_FACE, e2f);
        double free_edges = 0, closed = 0;
        for (int i = 1; i <= e2f.Extent(); ++i) {
            const TopoDS_Edge& e = TopoDS::Edge(e2f.FindKey(i));
            if (BRep_Tool::Degenerated(e)) continue;
            TopTools_IndexedMapOfShape own;
            for (TopTools_ListIteratorOfListOfShape it(e2f(i)); it.More(); it.Next()) own.Add(it.Value());
            if (own.Extent() == 1 && !BRep_Tool::IsClosed(e, TopoDS::Face(own(1)))) free_edges += 1;
            if (own.Extent() >= 2) closed = std::max(closed, 2 * BRep_Tool::Tolerance(e));
            for (int k = 2; k <= own.Extent(); ++k) up[root(faces.FindIndex(own(1)))] = root(faces.FindIndex(own(k)));
        }
        std::map<int, TopoDS_Compound> dom;
        BRep_Builder bb;
        for (int i = 1; i <= nf; ++i) {
            int r = root(i);
            if (!dom.count(r)) bb.MakeCompound(dom[r]);
            bb.Add(dom[r], faces(i));
        }
        std::vector<TopoDS_Shape> parts;
        for (auto& d : dom) parts.push_back(d.second);
        for (TopExp_Explorer w(s.s, TopAbs_WIRE, TopAbs_FACE); w.More(); w.Next()) parts.push_back(w.Current());
        double open = 0;
        for (size_t i = 0; i < parts.size() && parts.size() > 1; ++i) {
            double closest = 1e300;
            for (size_t j = 0; j < parts.size(); ++j) {
                if (i == j) continue;
                BRepExtrema_DistShapeShape d(parts[i], parts[j]);
                if (d.IsDone()) closest = std::min(closest, d.Value());
            }
            if (closest < 1e299) open = std::max(open, closest);
        }
        rust::Vec<double> out;
        for (double x : {double(parts.size()), free_edges, closed, open}) out.push_back(x);
        return out;
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
