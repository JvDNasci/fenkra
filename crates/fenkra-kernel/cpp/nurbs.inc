// SPDX-License-Identifier: LGPL-2.1-or-later
// B-rep ⇄ fluxo NURBS (B211, ADR 0090). O formato do fluxo está em crates/fenkra-3dm/src/lib.rs: é o que a
// ponte com o openNURBS escreve e lê. Aqui ele vira TopoDS com a topologia do arquivo (uma aresta por
// aresta do 3DM, com uma curva de aparo por face) e volta, depois de converter as faces para NURBS.
// Incluído por kernel.cpp.

#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_NurbsConvert.hxx>
#include <BRepLib.hxx>
#include <BRepTools.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Geom2dConvert.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom2d_TrimmedCurve.hxx>
#include <GeomConvert.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Geom_BezierCurve.hxx>
#include <Geom_BezierSurface.hxx>
#include <Geom_ConicalSurface.hxx>
#include <Geom_CylindricalSurface.hxx>
#include <Geom_Plane.hxx>
#include <Geom_RectangularTrimmedSurface.hxx>
#include <Geom_SphericalSurface.hxx>
#include <Geom_ToroidalSurface.hxx>
#include <Geom_Circle.hxx>
#include <Geom_Line.hxx>
#include <Geom_SurfaceOfLinearExtrusion.hxx>
#include <Geom_SurfaceOfRevolution.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <ShapeFix_Shape.hxx>
#include <ShapeFix_Solid.hxx>
#include <TColStd_Array1OfInteger.hxx>
#include <TColStd_Array1OfReal.hxx>
#include <TColStd_Array2OfReal.hxx>
#include <TColgp_Array1OfPnt.hxx>
#include <TColgp_Array1OfPnt2d.hxx>
#include <TColgp_Array2OfPnt.hxx>
#include <TopExp.hxx>
#include <TopTools_IndexedMapOfShape.hxx>

namespace fenkra {
namespace nurbs_fluxo {

constexpr double MARCA = 3141.0;
constexpr double VERSAO = 1.0;
constexpr double TOL_MIN = 1e-7;

struct Leitor {
    const double* p;
    size_t n, i = 0;
    double d() {
        if (i >= n) throw std::runtime_error("fluxo NURBS terminou antes da hora");
        return p[i++];
    }
    int k() { return int(std::lround(d())); }
};

// Nós completos (com as pontas repetidas) → nós distintos e multiplicidades, como o OpenCASCADE guarda.
void comprimir(const std::vector<double>& todos, std::vector<double>& nos, std::vector<int>& mult) {
    for (double x : todos) {
        if (!nos.empty() && std::fabs(x - nos.back()) <= 1e-12 * std::max(1.0, std::fabs(x))) ++mult.back();
        else nos.push_back(x), mult.push_back(1);
    }
    if (nos.size() < 2) throw std::runtime_error("vetor de nós com menos de dois valores distintos");
}

template <class A>
A arr_real(const std::vector<double>& v) {
    A a(1, int(v.size()));
    for (int i = 0; i < int(v.size()); ++i) a(i + 1) = v[size_t(i)];
    return a;
}

TColStd_Array1OfInteger arr_int(const std::vector<int>& v) {
    TColStd_Array1OfInteger a(1, int(v.size()));
    for (int i = 0; i < int(v.size()); ++i) a(i + 1) = v[size_t(i)];
    return a;
}

std::vector<double> ler_nos(Leitor& r, int n) {
    std::vector<double> v(size_t(std::max(n, 0)));
    for (auto& x : v) x = r.d();
    return v;
}

Handle(Geom_BSplineCurve) ler_nurbs3(Leitor& r) {
    const int grau = r.k(), n = r.k();
    if (grau < 1 || n < grau + 1) throw std::runtime_error("curva NURBS com grau ou polos inválidos");
    std::vector<double> nos, todos = ler_nos(r, n + grau + 1);
    std::vector<int> mult;
    comprimir(todos, nos, mult);
    TColgp_Array1OfPnt polos(1, n);
    TColStd_Array1OfReal pesos(1, n);
    bool racional = false;
    for (int i = 1; i <= n; ++i) {
        double x = r.d(), y = r.d(), z = r.d(), w = r.d();
        polos(i) = gp_Pnt(x, y, z);
        pesos(i) = w;
        if (std::fabs(w - 1.0) > 1e-14) racional = true;
    }
    auto kn = arr_real<TColStd_Array1OfReal>(nos);
    auto mu = arr_int(mult);
    return racional ? new Geom_BSplineCurve(polos, pesos, kn, mu, grau) : new Geom_BSplineCurve(polos, kn, mu, grau);
}

Handle(Geom_Curve) ler_curva3(Leitor& r) {
    const int tipo = r.k();
    if (tipo == -1) return Handle(Geom_Curve)();
    if (tipo == 0) return ler_nurbs3(r);
    if (tipo == 1) {
        double ox = r.d(), oy = r.d(), oz = r.d(), dx = r.d(), dy = r.d(), dz = r.d();
        return new Geom_Line(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz));
    }
    if (tipo == 2) {
        double cx = r.d(), cy = r.d(), cz = r.d(), nx = r.d(), ny = r.d(), nz = r.d(), xx = r.d(), xy = r.d(), xz = r.d(), raio = r.d();
        return new Geom_Circle(gp_Ax2(gp_Pnt(cx, cy, cz), gp_Dir(nx, ny, nz), gp_Dir(xx, xy, xz)), raio);
    }
    throw std::runtime_error("tipo de curva desconhecido no fluxo NURBS");
}

Handle(Geom2d_BSplineCurve) ler_curva2(Leitor& r) {
    const int grau = r.k(), n = r.k();
    if (grau < 1 || n < grau + 1) throw std::runtime_error("curva de aparo com grau ou polos inválidos");
    std::vector<double> nos, todos = ler_nos(r, n + grau + 1);
    std::vector<int> mult;
    comprimir(todos, nos, mult);
    TColgp_Array1OfPnt2d polos(1, n);
    TColStd_Array1OfReal pesos(1, n);
    bool racional = false;
    for (int i = 1; i <= n; ++i) {
        double u = r.d(), v = r.d(), w = r.d();
        polos(i) = gp_Pnt2d(u, v);
        pesos(i) = w;
        if (std::fabs(w - 1.0) > 1e-14) racional = true;
    }
    auto kn = arr_real<TColStd_Array1OfReal>(nos);
    auto mu = arr_int(mult);
    return racional ? new Geom2d_BSplineCurve(polos, pesos, kn, mu, grau) : new Geom2d_BSplineCurve(polos, kn, mu, grau);
}

// Revolução de reta ou círculo que é cilindro, cone, esfera ou toro, montada com o mesmo parâmetro (as
// curvas de aparo não mudam). O OpenCASCADE mede volume e área dessas superfícies pela fórmula; na
// revolução genérica a integração numérica erra na quarta casa. Nulo quando não é o caso.
Handle(Geom_Surface) revolucao_canonica(const Handle(Geom_Curve)& meridiano, const gp_Ax1& eixo) {
    constexpr double EPS = 1e-12;
    const gp_Dir A = eixo.Direction();
    const gp_Pnt E = eixo.Location();
    auto projecao = [&](const gp_Pnt& p) { return E.Translated(gp_Vec(A) * gp_Vec(E, p).Dot(gp_Vec(A))); };
    // posição com Y = A × X: girar X pelo ângulo u em torno do eixo leva a Y
    auto posicao = [&](const gp_Pnt& o, const gp_Dir& z, const gp_Dir& x) {
        gp_Ax3 ax(o, z, x);
        if (z.Dot(A) < 0) ax.YReverse();
        return ax;
    };
    if (auto l = Handle(Geom_Line)::DownCast(meridiano)) {
        const gp_Pnt O = l->Position().Location();
        const gp_Dir d = l->Position().Direction();
        const gp_Pnt c = projecao(O);
        const double R = c.Distance(O);
        if (R < EPS) return Handle(Geom_Surface)();
        const gp_Dir X(gp_Vec(c, O));
        if (gp_Vec(d).Crossed(gp_Vec(A)).Magnitude() < EPS) return new Geom_CylindricalSurface(posicao(c, d, X), R);
        const double dz = d.Dot(A);
        if (std::fabs(gp_Vec(d).Dot(gp_Vec(A).Crossed(gp_Vec(X)))) > EPS || std::fabs(dz) < 1e-9) return Handle(Geom_Surface)();
        const gp_Dir Z = dz > 0 ? A : A.Reversed();
        const double a = std::atan2(d.Dot(X), d.Dot(Z));
        if (std::fabs(a) < 1e-9 || std::fabs(a) > M_PI / 2 - 1e-9) return Handle(Geom_Surface)();
        return new Geom_ConicalSurface(posicao(c, Z, X), a, R);
    }
    if (auto ci = Handle(Geom_Circle)::DownCast(meridiano)) {
        const gp_Ax2 p = ci->Position();
        const gp_Dir Xc = p.XDirection(), Yc = p.YDirection();
        const double r = ci->Radius();
        // o plano do círculo contém o eixo, e o Y do círculo é o próprio eixo
        if (std::fabs(p.Direction().Dot(A)) > EPS || gp_Vec(Yc).Crossed(gp_Vec(A)).Magnitude() > EPS) return Handle(Geom_Surface)();
        const gp_Pnt C = p.Location();
        const gp_Pnt c = projecao(C);
        const double R = c.Distance(C);
        if (R < EPS) return new Geom_SphericalSurface(posicao(C, Yc, Xc), r);
        if (gp_Vec(c, C).Normalized().Dot(gp_Vec(Xc)) < 1 - EPS || !(R > r)) return Handle(Geom_Surface)();
        return new Geom_ToroidalSurface(posicao(c, Yc, Xc), R, r);
    }
    return Handle(Geom_Surface)();
}

Handle(Geom_Surface) ler_superficie(Leitor& r) {
    const int tipo = r.k();
    if (tipo == 1) {
        double ox = r.d(), oy = r.d(), oz = r.d(), dx = r.d(), dy = r.d(), dz = r.d();
        Handle(Geom_Curve) meridiano = ler_curva3(r);
        if (meridiano.IsNull()) throw std::runtime_error("superfície de revolução sem meridiano");
        const gp_Ax1 eixo(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz));
        Handle(Geom_Surface) canonica = revolucao_canonica(meridiano, eixo);
        if (!canonica.IsNull()) return canonica;
        return new Geom_SurfaceOfRevolution(meridiano, eixo);
    }
    if (tipo == 2) {
        double dx = r.d(), dy = r.d(), dz = r.d();
        Handle(Geom_Curve) base = ler_curva3(r);
        if (base.IsNull()) throw std::runtime_error("extrusão sem curva de base");
        return new Geom_SurfaceOfLinearExtrusion(base, gp_Dir(dx, dy, dz));
    }
    if (tipo != 0) throw std::runtime_error("tipo de superfície desconhecido no fluxo NURBS");
    const int gu = r.k(), gv = r.k(), nu = r.k(), nv = r.k();
    if (gu < 1 || gv < 1 || nu < gu + 1 || nv < gv + 1) throw std::runtime_error("superfície NURBS com grau ou polos inválidos");
    std::vector<double> ku, kv, tu = ler_nos(r, nu + gu + 1), tv = ler_nos(r, nv + gv + 1);
    std::vector<int> mu, mv;
    comprimir(tu, ku, mu);
    comprimir(tv, kv, mv);
    TColgp_Array2OfPnt polos(1, nu, 1, nv);
    TColStd_Array2OfReal pesos(1, nu, 1, nv);
    bool racional = false;
    for (int i = 1; i <= nu; ++i)
        for (int j = 1; j <= nv; ++j) {
            double x = r.d(), y = r.d(), z = r.d(), w = r.d();
            polos(i, j) = gp_Pnt(x, y, z);
            pesos(i, j) = w;
            if (std::fabs(w - 1.0) > 1e-14) racional = true;
        }
    auto aku = arr_real<TColStd_Array1OfReal>(ku), akv = arr_real<TColStd_Array1OfReal>(kv);
    auto amu = arr_int(mu), amv = arr_int(mv);
    if (racional) return new Geom_BSplineSurface(polos, pesos, aku, akv, amu, amv, gu, gv);
    return new Geom_BSplineSurface(polos, aku, akv, amu, amv, gu, gv);
}

// Leva o parâmetro da curva de aparo para o intervalo [f, l] da aresta, por mudança linear dos nós.
void reparametrizar(const Handle(Geom2d_BSplineCurve)& c, double f, double l) {
    const double a = c->FirstParameter(), b = c->LastParameter();
    if (!(b > a) || !(l > f)) return;
    if (std::fabs(a - f) < 1e-15 && std::fabs(b - l) < 1e-15) return;
    TColStd_Array1OfReal k(1, c->NbKnots());
    c->Knots(k);
    for (int i = k.Lower(); i <= k.Upper(); ++i) k(i) = f + (k(i) - a) * (l - f) / (b - a);
    c->SetKnots(k);
}

struct Aresta {
    TopoDS_Edge e;
    double f = 0, l = 0, tol = TOL_MIN;
    int v0 = -1;
    bool ok = false;
};

TopoDS_Shape de_fluxo(const double* p, size_t n, double tol_modelo) {
    Leitor r{p, n};
    if (r.d() != MARCA || r.d() != VERSAO) throw std::runtime_error("fluxo NURBS com marca ou versão desconhecida");
    BRep_Builder B;
    const double tol_m = std::max(tol_modelo, TOL_MIN);

    const int nv = r.k();
    std::vector<TopoDS_Vertex> V(size_t(std::max(nv, 0)));
    for (int i = 0; i < nv; ++i) {
        double x = r.d(), y = r.d(), z = r.d(), t = r.d();
        B.MakeVertex(V[size_t(i)], gp_Pnt(x, y, z), std::max(t, TOL_MIN));
    }
    auto vertice = [&](int i) -> const TopoDS_Vertex& {
        if (i < 0 || i >= nv) throw std::runtime_error("índice de vértice fora da lista no fluxo NURBS");
        return V[size_t(i)];
    };

    const int ne = r.k();
    std::vector<Aresta> E(size_t(std::max(ne, 0)));
    for (int i = 0; i < ne; ++i) {
        Aresta& a = E[size_t(i)];
        const int v0 = r.k(), v1 = r.k();
        a.tol = std::max(r.d(), TOL_MIN);
        a.v0 = v0;
        Handle(Geom_Curve) c = ler_curva3(r);
        if (c.IsNull()) continue;
        a.f = c->FirstParameter();
        a.l = c->LastParameter();
        B.MakeEdge(a.e, c, a.tol);
        B.Add(a.e, vertice(v0).Oriented(TopAbs_FORWARD));
        B.Add(a.e, vertice(v1).Oriented(TopAbs_REVERSED));
        B.Range(a.e, a.f, a.l);
        // a curva de aparo de cada face chega com outro parâmetro: o SameParameter acerta depois
        B.SameParameter(a.e, Standard_False);
        a.ok = true;
    }

    const int nf = r.k();
    TopoDS_Shell casca;
    B.MakeShell(casca);
    int faces = 0;
    for (int fi = 0; fi < nf; ++fi) {
        const bool invertida = r.k() != 0;
        Handle(Geom_Surface) S = ler_superficie(r);
        TopoDS_Face F;
        B.MakeFace(F, S, Precision::Confusion());
        std::map<int, std::vector<std::pair<Handle(Geom2d_BSplineCurve), bool>>> curvas;
        std::vector<TopoDS_Wire> lacos;
        const int nl = r.k();
        for (int li = 0; li < nl; ++li) {
            r.k();  // externo ou interno: o OpenCASCADE acha o de fora pela geometria
            const int nt = r.k();
            TopoDS_Wire W;
            B.MakeWire(W);
            for (int ti = 0; ti < nt; ++ti) {
                const int ei = r.k();
                const bool invertido = r.k() != 0;
                const int vi = r.k();
                Handle(Geom2d_BSplineCurve) c2 = ler_curva2(r);
                if (ei >= 0 && ei < ne && E[size_t(ei)].ok) {
                    const Aresta& a = E[size_t(ei)];
                    if (invertido) c2->Reverse();
                    reparametrizar(c2, a.f, a.l);
                    curvas[ei].push_back({c2, invertido});
                    B.Add(W, a.e.Oriented(invertido ? TopAbs_REVERSED : TopAbs_FORWARD));
                } else {
                    // aparo singular: aresta degenerada sobre o polo da superfície
                    int k = vi;
                    if (k < 0 && ei >= 0 && ei < ne) k = E[size_t(ei)].v0;
                    TopoDS_Vertex polo;
                    if (k >= 0 && k < nv) polo = V[size_t(k)];
                    else {
                        gp_Pnt2d q = c2->Value(c2->FirstParameter());
                        B.MakeVertex(polo, S->Value(q.X(), q.Y()), Precision::Confusion());
                    }
                    TopoDS_Edge D;
                    B.MakeEdge(D);
                    B.UpdateEdge(D, c2, F, Precision::Confusion());
                    B.Add(D, polo.Oriented(TopAbs_FORWARD));
                    B.Add(D, polo.Oriented(TopAbs_REVERSED));
                    B.Range(D, c2->FirstParameter(), c2->LastParameter());
                    B.Degenerated(D, Standard_True);
                    B.Add(W, D);
                }
            }
            lacos.push_back(W);
        }
        for (auto& [ei, lista] : curvas) {
            const Aresta& a = E[size_t(ei)];
            if (lista.size() == 1) {
                B.UpdateEdge(a.e, lista[0].first, F, a.tol);
            } else {
                // costura: a mesma aresta duas vezes na face, uma em cada sentido
                size_t ida = 0;
                while (ida < lista.size() && lista[ida].second) ++ida;
                if (ida == lista.size()) ida = 0;
                const size_t volta = ida == 0 ? 1 : 0;
                B.UpdateEdge(a.e, lista[ida].first, lista[volta].first, F, a.tol);
            }
        }
        for (const TopoDS_Wire& W : lacos) {
            TopoDS_Wire w = W;
            w.Closed(Standard_True);
            B.Add(F, w);
        }
        if (invertida) F.Reverse();
        B.Add(casca, F);
        ++faces;
    }
    if (faces == 0) throw std::runtime_error("o fluxo NURBS não tem faces");

    // a tolerância do modelo é só o teto: cada aresta fica com o desvio que de fato tem
    BRepLib::SameParameter(casca, Precision::Confusion(), Standard_False);
    Handle(ShapeFix_Shape) fix = new ShapeFix_Shape(casca);
    fix->SetPrecision(Precision::Confusion());
    fix->SetMaxTolerance(std::max(10.0 * tol_m, 1e-2));
    fix->Perform();
    TopoDS_Shape r0 = fix->Shape();

    // cascas fechadas viram sólidos, com o volume para fora
    TopoDS_Compound tudo;
    B.MakeCompound(tudo);
    int partes = 0;
    TopoDS_Shape unica;
    for (TopExp_Explorer se(r0, TopAbs_SHELL); se.More(); se.Next()) {
        TopoDS_Shape parte = se.Current();
        if (BRep_Tool::IsClosed(parte)) {
            BRepBuilderAPI_MakeSolid ms(TopoDS::Shell(parte));
            if (ms.IsDone()) {
                Handle(ShapeFix_Solid) fs = new ShapeFix_Solid(ms.Solid());
                fs->Perform();
                parte = fs->Solid();
                GProp_GProps g;
                BRepGProp::VolumeProperties(parte, g);
                if (g.Mass() < 0) parte.Reverse();
            }
        }
        B.Add(tudo, parte);
        unica = parte;
        ++partes;
    }
    if (partes == 0) return r0;
    return partes == 1 ? unica : TopoDS_Shape(tudo);
}

// ------------------------------------------------------------------ TopoDS → fluxo

void nos_completos(const TColStd_Array1OfReal& k, const TColStd_Array1OfInteger& m, std::vector<double>& f) {
    for (int i = k.Lower(); i <= k.Upper(); ++i)
        for (int j = 0; j < m(i); ++j) f.push_back(k(i));
}

void escrever_nurbs3(Handle(Geom_BSplineCurve) c, std::vector<double>& f) {
    if (c->IsPeriodic()) {
        c = Handle(Geom_BSplineCurve)::DownCast(c->Copy());
        c->SetNotPeriodic();
    }
    f.push_back(0);
    f.push_back(c->Degree());
    f.push_back(c->NbPoles());
    TColStd_Array1OfReal k(1, c->NbKnots());
    TColStd_Array1OfInteger m(1, c->NbKnots());
    c->Knots(k);
    c->Multiplicities(m);
    nos_completos(k, m, f);
    for (int i = 1; i <= c->NbPoles(); ++i) {
        const gp_Pnt q = c->Pole(i);
        f.insert(f.end(), {q.X(), q.Y(), q.Z(), c->Weight(i)});
    }
}

void escrever_curva2(Handle(Geom2d_BSplineCurve) c, std::vector<double>& f) {
    if (c->IsPeriodic()) c->SetNotPeriodic();
    f.push_back(c->Degree());
    f.push_back(c->NbPoles());
    TColStd_Array1OfReal k(1, c->NbKnots());
    TColStd_Array1OfInteger m(1, c->NbKnots());
    c->Knots(k);
    c->Multiplicities(m);
    nos_completos(k, m, f);
    for (int i = 1; i <= c->NbPoles(); ++i) {
        const gp_Pnt2d q = c->Pole(i);
        f.insert(f.end(), {q.X(), q.Y(), c->Weight(i)});
    }
}

Handle(Geom_BSplineCurve) trecho3(const Handle(Geom_Curve)& c, double f, double l) {
    Handle(Geom_BSplineCurve) b = Handle(Geom_BSplineCurve)::DownCast(c->Copy());
    if (b.IsNull()) return GeomConvert::CurveToBSplineCurve(new Geom_TrimmedCurve(c, f, l));
    if (b->IsPeriodic()) b->SetNotPeriodic();
    if (f > b->FirstParameter() + 1e-12 || l < b->LastParameter() - 1e-12) b->Segment(f, l);
    return b;
}

Handle(Geom2d_BSplineCurve) trecho2(const Handle(Geom2d_Curve)& c, double f, double l) {
    Handle(Geom2d_BSplineCurve) b = Handle(Geom2d_BSplineCurve)::DownCast(c->Copy());
    if (b.IsNull()) return Geom2dConvert::CurveToBSplineCurve(new Geom2d_TrimmedCurve(c, f, l));
    if (b->IsPeriodic()) b->SetNotPeriodic();
    if (f > b->FirstParameter() + 1e-12 || l < b->LastParameter() - 1e-12) b->Segment(f, l);
    return b;
}

void escrever_superficie(Handle(Geom_BSplineSurface) s, std::vector<double>& f) {
    if (s->IsUPeriodic() || s->IsVPeriodic()) {
        s = Handle(Geom_BSplineSurface)::DownCast(s->Copy());
        if (s->IsUPeriodic()) s->SetUNotPeriodic();
        if (s->IsVPeriodic()) s->SetVNotPeriodic();
    }
    f.push_back(0);
    f.push_back(s->UDegree());
    f.push_back(s->VDegree());
    f.push_back(s->NbUPoles());
    f.push_back(s->NbVPoles());
    TColStd_Array1OfReal ku(1, s->NbUKnots()), kv(1, s->NbVKnots());
    TColStd_Array1OfInteger mu(1, s->NbUKnots()), mv(1, s->NbVKnots());
    s->UKnots(ku);
    s->VKnots(kv);
    s->UMultiplicities(mu);
    s->VMultiplicities(mv);
    nos_completos(ku, mu, f);
    nos_completos(kv, mv, f);
    for (int i = 1; i <= s->NbUPoles(); ++i)
        for (int j = 1; j <= s->NbVPoles(); ++j) {
            const gp_Pnt q = s->Pole(i, j);
            f.insert(f.end(), {q.X(), q.Y(), q.Z(), s->Weight(i, j)});
        }
}

// Curva no tipo próprio do fluxo, com o mesmo parâmetro do OpenCASCADE (para meridiano e base de
// superfície, em que o parâmetro conta). Falso quando não há forma de mesmo parâmetro.
bool curva_nativa(Handle(Geom_Curve) c, std::vector<double>& f) {
    while (auto t = Handle(Geom_TrimmedCurve)::DownCast(c)) c = t->BasisCurve();
    if (auto l = Handle(Geom_Line)::DownCast(c)) {
        const gp_Ax1 a = l->Position();
        f.insert(f.end(), {1.0, a.Location().X(), a.Location().Y(), a.Location().Z(), a.Direction().X(), a.Direction().Y(), a.Direction().Z()});
        return true;
    }
    if (auto ci = Handle(Geom_Circle)::DownCast(c)) {
        const gp_Ax2 a = ci->Position();
        f.insert(f.end(), {2.0, a.Location().X(), a.Location().Y(), a.Location().Z(), a.Direction().X(), a.Direction().Y(), a.Direction().Z(), a.XDirection().X(), a.XDirection().Y(), a.XDirection().Z(), ci->Radius()});
        return true;
    }
    if (auto b = Handle(Geom_BSplineCurve)::DownCast(c)) {
        escrever_nurbs3(b, f);
        return true;
    }
    if (auto z = Handle(Geom_BezierCurve)::DownCast(c)) {
        escrever_nurbs3(GeomConvert::CurveToBSplineCurve(z), f);
        return true;
    }
    return false;
}

// Meridiano em círculo no plano de X e Z da posição: P(v) = centro + raio (cos v X + sin v Z).
void meridiano_circulo(const gp_Pnt& centro, const gp_Dir& x, const gp_Dir& z, double raio, std::vector<double>& f) {
    const gp_Dir n = x.Crossed(z);
    f.insert(f.end(), {2.0, centro.X(), centro.Y(), centro.Z(), n.X(), n.Y(), n.Z(), x.X(), x.Y(), x.Z(), raio});
}

void eixo_de_revolucao(const gp_Ax3& p, std::vector<double>& f) {
    // girar X por u leva ao Y da posição: no sistema indireto o giro é em torno de -Z
    const gp_Dir z = p.Direct() ? p.Direction() : p.Direction().Reversed();
    f.insert(f.end(), {1.0, p.Location().X(), p.Location().Y(), p.Location().Z(), z.X(), z.Y(), z.Z()});
}

// Superfície da face no tipo do fluxo com o mesmo parâmetro: as curvas de aparo seguem como estão.
// Plano vira retalho NURBS de grau 1 sobre os limites da face; cilindro, cone, esfera, toro e revolução
// viram revolução com meridiano exato (a forma NURBS racional desses tem outro parâmetro, e o volume
// medido nela sai com erro de até 1 por cento). Falso quando não há caminho de mesmo parâmetro.
bool superficie_nativa(Handle(Geom_Surface) S, const TopoDS_Face& ff, std::vector<double>& f) {
    while (auto t = Handle(Geom_RectangularTrimmedSurface)::DownCast(S)) S = t->BasisSurface();
    std::vector<double> g;
    if (auto pl = Handle(Geom_Plane)::DownCast(S)) {
        double u0, u1, v0, v1;
        BRepTools::UVBounds(ff, u0, u1, v0, v1);
        if (!(u1 > u0) || !(v1 > v0)) return false;
        g.insert(g.end(), {0.0, 1.0, 1.0, 2.0, 2.0, u0, u0, u1, u1, v0, v0, v1, v1});
        for (double u : {u0, u1})
            for (double v : {v0, v1}) {
                const gp_Pnt q = pl->Value(u, v);
                g.insert(g.end(), {q.X(), q.Y(), q.Z(), 1.0});
            }
    } else if (auto cy = Handle(Geom_CylindricalSurface)::DownCast(S)) {
        const gp_Ax3 p = cy->Position();
        eixo_de_revolucao(p, g);
        const gp_Pnt o = p.Location().Translated(cy->Radius() * gp_Vec(p.XDirection()));
        g.insert(g.end(), {1.0, o.X(), o.Y(), o.Z(), p.Direction().X(), p.Direction().Y(), p.Direction().Z()});
    } else if (auto co = Handle(Geom_ConicalSurface)::DownCast(S)) {
        const gp_Ax3 p = co->Position();
        eixo_de_revolucao(p, g);
        const gp_Pnt o = p.Location().Translated(co->RefRadius() * gp_Vec(p.XDirection()));
        const gp_Vec d = std::sin(co->SemiAngle()) * gp_Vec(p.XDirection()) + std::cos(co->SemiAngle()) * gp_Vec(p.Direction());
        g.insert(g.end(), {1.0, o.X(), o.Y(), o.Z(), d.X(), d.Y(), d.Z()});
    } else if (auto sf = Handle(Geom_SphericalSurface)::DownCast(S)) {
        const gp_Ax3 p = sf->Position();
        eixo_de_revolucao(p, g);
        meridiano_circulo(p.Location(), p.XDirection(), p.Direction(), sf->Radius(), g);
    } else if (auto to = Handle(Geom_ToroidalSurface)::DownCast(S)) {
        const gp_Ax3 p = to->Position();
        eixo_de_revolucao(p, g);
        meridiano_circulo(p.Location().Translated(to->MajorRadius() * gp_Vec(p.XDirection())), p.XDirection(), p.Direction(), to->MinorRadius(), g);
    } else if (auto rv = Handle(Geom_SurfaceOfRevolution)::DownCast(S)) {
        const gp_Ax1 a = rv->Axis();
        g.insert(g.end(), {1.0, a.Location().X(), a.Location().Y(), a.Location().Z(), a.Direction().X(), a.Direction().Y(), a.Direction().Z()});
        if (!curva_nativa(rv->BasisCurve(), g)) return false;
    } else if (auto ex = Handle(Geom_SurfaceOfLinearExtrusion)::DownCast(S)) {
        const gp_Dir d = ex->Direction();
        g.insert(g.end(), {2.0, d.X(), d.Y(), d.Z()});
        if (!curva_nativa(ex->BasisCurve(), g)) return false;
    } else if (auto bs = Handle(Geom_BSplineSurface)::DownCast(S)) {
        escrever_superficie(bs, g);
    } else if (auto bz = Handle(Geom_BezierSurface)::DownCast(S)) {
        escrever_superficie(GeomConvert::SurfaceToBSplineSurface(bz), g);
    } else {
        return false;
    }
    f.insert(f.end(), g.begin(), g.end());
    return true;
}

Handle(Geom_Surface) superficie_posta(const TopoDS_Face& ff) {
    TopLoc_Location loc;
    Handle(Geom_Surface) S = BRep_Tool::Surface(ff, loc);
    if (!S.IsNull() && !loc.IsIdentity()) S = Handle(Geom_Surface)::DownCast(S->Transformed(loc.Transformation()));
    return S;
}

std::vector<double> para_fluxo(const TopoDS_Shape& forma) {
    // só converte tudo para NURBS se alguma face não tiver caminho de mesmo parâmetro
    bool converter = false;
    for (TopExp_Explorer fx(forma, TopAbs_FACE); fx.More() && !converter; fx.Next()) {
        const TopoDS_Face ff = TopoDS::Face(fx.Current().Oriented(TopAbs_FORWARD));
        std::vector<double> lixo;
        Handle(Geom_Surface) S = superficie_posta(ff);
        converter = S.IsNull() || !superficie_nativa(S, ff, lixo);
    }
    TopoDS_Shape s = forma;
    if (converter) {
        BRepBuilderAPI_NurbsConvert conv(forma, Standard_True);
        s = conv.Shape();
    }
    TopTools_IndexedMapOfShape vm, em;
    TopExp::MapShapes(s, TopAbs_VERTEX, vm);
    TopExp::MapShapes(s, TopAbs_EDGE, em);
    std::vector<double> f{MARCA, VERSAO};
    f.push_back(vm.Extent());
    for (int i = 1; i <= vm.Extent(); ++i) {
        const TopoDS_Vertex& v = TopoDS::Vertex(vm(i));
        const gp_Pnt q = BRep_Tool::Pnt(v);
        f.insert(f.end(), {q.X(), q.Y(), q.Z(), BRep_Tool::Tolerance(v)});
    }
    f.push_back(em.Extent());
    for (int i = 1; i <= em.Extent(); ++i) {
        const TopoDS_Edge e = TopoDS::Edge(em(i).Oriented(TopAbs_FORWARD));
        f.push_back(vm.FindIndex(TopExp::FirstVertex(e)) - 1);
        f.push_back(vm.FindIndex(TopExp::LastVertex(e)) - 1);
        f.push_back(BRep_Tool::Tolerance(e));
        TopLoc_Location loc;
        double a = 0, b = 0;
        Handle(Geom_Curve) c = BRep_Tool::Degenerated(e) ? Handle(Geom_Curve)() : BRep_Tool::Curve(e, loc, a, b);
        if (c.IsNull()) {
            f.push_back(-1);
            continue;
        }
        if (!loc.IsIdentity()) c = Handle(Geom_Curve)::DownCast(c->Transformed(loc.Transformation()));
        escrever_nurbs3(trecho3(c, a, b), f);
    }
    std::vector<double> faces;
    int nf = 0;
    for (TopExp_Explorer fx(s, TopAbs_FACE); fx.More(); fx.Next()) {
        const TopoDS_Face fc = TopoDS::Face(fx.Current());
        const TopoDS_Face ff = TopoDS::Face(fc.Oriented(TopAbs_FORWARD));
        Handle(Geom_Surface) S = superficie_posta(ff);
        if (S.IsNull()) continue;
        faces.push_back(fc.Orientation() == TopAbs_REVERSED ? 1.0 : 0.0);
        if (!superficie_nativa(S, ff, faces)) throw std::runtime_error("face sem superfície de mesmo parâmetro depois da conversão para NURBS");
        const TopoDS_Wire externo = BRepTools::OuterWire(ff);
        std::vector<TopoDS_Wire> lacos;
        for (TopExp_Explorer wx(ff, TopAbs_WIRE); wx.More(); wx.Next()) lacos.push_back(TopoDS::Wire(wx.Current()));
        faces.push_back(double(lacos.size()));
        for (const TopoDS_Wire& w : lacos) {
            faces.push_back(w.IsSame(externo) ? 1.0 : 0.0);
            std::vector<TopoDS_Edge> arestas;
            for (BRepTools_WireExplorer we(w, ff); we.More(); we.Next()) arestas.push_back(we.Current());
            faces.push_back(double(arestas.size()));
            for (const TopoDS_Edge& e : arestas) {
                const bool degenerada = BRep_Tool::Degenerated(e);
                const bool invertida = e.Orientation() == TopAbs_REVERSED;
                faces.push_back(degenerada ? -1.0 : double(em.FindIndex(e) - 1));
                faces.push_back(!degenerada && invertida ? 1.0 : 0.0);
                faces.push_back(degenerada ? double(vm.FindIndex(TopExp::FirstVertex(e)) - 1) : -1.0);
                double a = 0, b = 0;
                Handle(Geom2d_Curve) pc = BRep_Tool::CurveOnSurface(e, ff, a, b);
                if (pc.IsNull()) throw std::runtime_error("aresta sem curva de aparo na face");
                Handle(Geom2d_BSplineCurve) c2 = trecho2(pc, a, b);
                // o aparo corre no sentido do laço
                if (invertida) c2->Reverse();
                escrever_curva2(c2, faces);
            }
        }
        ++nf;
    }
    f.push_back(nf);
    f.insert(f.end(), faces.begin(), faces.end());
    return f;
}

}  // namespace nurbs_fluxo

std::unique_ptr<Shape> brep_de_fluxo(rust::Slice<const double> fluxo, double tolerancia) {
    return guard("3DM → B-rep", [&] {
        auto r = std::make_unique<Shape>();
        r->s = nurbs_fluxo::de_fluxo(fluxo.data(), fluxo.size(), tolerancia);
        return r;
    });
}

rust::Vec<double> fluxo_de_brep(const Shape& s) {
    return guard("B-rep → 3DM", [&] {
        if (s.s.IsNull()) throw std::runtime_error("forma vazia");
        rust::Vec<double> v;
        for (double x : nurbs_fluxo::para_fluxo(s.s)) v.push_back(x);
        return v;
    });
}

}  // namespace fenkra

namespace fenkra {

// Volume pela integração adaptativa até o erro relativo `eps`. A integração padrão do VolumeProperties usa
// um número fixo de pontos de Gauss e, em face plana aparada por curva NURBS racional (o círculo de um
// furo vindo de 3DM, por exemplo), erra na quarta casa: 1130,84 em vez de 1130,97 num cilindro de raio 6.
double volume_preciso(const Shape& s, double eps) {
    if (s.s.IsNull()) return 0.0;
    try {
        GProp_GProps g;
        BRepGProp::VolumeProperties(s.s, g, eps);
        return g.Mass();
    } catch (const Standard_Failure&) {
        return 0.0;
    }
}

}  // namespace fenkra
