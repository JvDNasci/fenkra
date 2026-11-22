// SPDX-License-Identifier: LGPL-2.1-or-later
// B559: curva exata vinda da ponte (ver fenkra-kernel/src/curva.rs). Incluído em features.inc logo
// depois de spine_wire, dentro do namespace anônimo: as funções de superfície, varredura e chapa
// montam o fio por aqui.
//
// Codificação: [NaN, n, (tipo, tamanho, dados...)×n]. Tipos: 0 reta, 1 arco por 3 pontos, 2 círculo,
// 3 elipse, 4 B-spline, 5 spline pelos pontos (interpolação do kernel), 6 polilinha amostrada.
// Lista crua de pontos (sem o NaN) continua aceita e segue a regra antiga de spine_wire.

bool e_curva_codificada(rust::Slice<const double> v) { return v.size() >= 2 && std::isnan(v[0]); }

struct LeitorCurva {
    const double* p;
    size_t n, i = 0;
    double num() {
        if (i >= n) throw std::runtime_error("curva codificada incompleta");
        return p[i++];
    }
    gp_Pnt pnt() {
        const double x = num(), y = num(), z = num();
        return gp_Pnt(x, y, z);
    }
    gp_Dir dir() {
        const double x = num(), y = num(), z = num();
        gp_Vec v(x, y, z);
        if (v.Magnitude() < 1e-300) throw std::runtime_error("direção nula na curva");
        return gp_Dir(v);
    }
};

// Polilinha amostrada: arestas suaves entre os cantos vivos (> 20°). Trecho de 4 ou mais pontos
// entre cantos vira uma B-spline; trecho menor, retas. Antes um canto só bastava para a curva inteira
// virar N retas.
void arestas_da_polilinha(const std::vector<gp_Pnt>& p0, std::vector<TopoDS_Edge>& out) {
    std::vector<gp_Pnt> p;
    for (const gp_Pnt& q : p0)
        if (p.empty() || p.back().Distance(q) > 1e-9) p.push_back(q);
    const size_t n = p.size();
    if (n < 2) return;
    std::vector<size_t> quebras{0};
    for (size_t i = 1; i + 1 < n; ++i) {
        gp_Vec a(p[i - 1], p[i]), b(p[i], p[i + 1]);
        if (a.Angle(b) > 0.35) quebras.push_back(i);
    }
    quebras.push_back(n - 1);
    for (size_t k = 0; k + 1 < quebras.size(); ++k) {
        const size_t s = quebras[k], e = quebras[k + 1];
        if (e - s + 1 >= 4) {
            TColgp_Array1OfPnt arr(1, static_cast<int>(e - s + 1));
            for (size_t j = s; j <= e; ++j) arr.SetValue(static_cast<int>(j - s) + 1, p[j]);
            GeomAPI_PointsToBSpline bs(arr, 3, 8, GeomAbs_C2, 1e-4);
            if (bs.IsDone()) {
                out.push_back(BRepBuilderAPI_MakeEdge(bs.Curve()).Edge());
                continue;
            }
        }
        for (size_t j = s; j < e; ++j) out.push_back(BRepBuilderAPI_MakeEdge(p[j], p[j + 1]).Edge());
    }
}

// Sobe a continuidade nos nós internos que a curva já tem de fato (a Hermite chega com nós duplos,
// que o kernel leria como C1 mesmo quando a curva é C2).
void reduz_nos(const Handle(Geom_BSplineCurve)& c) {
    for (int i = 2; i < c->NbKnots(); ++i)
        if (c->Multiplicity(i) > 1) c->RemoveKnot(i, 1, 1e-7);
}

Handle(Geom_BSplineCurve) bspline_lida(LeitorCurva& r) {
    const int grau = static_cast<int>(r.num());
    const int np = static_cast<int>(r.num()), nk = static_cast<int>(r.num()), nw = static_cast<int>(r.num());
    if (grau < 1 || np < grau + 1 || nk != np + grau + 1) throw std::runtime_error("B-spline com polos e nós incoerentes");
    TColgp_Array1OfPnt polos(1, np);
    for (int i = 1; i <= np; ++i) polos.SetValue(i, r.pnt());
    std::vector<double> plano(nk);
    for (int i = 0; i < nk; ++i) plano[i] = r.num();
    std::vector<double> nos;
    std::vector<int> mult;
    for (double t : plano) {
        if (!nos.empty() && std::abs(t - nos.back()) < 1e-12) ++mult.back();
        else nos.push_back(t), mult.push_back(1);
    }
    TColStd_Array1OfReal k(1, static_cast<int>(nos.size()));
    TColStd_Array1OfInteger m(1, static_cast<int>(nos.size()));
    for (size_t i = 0; i < nos.size(); ++i) k.SetValue(static_cast<int>(i) + 1, nos[i]), m.SetValue(static_cast<int>(i) + 1, mult[i]);
    Handle(Geom_BSplineCurve) c;
    if (nw == np) {
        TColStd_Array1OfReal w(1, np);
        for (int i = 1; i <= np; ++i) w.SetValue(i, r.num());
        c = new Geom_BSplineCurve(polos, w, k, m, grau);
    } else {
        for (int i = 0; i < nw; ++i) r.num();
        c = new Geom_BSplineCurve(polos, k, m, grau);
    }
    reduz_nos(c);
    return c;
}

void arestas_do_trecho(LeitorCurva& r, std::vector<TopoDS_Edge>& out) {
    const int tipo = static_cast<int>(r.num());
    const size_t tam = static_cast<size_t>(r.num());
    const size_t fim = r.i + tam;
    switch (tipo) {
        case 0: {
            gp_Pnt a = r.pnt(), b = r.pnt();
            if (a.Distance(b) > 1e-9) out.push_back(BRepBuilderAPI_MakeEdge(a, b).Edge());
            break;
        }
        case 1: {
            gp_Pnt a = r.pnt(), m = r.pnt(), b = r.pnt();
            GC_MakeArcOfCircle arc(a, m, b);
            if (arc.IsDone()) out.push_back(BRepBuilderAPI_MakeEdge(arc.Value()).Edge());
            else if (a.Distance(b) > 1e-9) out.push_back(BRepBuilderAPI_MakeEdge(a, b).Edge());  // arco achatado: reta
            break;
        }
        case 2: {
            gp_Pnt c = r.pnt();
            gp_Dir n = r.dir(), x = r.dir();
            const double raio = r.num();
            gp_Vec xv(x);
            xv -= gp_Vec(n) * xv.Dot(gp_Vec(n));
            out.push_back(BRepBuilderAPI_MakeEdge(gp_Circ(gp_Ax2(c, n, gp_Dir(xv)), raio)).Edge());
            break;
        }
        case 3: {
            gp_Pnt c = r.pnt();
            gp_Dir n = r.dir();
            const double mx = r.num(), my = r.num(), mz = r.num();
            gp_Vec maior(mx, my, mz);
            const double menor = r.num();
            double t0 = r.num(), t1 = r.num();
            maior -= gp_Vec(n) * maior.Dot(gp_Vec(n));
            double ra = maior.Magnitude(), rb = menor;
            gp_Dir xd(maior);
            if (rb > ra) {
                // o kernel quer o raio maior no eixo X: gira o eixo 90° e desloca o parâmetro
                xd = n.Crossed(xd);
                std::swap(ra, rb);
                t0 -= M_PI / 2;
                t1 -= M_PI / 2;
            }
            gp_Elips el(gp_Ax2(c, n, xd), ra, rb);
            if (std::abs(t1 - t0) >= 2 * M_PI - 1e-9) {
                out.push_back(BRepBuilderAPI_MakeEdge(el).Edge());
            } else if (t1 > t0) {
                out.push_back(BRepBuilderAPI_MakeEdge(el, t0, t1).Edge());
            } else {
                TopoDS_Edge e = BRepBuilderAPI_MakeEdge(el, t1, t0).Edge();
                e.Reverse();
                out.push_back(e);
            }
            break;
        }
        case 4:
            out.push_back(BRepBuilderAPI_MakeEdge(bspline_lida(r)).Edge());
            break;
        case 5: {
            const bool fechada = r.num() > 0.5;
            std::vector<gp_Pnt> p;
            while (r.i + 3 <= fim) {
                gp_Pnt q = r.pnt();
                if (p.empty() || p.back().Distance(q) > 1e-9) p.push_back(q);
            }
            if (fechada && p.size() > 2 && p.front().Distance(p.back()) < 1e-9) p.pop_back();
            if (p.size() < 2) break;
            if (p.size() == 2) {
                out.push_back(BRepBuilderAPI_MakeEdge(p[0], p[1]).Edge());
                break;
            }
            Handle(TColgp_HArray1OfPnt) arr = new TColgp_HArray1OfPnt(1, static_cast<int>(p.size()));
            for (size_t k = 0; k < p.size(); ++k) arr->SetValue(static_cast<int>(k) + 1, p[k]);
            // a mesma interpolação do perfil do sketch (make_face): é a definição única da spline
            GeomAPI_Interpolate it(arr, fechada && p.size() >= 3 ? Standard_True : Standard_False, 1e-7);
            it.Perform();
            if (!it.IsDone()) throw std::runtime_error("a spline não pôde ser criada pelos pontos");
            out.push_back(BRepBuilderAPI_MakeEdge(it.Curve()).Edge());
            break;
        }
        case 6: {
            std::vector<gp_Pnt> p;
            while (r.i < fim) p.push_back(r.pnt());
            arestas_da_polilinha(p, out);
            break;
        }
        default:
            throw std::runtime_error("tipo de trecho de curva desconhecido");
    }
    r.i = fim;
}

// Arestas da curva, na ordem (exata ou polilinha antiga).
std::vector<TopoDS_Edge> arestas_da_curva(rust::Slice<const double> v) {
    std::vector<TopoDS_Edge> out;
    if (!e_curva_codificada(v)) {
        std::vector<gp_Pnt> p;
        for (size_t i = 0; i + 2 < v.size(); i += 3) p.push_back(gp_Pnt(v[i], v[i + 1], v[i + 2]));
        // regra antiga: suave inteira vira uma B-spline; com canto, segue por trechos
        Handle(Geom_BSplineCurve) c = p.size() >= 3 ? guia_suave(v) : Handle(Geom_BSplineCurve)();
        if (!c.IsNull()) out.push_back(BRepBuilderAPI_MakeEdge(c).Edge());
        else arestas_da_polilinha(p, out);
        return out;
    }
    LeitorCurva r{v.data(), v.size()};
    r.num();
    const int n = static_cast<int>(r.num());
    for (int k = 0; k < n; ++k) arestas_do_trecho(r, out);
    return out;
}

// Fio da curva: uma aresta exata por trecho (G0 nos cantos).
TopoDS_Wire wire_curva(rust::Slice<const double> v) {
    std::vector<TopoDS_Edge> es = arestas_da_curva(v);
    if (es.empty()) throw std::runtime_error("curva vazia ou com comprimento zero");
    BRepBuilderAPI_MakeWire w;
    for (auto& e : es) {
        w.Add(e);
        if (!w.IsDone()) throw std::runtime_error("os trechos da curva não se encadeiam pelas pontas");
    }
    return w.Wire();
}

// A curva tem algum trecho exato (não é só a polilinha amostrada).
bool curva_tem_trecho_exato(rust::Slice<const double> v) {
    if (!e_curva_codificada(v)) return false;
    LeitorCurva r{v.data(), v.size()};
    r.num();
    const int n = static_cast<int>(r.num());
    for (int k = 0; k < n; ++k) {
        const int tipo = static_cast<int>(r.num());
        const size_t tam = static_cast<size_t>(r.num());
        if (tipo != 6) return true;
        r.i += tam;
    }
    return false;
}

// O fio é G1: cada aresta começa na direção em que a anterior termina (sem canto vivo).
bool fio_tangente(const TopoDS_Wire& w, double tol = 1e-3) {
    gp_Vec antes;
    bool tem = false;
    for (BRepTools_WireExplorer x(w); x.More(); x.Next()) {
        BRepAdaptor_Curve c(x.Current());
        const bool rev = x.Current().Orientation() == TopAbs_REVERSED;
        gp_Pnt p;
        gp_Vec d0, d1;
        c.D1(rev ? c.LastParameter() : c.FirstParameter(), p, d0);
        c.D1(rev ? c.FirstParameter() : c.LastParameter(), p, d1);
        if (rev) d0.Reverse(), d1.Reverse();
        if (tem && d0.Magnitude() > 1e-12 && antes.Magnitude() > 1e-12 && antes.Angle(d0) > tol) return false;
        antes = d1;
        tem = true;
    }
    return true;
}

// Curva única do fio, se ele tem uma aresta só (para os caminhos de varredura que dividem a guia).
Handle(Geom_BSplineCurve) bspline_do_fio(const TopoDS_Wire& w) {
    TopoDS_Edge unica;
    int n = 0;
    for (TopExp_Explorer x(w, TopAbs_EDGE); x.More(); x.Next(), ++n) unica = TopoDS::Edge(x.Current());
    if (n != 1) return Handle(Geom_BSplineCurve)();
    double f, l;
    Handle(Geom_Curve) g = BRep_Tool::Curve(unica, f, l);
    if (g.IsNull()) return Handle(Geom_BSplineCurve)();
    if (g->DynamicType() == STANDARD_TYPE(Geom_Line)) return Handle(Geom_BSplineCurve)();
    Handle(Geom_BSplineCurve) b = GeomConvert::CurveToBSplineCurve(new Geom_TrimmedCurve(g, f, l));
    if (unica.Orientation() == TopAbs_REVERSED) b->Reverse();
    return b;
}

// Pontos do fio (polilinha densa), para as regras que ainda medem por amostras.
std::vector<gp_Pnt> pontos_do_fio(const TopoDS_Wire& w) {
    std::vector<gp_Pnt> out;
    for (BRepTools_WireExplorer x(w); x.More(); x.Next()) {
        BRepAdaptor_Curve c(x.Current());
        GCPnts_TangentialDeflection td(c, 0.05, 0.05, 2);
        const bool rev = x.Current().Orientation() == TopAbs_REVERSED;
        std::vector<gp_Pnt> v;
        for (int i = 1; i <= td.NbPoints(); ++i) v.push_back(td.Value(i));
        if (rev) std::reverse(v.begin(), v.end());
        for (auto& q : v)
            if (out.empty() || out.back().Distance(q) > 1e-9) out.push_back(q);
    }
    return out;
}

// Trecho exato de uma aresta, codificado como curva de um trecho. Segue o parâmetro da curva (do
// primeiro ao último), como a polilinha de edge_pts_at, para as duas andarem no mesmo sentido.
void codifica_aresta(const TopoDS_Edge& e, rust::Vec<double>& out) {
    double f, l;
    TopLoc_Location loc;
    Handle(Geom_Curve) g0 = BRep_Tool::Curve(e, loc, f, l);
    if (g0.IsNull()) throw std::runtime_error("a aresta não tem curva 3D");
    Handle(Geom_Curve) g = Handle(Geom_Curve)::DownCast(g0->Transformed(loc.Transformation()));
    while (!g.IsNull() && g->DynamicType() == STANDARD_TYPE(Geom_TrimmedCurve)) g = Handle(Geom_TrimmedCurve)::DownCast(g)->BasisCurve();
    const bool rev = false;
    gp_Pnt a = g->Value(rev ? l : f), b = g->Value(rev ? f : l), m = g->Value((f + l) / 2);
    auto push3 = [&](const gp_XYZ& p) {
        out.push_back(p.X());
        out.push_back(p.Y());
        out.push_back(p.Z());
    };
    out.push_back(std::numeric_limits<double>::quiet_NaN());
    out.push_back(1);
    GeomAdaptor_Curve ac(g, f, l);
    switch (ac.GetType()) {
        case GeomAbs_Line:
            out.push_back(0);
            out.push_back(6);
            push3(a.XYZ());
            push3(b.XYZ());
            return;
        case GeomAbs_Circle: {
            gp_Circ c = ac.Circle();
            if (std::abs(l - f - 2 * M_PI) < 1e-9) {
                gp_Dir n = c.Axis().Direction();
                if (rev) n.Reverse();
                gp_Vec x(c.Location(), a);
                out.push_back(2);
                out.push_back(10);
                push3(c.Location().XYZ());
                push3(n.XYZ());
                push3(x.XYZ());
                out.push_back(c.Radius());
            } else {
                out.push_back(1);
                out.push_back(9);
                push3(a.XYZ());
                push3(m.XYZ());
                push3(b.XYZ());
            }
            return;
        }
        case GeomAbs_Ellipse: {
            gp_Elips el = ac.Ellipse();
            gp_Dir n = el.Axis().Direction();
            gp_Vec maior = gp_Vec(el.XAxis().Direction()) * el.MajorRadius();
            double t0 = f, t1 = l;
            if (rev) n.Reverse(), t0 = -l, t1 = -f;
            out.push_back(3);
            out.push_back(12);
            push3(el.Location().XYZ());
            push3(n.XYZ());
            push3(maior.XYZ());
            out.push_back(el.MinorRadius());
            out.push_back(t0);
            out.push_back(t1);
            return;
        }
        default: {
            Handle(Geom_BSplineCurve) bs = GeomConvert::CurveToBSplineCurve(new Geom_TrimmedCurve(g, f, l));
            if (bs->IsPeriodic()) bs->SetNotPeriodic();
            if (rev) bs->Reverse();
            const int np = bs->NbPoles();
            TColStd_Array1OfReal plano(1, bs->NbPoles() + bs->Degree() + 1);
            bs->KnotSequence(plano);
            const bool rac = bs->IsRational();
            out.push_back(4);
            out.push_back(4 + 3 * np + plano.Length() + (rac ? np : 0));
            out.push_back(bs->Degree());
            out.push_back(np);
            out.push_back(plano.Length());
            out.push_back(rac ? np : 0);
            for (int i = 1; i <= np; ++i) push3(bs->Pole(i).XYZ());
            for (int i = plano.Lower(); i <= plano.Upper(); ++i) out.push_back(plano(i));
            if (rac)
                for (int i = 1; i <= np; ++i) out.push_back(bs->Weight(i));
        }
    }
}
