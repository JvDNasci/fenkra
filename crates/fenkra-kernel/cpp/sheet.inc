// SPDX-License-Identifier: LGPL-2.1-or-later
// Chapa metálica: aba de aresta (dobra + parede reta) e sua versão planificada.
// Incluído no fim de kernel.cpp, depois de hybrid.inc.

#include <Geom_Circle.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <BRepBuilderAPI_TransitionMode.hxx>
#include <BOPAlgo_ArgumentAnalyzer.hxx>
#include <BOPAlgo_CheckResult.hxx>
#include <BRepCheck_Result.hxx>
#include <BRepCheck_ListOfStatus.hxx>
#include <ShapeAnalysis_ShapeTolerance.hxx>

namespace fenkra {

namespace {

// Ponto girado em torno do eixo (origem o, direção unitária k) pelo ângulo a (rad).
gp_Pnt rot_pt(const gp_Pnt& p, const gp_Pnt& o, const gp_Dir& k, double a) {
    gp_Trsf t;
    t.SetRotation(gp_Ax1(o, k), a);
    return p.Transformed(t);
}

TopoDS_Face quad(const gp_Pnt& a, const gp_Pnt& b, const gp_Pnt& c, const gp_Pnt& d) {
    BRepBuilderAPI_MakePolygon pg(a, b, c, d, Standard_True);
    return BRepBuilderAPI_MakeFace(pg.Wire(), Standard_True).Face();
}

// B432: a varredura por um caminho em spline sai com arestas gêmeas: duas faces vizinhas (a da dobra
// e a da parede) terminam na mesma posição, até 1e-15, mas cada uma com a sua aresta, e a casca fica
// aberta (22 arestas livres no caso da nervura de asa). O volume e a malha saem certos, mas o
// `BRepCheck` reprova e o sólido derruba as booleanas seguintes. Costurar as faces pela tolerância
// do kernel junta as gêmeas; o `ShapeFix_Solid` arruma o sentido da casca.
TopoDS_Shape costura_fechada(const TopoDS_Shape& s) {
    BRepBuilderAPI_Sewing sw(1e-4);
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) sw.Add(e.Current());
    sw.Perform();
    TopExp_Explorer sh(sw.SewedShape(), TopAbs_SHELL);
    if (!sh.More()) return s;
    BRepBuilderAPI_MakeSolid ms(TopoDS::Shell(sh.Current()));
    if (!ms.IsDone()) return s;
    Handle(ShapeFix_Solid) fx = new ShapeFix_Solid(ms.Solid());
    fx->Perform();
    TopoDS_Shape r = fx->Solid();
    return BRepCheck_Analyzer(r).IsValid() ? r : s;
}

}  // namespace

// Aba na aresta reta mais próxima de p. A dobra vai para o lado da face grande que contém a
// aresta (`flip` inverte). Parâmetros: espessura t, raio interno r, ângulo (graus), comprimento
// reto depois da dobra, recuos laterais off1/off2 a partir de cada ponta. `flat`: devolve a tira
// planificada (comprimento desenvolvido = ângulo × (r + k·t) + reto). Saída `info`:
// [comprimento da aresta usado, espessura medida, nx, ny, nz (normal da face), ux, uy, uz (para fora)].
std::unique_ptr<Shape> sm_flange(const Shape& body, double px, double py, double pz, double t, double r, double angle_deg, double length,
                                 double off1, double off2, bool flip, bool flat, double k, rust::Vec<double>& info) {
    return guard("aba", [&] {
        if (t <= 0) throw std::runtime_error("espessura da chapa precisa ser positiva");
        if (r <= 0) r = 1e-3;
        gp_Pnt p(px, py, pz);
        TopTools_IndexedDataMapOfShapeListOfShape m;
        TopExp::MapShapesAndAncestors(body.s, TopAbs_EDGE, TopAbs_FACE, m);
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(p).Vertex();
        int best = 0;
        double bd = 1e300;
        for (int i = 1; i <= m.Extent(); ++i) {
            BRepExtrema_DistShapeShape d(m.FindKey(i), v);
            if (d.IsDone() && d.Value() < bd) bd = d.Value(), best = i;
        }
        if (best == 0) throw std::runtime_error("nenhuma aresta perto do ponto");
        TopoDS_Edge e = TopoDS::Edge(m.FindKey(best));
        BRepAdaptor_Curve ce(e);
        if (ce.GetType() != GeomAbs_Line) throw std::runtime_error("a aba precisa de uma aresta reta");
        const TopTools_ListOfShape& fs = m.FindFromIndex(best);
        if (fs.Extent() != 2) throw std::runtime_error("aresta sem duas faces vizinhas");
        TopoDS_Face fa = TopoDS::Face(fs.First()), fb = TopoDS::Face(fs.Last());
        if (area_of(fa) < area_of(fb)) std::swap(fa, fb);
        // ponta de uma dobra (aba sobre aba): a face plana é a ponta (espessura da chapa) e a
        // face curva é a superfície da dobra; a referência passa a ser a curva, na própria aresta
        const bool plane_a = BRepAdaptor_Surface(fa).GetType() == GeomAbs_Plane, plane_b = BRepAdaptor_Surface(fb).GetType() == GeomAbs_Plane;
        if (plane_a != plane_b && plane_a) std::swap(fa, fb);
        const bool tip = !(plane_a && plane_b);
        gp_Pnt emid = ce.Value((ce.FirstParameter() + ce.LastParameter()) / 2);
        auto normal_of = [&](const TopoDS_Face& f, bool at_edge) {
            BRepAdaptor_Surface s(f);
            double u1, u2, v1, v2;
            BRepTools::UVBounds(f, u1, u2, v1, v2);
            double uc = (u1 + u2) / 2, vc = (v1 + v2) / 2;
            if (at_edge) {
                GeomAPI_ProjectPointOnSurf pr(emid, BRep_Tool::Surface(f));
                if (!pr.IsDone() || pr.NbPoints() < 1) throw std::runtime_error("a aba não achou a superfície da dobra na aresta");
                pr.LowerDistanceParameters(uc, vc);
            } else if (s.GetType() != GeomAbs_Plane) {
                throw std::runtime_error("a aba precisa de faces planas na aresta (parede de chapa)");
            }
            gp_Pnt q;
            gp_Vec n;
            BRepGProp_Face(f).Normal(uc, vc, q, n);
            n.Normalize();
            return n;
        };
        if (tip) {
            // só vale se a face curva for cilíndrica (dobra de chapa) e a outra, a ponta plana
            if (BRepAdaptor_Surface(fa).GetType() != GeomAbs_Cylinder || !(plane_a || plane_b))
                throw std::runtime_error("a aba precisa de faces planas na aresta (parede de chapa)");
        }
        gp_Vec n = normal_of(fa, tip), u = normal_of(fb, false);
        gp_Pnt p0 = BRep_Tool::Pnt(TopExp::FirstVertex(e)), p1 = BRep_Tool::Pnt(TopExp::LastVertex(e));
        gp_Vec ev(p0, p1);
        double L = ev.Magnitude();
        ev.Normalize();
        double l = L - off1 - off2;
        if (l <= 1e-6) throw std::runtime_error("os recuos laterais são maiores que a aresta");
        gp_Pnt c0 = p0.Translated(ev * off1), c1 = p1.Translated(ev * -off2);
        if (flip) {
            // dobra para o lado oposto: a referência passa a ser a outra face da chapa
            c0.Translate(n * -t);
            c1.Translate(n * -t);
            n.Reverse();
        }
        // tira lateral (seção da chapa na aresta): da face de referência até a oposta
        gp_Pnt b0 = c0.Translated(n * -t), b1 = c1.Translated(n * -t);
        TopoDS_Face strip = quad(c0, c1, b1, b0);
        // espessura medida: quanto a face lateral desce da face de referência (independe da entrada)
        double thick = 0;
        for (TopExp_Explorer vx(fb, TopAbs_VERTEX); vx.More(); vx.Next())
            thick = std::max(thick, gp_Vec(BRep_Tool::Pnt(TopoDS::Vertex(vx.Current())), emid).Dot(flip ? -n : n));
        for (double x : {l, thick, n.X(), n.Y(), n.Z(), u.X(), u.Y(), u.Z(), c0.X(), c0.Y(), c0.Z(), c1.X(), c1.Y(), c1.Z()}) info.push_back(x);
        double a = angle_deg * M_PI / 180.0;
        if (flat) {
            double dev = a * (r + k * t) + length;
            if (dev <= 1e-9) throw std::runtime_error("comprimento desenvolvido nulo");
            return wrap(BRepPrimAPI_MakePrism(strip, u * dev).Shape());
        }
        // dobra: a tira gira em torno do eixo no lado interno (raio r a partir da face de referência)
        gp_Dir axis(u.Crossed(n));
        gp_Pnt o = c0.Translated(n * r);
        TopoDS_Shape bend = BRepPrimAPI_MakeRevol(strip, gp_Ax1(o, axis), a).Shape();
        TopoDS_Shape tool = bend;
        if (length > 1e-9) {
            TopoDS_Face end = quad(rot_pt(c0, o, axis, a), rot_pt(c1, o, axis, a), rot_pt(b1, o, axis, a), rot_pt(b0, o, axis, a));
            gp_Vec d = u;
            gp_Trsf rt;
            rt.SetRotation(gp_Ax1(o, axis), a);
            d.Transform(rt);
            tool = BRepAlgoAPI_Fuse(bend, BRepPrimAPI_MakePrism(end, d * length).Shape()).Shape();
        }
        return wrap(tool);
    });
}

// Varre a seção `face` ao longo de uma cadeia plana de pontos (arestas retas e curvas em sequência,
// tangentes entre si). A polilinha é segmentada: vértices com desvio de direção menor que ~0,5°
// são reta; um trecho de vértices com desvio maior é a curva (arco, se os pontos caem num círculo,
// senão spline com as tangentes das retas vizinhas). O caminho vira um fio de arestas exatas, e a
// varredura usa a normal da chapa como binormal fixa, o que mantém a seção em prumo mesmo nas
// retas (a spline única por todos os pontos oscilava na emenda reta-curva e arruinava o sólido).
std::unique_ptr<Shape> sm_chain_sweep(const Shape& face, rust::Slice<const double> pts, double nx, double ny, double nz) {
    return guard("aba em cadeia", [&] {
        // B559: cadeia exata (arestas do modelo, curva do sketch): o fio vem pronto, uma aresta por
        // trecho, sem refazer a curva a partir de amostras
        TopoDS_Wire caminho;
        bool fechada = false;
        if (e_curva_codificada(pts)) {
            caminho = wire_curva(pts);
            TopoDS_Vertex va, vb;
            TopExp::Vertices(caminho, va, vb);
            fechada = BRep_Tool::Pnt(va).Distance(BRep_Tool::Pnt(vb)) < 1e-6;
        } else {
        std::vector<gp_Pnt> p;
        for (size_t i = 0; i + 2 < pts.size(); i += 3) {
            gp_Pnt q(pts[i], pts[i + 1], pts[i + 2]);
            if (p.empty() || p.back().Distance(q) > 1e-6) p.push_back(q);
        }
        const int n = static_cast<int>(p.size());
        if (n < 2) throw std::runtime_error("cadeia com menos de dois pontos");
        std::vector<char> curved(n, 0);
        for (int i = 1; i + 1 < n; ++i) curved[i] = gp_Vec(p[i - 1], p[i]).Angle(gp_Vec(p[i], p[i + 1])) > 0.0087;
        BRepBuilderAPI_MakeWire w;
        auto add_line = [&](const gp_Pnt& a, const gp_Pnt& b) {
            if (a.Distance(b) > 1e-6) w.Add(BRepBuilderAPI_MakeEdge(a, b).Edge());
        };
        int i = 0;  // ponto onde o próximo trecho começa
        while (i + 1 < n) {
            int c0 = i + 1;
            while (c0 < n - 1 && !curved[c0]) ++c0;
            if (c0 >= n - 1) {  // só retas até o fim
                add_line(p[i], p[n - 1]);
                break;
            }
            int c1 = c0;
            while (c1 + 1 < n - 1 && curved[c1 + 1]) ++c1;
            if (c1 == c0) {  // quina isolada: duas retas se encontram
                add_line(p[i], p[c0]);
                i = c0;
                continue;
            }
            add_line(p[i], p[c0]);
            // curva de p[c0] a p[c1]
            bool ok = false;
            const int m = (c0 + c1) / 2;
            GC_MakeArcOfCircle arc(p[c0], p[m], p[c1]);
            if (arc.IsDone()) {
                Handle(Geom_TrimmedCurve) tc = arc.Value();
                Handle(Geom_Circle) circ = Handle(Geom_Circle)::DownCast(tc->BasisCurve());
                if (!circ.IsNull()) {
                    double rad = circ->Radius(), worst = 0;
                    for (int j = c0; j <= c1; ++j) worst = std::max(worst, std::abs(circ->Circ().Distance(p[j])));
                    ok = worst < std::max(1e-3, rad * 2e-3);
                }
                if (ok) w.Add(BRepBuilderAPI_MakeEdge(tc).Edge());
            }
            if (!ok) {
                Handle(TColgp_HArray1OfPnt) arr = new TColgp_HArray1OfPnt(1, c1 - c0 + 1);
                for (int j = c0; j <= c1; ++j) arr->SetValue(j - c0 + 1, p[j]);
                GeomAPI_Interpolate ip(arr, Standard_False, 1e-6);
                ip.Perform();
                if (!ip.IsDone()) throw std::runtime_error("não foi possível ajustar a curva da aresta");
                w.Add(BRepBuilderAPI_MakeEdge(ip.Curve()).Edge());
            }
            i = c1;
        }
        if (!w.IsDone()) throw std::runtime_error("cadeia de arestas inválida");
        caminho = w.Wire();
        fechada = p.front().Distance(p.back()) < 1e-6;
        }
        BRepOffsetAPI_MakePipeShell ps(caminho);
        ps.SetMode(gp_Dir(nx, ny, nz));
        ps.SetTransitionMode(BRepBuilderAPI_RoundCorner);
        // paredes retas de uma cadeia fechada saíam com 0,5 mm de ondulação com a tolerância padrão
        // (só na cadeia fechada: numa spline aberta a tolerância apertada fazia a varredura falhar, B529)
        if (fechada) {
            ps.SetTolerance(1e-6, 1e-6, 1e-4);
            ps.SetMaxDegree(8);
            ps.SetMaxSegments(300);
        }
        TopoDS_Wire prof;
        for (TopExp_Explorer ex(face.s, TopAbs_WIRE); ex.More(); ex.Next()) prof = TopoDS::Wire(ex.Current());
        if (prof.IsNull()) throw std::runtime_error("seção da aba sem contorno");
        ps.Add(prof);
        ps.Build();
        if (!ps.IsDone() || !ps.MakeSolid()) throw std::runtime_error("não foi possível varrer a seção ao longo da cadeia");
        TopoDS_Shape s = ps.Shape();
        if (!BRepCheck_Analyzer(s).IsValid()) s = costura_fechada(s);
        GProp_GProps props;
        BRepGProp::VolumeProperties(s, props);
        if (props.Mass() < 0) s.Reverse();
        return wrap(s);
    });
}

// Perfil plano deslocado para fora (d > 0) ou para dentro (d < 0).
std::unique_ptr<Shape> offset_face(const Shape& face, double d) {
    return guard("deslocar perfil", [&] {
        TopoDS_Shape result;
        for (TopExp_Explorer fe(face.s, TopAbs_FACE); fe.More(); fe.Next()) {
            BRepOffsetAPI_MakeOffset mo(TopoDS::Face(fe.Current()), GeomAbs_Arc);
            mo.Perform(d);
            if (!mo.IsDone()) throw std::runtime_error("não foi possível deslocar o perfil");
            TopoDS_Shape f = face_from_wires(mo.Shape());
            result = result.IsNull() ? f : BRepAlgoAPI_Fuse(result, f).Shape();
        }
        if (result.IsNull()) throw std::runtime_error("perfil vazio");
        return wrap(result);
    });
}

// Caixa com canto em o, eixo x (comprimento lx), normal z (lz); y = z × x (ly).
std::unique_ptr<Shape> box_tool(double ox, double oy, double oz, double xx, double xy, double xz, double zx, double zy, double zz, double lx, double ly, double lz) {
    return guard("caixa", [&] {
        gp_Ax2 ax(gp_Pnt(ox, oy, oz), gp_Dir(zx, zy, zz), gp_Dir(xx, xy, xz));
        return wrap(BRepPrimAPI_MakeBox(ax, lx, ly, lz).Shape());
    });
}

// Prisma de um polígono plano (pontos 3D achatados) ao longo do vetor (dx, dy, dz).
std::unique_ptr<Shape> poly_prism(rust::Slice<const double> pts, double dx, double dy, double dz) {
    return guard("prisma", [&] {
        BRepBuilderAPI_MakePolygon pg;
        for (size_t i = 0; i + 2 < pts.size(); i += 3) pg.Add(gp_Pnt(pts[i], pts[i + 1], pts[i + 2]));
        pg.Close();
        TopoDS_Face f = BRepBuilderAPI_MakeFace(pg.Wire(), Standard_True).Face();
        return wrap(BRepPrimAPI_MakePrism(f, gp_Vec(dx, dy, dz)).Shape());
    });
}

// B566: curva exata de um trecho só aparada entre os pontos dela mais perto de (a) e de (b), no
// sentido de (a) para (b). Serve para a linha de dobra da aba por superfície seguir a aresta exata da web
// só no trecho das amostras.
rust::Vec<double> curva_aparada(rust::Slice<const double> v, double ax, double ay, double az, double bx, double by, double bz) {
    return guard("aparar curva", [&] {
        TopoDS_Wire w = wire_curva(v);
        TopoDS_Edge e;
        int n = 0;
        for (TopExp_Explorer x(w, TopAbs_EDGE); x.More(); x.Next(), ++n) e = TopoDS::Edge(x.Current());
        if (n != 1) throw std::runtime_error("a curva a aparar precisa ter um trecho só");
        double f, l;
        Handle(Geom_Curve) g = BRep_Tool::Curve(e, f, l);
        if (g.IsNull()) throw std::runtime_error("trecho sem curva 3D");
        auto param = [&](const gp_Pnt& p) {
            GeomAPI_ProjectPointOnCurve pr(p, g, f, l);
            if (pr.NbPoints() == 0) throw std::runtime_error("o ponto não projeta na curva");
            return pr.LowerDistanceParameter();
        };
        double u0 = param(gp_Pnt(ax, ay, az)), u1 = param(gp_Pnt(bx, by, bz));
        if (std::abs(u1 - u0) < 1e-12) throw std::runtime_error("trecho aparado nulo");
        TopoDS_Edge t = BRepBuilderAPI_MakeEdge(g, std::min(u0, u1), std::max(u0, u1)).Edge();
        rust::Vec<double> out;
        codifica_aresta(t, out);
        return out;
    });
}

// B566: cada ponto (x, y, z em sequência) levado ao ponto mais perto da curva de um trecho só.
rust::Vec<double> curva_projeta(rust::Slice<const double> v, rust::Slice<const double> pts) {
    return guard("projetar na curva", [&] {
        TopoDS_Wire w = wire_curva(v);
        TopExp_Explorer x(w, TopAbs_EDGE);
        if (!x.More()) throw std::runtime_error("curva vazia");
        double f, l;
        Handle(Geom_Curve) g = BRep_Tool::Curve(TopoDS::Edge(x.Current()), f, l);
        if (g.IsNull()) throw std::runtime_error("trecho sem curva 3D");
        GeomAPI_ProjectPointOnCurve pr;
        pr.Init(g, f, l);
        rust::Vec<double> out;
        for (size_t i = 0; i + 2 < pts.size(); i += 3) {
            gp_Pnt p(pts[i], pts[i + 1], pts[i + 2]);
            pr.Perform(p);
            if (pr.NbPoints() > 0) p = pr.NearestPoint();
            out.push_back(p.X());
            out.push_back(p.Y());
            out.push_back(p.Z());
        }
        return out;
    });
}

// ---------------------------------------------------------------- B566: diagnóstico de fusão
namespace {
void diag_forma(std::ostringstream& o, const char* nome, const TopoDS_Shape& s) {
    BRepCheck_Analyzer an(s);
    int nf = 0, ne = 0, nv = 0, nso = 0, nsh = 0;
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) ++nf;
    for (TopExp_Explorer e(s, TopAbs_EDGE); e.More(); e.Next()) ++ne;
    for (TopExp_Explorer e(s, TopAbs_VERTEX); e.More(); e.Next()) ++nv;
    for (TopExp_Explorer e(s, TopAbs_SOLID); e.More(); e.Next()) ++nso;
    for (TopExp_Explorer e(s, TopAbs_SHELL); e.More(); e.Next()) ++nsh;
    ShapeAnalysis_ShapeTolerance st;
    GProp_GProps g;
    BRepGProp::VolumeProperties(s, g);
    o << nome << ": tipo " << int(s.ShapeType()) << " valida " << an.IsValid() << " solidos " << nso << " cascas " << nsh << " faces " << nf << " arestas " << ne
      << " vertices " << nv << " tolmax " << st.Tolerance(s, 1) << " tolmed " << st.Tolerance(s, 0) << " volume " << g.Mass() << "\n";
    // faces por tipo de superfície (código do GeomAbs_SurfaceType × quantidade)
    int tipos[12] = {0};
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) tipos[std::min(int(BRepAdaptor_Surface(TopoDS::Face(e.Current())).GetType()), 11)]++;
    o << "  tipos de superficie:";
    for (int j = 0; j < 12; ++j)
        if (tipos[j]) o << " " << j << "x" << tipos[j];
    o << "\n";
    if (an.IsValid()) return;
    int listados = 0;
    for (TopAbs_ShapeEnum t : {TopAbs_SOLID, TopAbs_SHELL, TopAbs_FACE, TopAbs_WIRE, TopAbs_EDGE, TopAbs_VERTEX}) {
        TopTools_IndexedMapOfShape m;
        TopExp::MapShapes(s, t, m);
        for (int i = 1; i <= m.Extent() && listados < 40; ++i) {
            Handle(BRepCheck_Result) r = an.Result(m(i));
            if (r.IsNull()) continue;
            std::ostringstream st2;
            for (BRepCheck_ListIteratorOfListOfStatus it(r->Status()); it.More(); it.Next())
                if (it.Value() != BRepCheck_NoError) st2 << " " << int(it.Value());
            for (r->InitContextIterator(); r->MoreShapeInContext(); r->NextShapeInContext())
                for (BRepCheck_ListIteratorOfListOfStatus it(r->StatusOnShape()); it.More(); it.Next())
                    if (it.Value() != BRepCheck_NoError) st2 << " c" << int(it.Value());
            if (st2.str().empty()) continue;
            ++listados;
            o << "  sub tipo " << int(t) << " #" << i << ":" << st2.str();
            if (t == TopAbs_FACE) o << " superficie " << int(BRepAdaptor_Surface(TopoDS::Face(m(i))).GetType());
            if (t == TopAbs_EDGE) {
                const TopoDS_Edge& e = TopoDS::Edge(m(i));
                o << " tol " << BRep_Tool::Tolerance(e) << " curva " << int(BRepAdaptor_Curve(e).GetType());
                TopoDS_Vertex va, vb;
                TopExp::Vertices(e, va, vb);
                if (!va.IsNull()) {
                    gp_Pnt p = BRep_Tool::Pnt(va);
                    o << " em (" << p.X() << ", " << p.Y() << ", " << p.Z() << ")";
                }
            }
            if (t == TopAbs_VERTEX) {
                gp_Pnt p = BRep_Tool::Pnt(TopoDS::Vertex(m(i)));
                o << " tol " << BRep_Tool::Tolerance(TopoDS::Vertex(m(i))) << " em (" << p.X() << ", " << p.Y() << ", " << p.Z() << ")";
            }
            o << "\n";
        }
    }
}
}  // namespace

// Relatório de uma fusão que falha: a análise de cada argumento (tipo, validade, tolerâncias e as
// subformas reprovadas com o código do `BRepCheck`), o analisador de argumentos da booleana e a fusão
// repetida com tolerância difusa crescente. Só para diagnóstico (testes e investigação).
rust::String diag_fusao(const Shape& a, const Shape& b) {
    std::ostringstream o;
    o.precision(6);
    try {
        diag_forma(o, "A", a.s);
        diag_forma(o, "B", b.s);
        BOPAlgo_ArgumentAnalyzer aa;
        aa.SetShape1(a.s);
        aa.SetShape2(b.s);
        aa.OperationType() = BOPAlgo_FUSE;
        aa.ArgumentTypeMode() = Standard_True;
        aa.SelfInterMode() = Standard_True;
        aa.SmallEdgeMode() = Standard_True;
        aa.RebuildFaceMode() = Standard_True;
        aa.TangentMode() = Standard_True;
        aa.MergeVertexMode() = Standard_True;
        aa.MergeEdgeMode() = Standard_True;
        aa.ContinuityMode() = Standard_True;
        aa.CurveOnSurfaceMode() = Standard_True;
        aa.Perform();
        o << "analisador: falhas " << aa.HasFaulty() << "\n";
        int n = 0;
        for (const BOPAlgo_CheckResult& c : aa.GetCheckResult()) {
            if (++n > 30) break;
            o << "  estado " << int(c.GetCheckStatus()) << " dist1 " << c.GetMaxDistance1() << " dist2 " << c.GetMaxDistance2() << " formas1 " << c.GetFaultyShapes1().Extent()
              << " formas2 " << c.GetFaultyShapes2().Extent() << "\n";
        }
        for (double fz : {0.0, 1e-4, 1e-3, 1e-2}) {
            TopTools_ListOfShape args, tools;
            args.Append(a.s);
            tools.Append(b.s);
            BRepAlgoAPI_Fuse f;
            f.SetArguments(args);
            f.SetTools(tools);
            f.SetNonDestructive(Standard_True);
            if (fz > 0) f.SetFuzzyValue(fz);
            f.Build();
            o << "fusao difusa " << fz << ": erros " << f.HasErrors() << " avisos " << f.HasWarnings();
            if (f.HasWarnings()) {
                std::ostringstream w;
                f.DumpWarnings(w);
                std::string ws = w.str();
                for (char& ch : ws)
                    if (ch == '\n') ch = '|';
                o << " [" << ws.substr(0, 400) << "]";
            }
            o << "\n";
            if (!f.HasErrors()) {
                diag_forma(o, "  resultado", f.Shape());
                diag_forma(o, "  unificado", unify(f.Shape()));
            }
        }
    } catch (const Standard_Failure& e) {
        o << "excecao " << e.GetMessageString() << "\n";
    }
    return rust::String(o.str());
}

}  // namespace fenkra
