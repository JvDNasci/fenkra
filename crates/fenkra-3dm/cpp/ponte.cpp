// SPDX-License-Identifier: LGPL-2.1-or-later
// Ponte do Fenkra com o openNURBS (B211, ADR 0090): lê e grava 3DM e troca a B-rep com o Rust pelo fluxo
// de números descrito em `src/lib.rs`. Aqui não entra nada do OpenCASCADE.
#include "opennurbs_public.h"
#include "ponte.h"
#include "fenkra-3dm/src/ffi.rs.h"

#include <cmath>
#include <map>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>

namespace fenkra3dm {
namespace {

constexpr double MARCA = 3141.0;
constexpr double VERSAO_FLUXO = 1.0;
constexpr int PROFUNDIDADE_BLOCOS = 16;

void iniciar() {
    static std::once_flag uma;
    std::call_once(uma, [] { ON::Begin(); });
}

std::string utf8(const ON_wString& w) {
    ON_String s(w);
    const char* p = s.Array();
    return p ? std::string(p) : std::string();
}

ON_wString largo(const std::string& s) { return ON_wString(s.c_str()); }

std::string texto_do_log(const ON_wString& w, size_t limite = 600) {
    std::string s = utf8(w);
    if (s.size() > limite) s = s.substr(0, limite) + "...";
    return s;
}

uint32_t rgb(const ON_Color& c) { return (uint32_t(c.Red()) << 16) | (uint32_t(c.Green()) << 8) | uint32_t(c.Blue()); }
struct MenorUuid {
    bool operator()(const ON_UUID& a, const ON_UUID& b) const { return ON_UuidCompare(a, b) < 0; }
};

ON_Color cor(uint32_t c) { return ON_Color(int((c >> 16) & 255), int((c >> 8) & 255), int(c & 255)); }

// ------------------------------------------------------------------ escrita do fluxo (3DM → Rust)

using Fluxo = std::vector<double>;

void nos_completos(const ON_NurbsCurve& c, Fluxo& f) {
    const int nk = c.KnotCount();
    f.push_back(c.Knot(0));
    for (int i = 0; i < nk; ++i) f.push_back(c.Knot(i));
    f.push_back(c.Knot(nk - 1));
}

void curva3_nurbs(const ON_NurbsCurve& c, Fluxo& f) {
    f.push_back(0);
    f.push_back(c.Order() - 1);
    f.push_back(c.CVCount());
    nos_completos(c, f);
    for (int i = 0; i < c.CVCount(); ++i) {
        ON_3dPoint p;
        c.GetCV(i, p);
        f.insert(f.end(), {p.x, p.y, p.z, c.IsRational() ? c.Weight(i) : 1.0});
    }
}

// Curva com o parâmetro do OpenCASCADE ligado ao do openNURBS por t' = a·t + b. Reta e arco vão pelo tipo
// próprio (a forma NURBS do arco tem outro parâmetro); o resto só se a forma NURBS tiver o mesmo parâmetro.
bool curva3_afim(const ON_Curve* c, Fluxo& f, double& a, double& b) {
    if (!c) return false;
    const ON_Interval d = c->Domain();
    if (!(d.Length() > 0)) return false;
    if (auto lc = ON_LineCurve::Cast(c)) {
        const double L = lc->m_line.Length();
        if (!(L > 0)) return false;
        const ON_3dVector dir = lc->m_line.Direction() / L;
        f.insert(f.end(), {1.0, lc->m_line.from.x, lc->m_line.from.y, lc->m_line.from.z, dir.x, dir.y, dir.z});
        a = L / d.Length();
        b = -d[0] * a;
        return true;
    }
    if (auto ac = ON_ArcCurve::Cast(c)) {
        const ON_Arc& arc = ac->m_arc;
        const ON_Interval ang = arc.DomainRadians();
        const ON_Plane& p = arc.plane;
        f.insert(f.end(), {2.0, p.origin.x, p.origin.y, p.origin.z, p.zaxis.x, p.zaxis.y, p.zaxis.z, p.xaxis.x, p.xaxis.y, p.xaxis.z, arc.radius});
        a = ang.Length() / d.Length();
        b = ang[0] - d[0] * a;
        return true;
    }
    ON_NurbsCurve nc;
    if (c->GetNurbForm(nc) != 1) return false;
    curva3_nurbs(nc, f);
    a = 1;
    b = 0;
    return true;
}

// Leva um ponto do espaço de parâmetros da face (o do aparo) para o da superfície escrita no fluxo.
struct MapaUV {
    bool troca_face = false;  // a face é uma vista transposta da superfície
    bool troca = false;       // a superfície escrita tem u e v trocados em relação à do openNURBS
    double au = 1, bu = 0, av = 1, bv = 0;
    const ON_Surface* aproximar = nullptr;  // sem mapa afim: ponto a ponto pela forma NURBS

    void aplicar(double& s, double& t) const {
        if (troca_face) std::swap(s, t);
        if (aproximar) {
            double ns = s, nt = t;
            if (aproximar->GetNurbFormParameterFromSurfaceParameter(s, t, &ns, &nt)) s = ns, t = nt;
            return;
        }
        if (troca) std::swap(s, t);
        s = au * s + bu;
        t = av * t + bv;
    }
};

void superficie_nurbs(const ON_NurbsSurface& ns, Fluxo& f) {
    f.push_back(0);
    f.push_back(ns.Order(0) - 1);
    f.push_back(ns.Order(1) - 1);
    f.push_back(ns.CVCount(0));
    f.push_back(ns.CVCount(1));
    for (int dir = 0; dir < 2; ++dir) {
        const int nk = ns.KnotCount(dir);
        f.push_back(ns.Knot(dir, 0));
        for (int i = 0; i < nk; ++i) f.push_back(ns.Knot(dir, i));
        f.push_back(ns.Knot(dir, nk - 1));
    }
    for (int i = 0; i < ns.CVCount(0); ++i)
        for (int j = 0; j < ns.CVCount(1); ++j) {
            ON_3dPoint p;
            ns.GetCV(i, j, p);
            f.insert(f.end(), {p.x, p.y, p.z, ns.IsRational() ? ns.Weight(i, j) : 1.0});
        }
}

// Escreve a superfície da face e devolve o mapa dos aparos e se a orientação inverte. `aprox` marca a face
// sem caminho exato.
bool superficie_da_face(const ON_BrepFace& face, Fluxo& f, MapaUV& m, bool& inverte, bool& aprox) {
    const ON_Surface* s = face.SurfaceOf();
    if (!s) return false;
    m.troca_face = face.ProxySurfaceIsTransposed();
    inverte = m.troca_face;
    ON_NurbsSurface ns;
    if (s->GetNurbForm(ns) == 1) {
        superficie_nurbs(ns, f);
        return true;
    }
    if (auto rs = ON_RevSurface::Cast(s)) {
        Fluxo meridiano;
        double a = 1, b = 0;
        const double L = rs->m_axis.Length();
        if (L > 0 && rs->m_t.Length() > 0 && curva3_afim(rs->m_curve, meridiano, a, b)) {
            const ON_3dVector d = rs->m_axis.Direction() / L;
            f.insert(f.end(), {1.0, rs->m_axis.from.x, rs->m_axis.from.y, rs->m_axis.from.z, d.x, d.y, d.z});
            f.insert(f.end(), meridiano.begin(), meridiano.end());
            // parâmetro do openNURBS: (ângulo em m_t, meridiano), trocados quando a superfície é transposta
            m.troca = rs->m_bTransposed;
            m.au = rs->m_angle.Length() / rs->m_t.Length();
            m.bu = rs->m_angle[0] - rs->m_t[0] * m.au;
            m.av = a;
            m.bv = b;
            inverte = inverte != rs->m_bTransposed;
            return true;
        }
    }
    if (auto ss = ON_SumSurface::Cast(s)) {
        // soma de uma curva com uma reta: extrusão linear da curva
        for (int reta = 1; reta >= 0; --reta) {
            auto lc = ON_LineCurve::Cast(ss->m_curve[reta]);
            const ON_Curve* outra = ss->m_curve[1 - reta];
            if (!lc || !outra || !(lc->m_t.Length() > 0)) continue;
            const double L = lc->m_line.Length();
            if (!(L > 0)) continue;
            std::unique_ptr<ON_Curve> base(outra->DuplicateCurve());
            base->Translate(ON_3dVector(lc->m_line.from) + ss->m_basepoint);
            Fluxo fb;
            double a = 1, b = 0;
            if (!curva3_afim(base.get(), fb, a, b)) continue;
            const ON_3dVector d = lc->m_line.Direction() / L;
            f.insert(f.end(), {2.0, d.x, d.y, d.z});
            f.insert(f.end(), fb.begin(), fb.end());
            // (s, t) da soma: s na curva 0, t na curva 1; a base fica em u
            m.troca = reta == 0;
            m.au = a;
            m.bu = b;
            m.av = L / lc->m_t.Length();
            m.bv = -lc->m_t[0] * m.av;
            inverte = inverte != (reta == 0);
            return true;
        }
    }
    if (s->GetNurbForm(ns) == 0) return false;
    superficie_nurbs(ns, f);
    m.aproximar = s;
    aprox = true;
    return true;
}

void curva2(const ON_NurbsCurve& c, const MapaUV& m, Fluxo& f) {
    f.push_back(c.Order() - 1);
    f.push_back(c.CVCount());
    nos_completos(c, f);
    for (int i = 0; i < c.CVCount(); ++i) {
        ON_3dPoint p;
        c.GetCV(i, p);
        double u = p.x, v = p.y;
        m.aplicar(u, v);
        f.insert(f.end(), {u, v, c.IsRational() ? c.Weight(i) : 1.0});
    }
}

double tol_ou_zero(double t) { return ON_IsValid(t) && t > 0 ? t : 0.0; }

// B-rep do openNURBS → fluxo. Faces sem superfície utilizável ficam de fora e são contadas.
Fluxo brep_para_fluxo(const ON_Brep& b, bool& aprox, int& perdidas) {
    Fluxo f{MARCA, VERSAO_FLUXO};
    f.push_back(b.m_V.Count());
    for (int i = 0; i < b.m_V.Count(); ++i) {
        const ON_3dPoint p = b.m_V[i].point;
        f.insert(f.end(), {p.x, p.y, p.z, tol_ou_zero(b.m_V[i].m_tolerance)});
    }
    f.push_back(b.m_E.Count());
    for (int i = 0; i < b.m_E.Count(); ++i) {
        const ON_BrepEdge& e = b.m_E[i];
        f.insert(f.end(), {double(e.m_vi[0]), double(e.m_vi[1]), tol_ou_zero(e.m_tolerance)});
        ON_NurbsCurve nc;
        if (e.GetNurbForm(nc)) curva3_nurbs(nc, f);
        else f.push_back(-1);
    }
    Fluxo faces;
    int nf = 0;
    for (int fi = 0; fi < b.m_F.Count(); ++fi) {
        const ON_BrepFace& face = b.m_F[fi];
        Fluxo uma;
        MapaUV m;
        bool inverte = false;
        uma.push_back(0);  // a orientação, preenchida abaixo
        if (!superficie_da_face(face, uma, m, inverte, aprox)) {
            ++perdidas;
            continue;
        }
        uma[0] = (face.m_bRev != inverte) ? 1.0 : 0.0;
        std::vector<int> lacos;
        for (int k = 0; k < face.m_li.Count(); ++k) {
            const ON_BrepLoop* L = b.Loop(face.m_li[k]);
            if (L && (L->m_type == ON_BrepLoop::outer || L->m_type == ON_BrepLoop::inner || L->m_type == ON_BrepLoop::slit)) lacos.push_back(face.m_li[k]);
        }
        uma.push_back(double(lacos.size()));
        for (int li : lacos) {
            const ON_BrepLoop& L = b.m_L[li];
            uma.push_back(L.m_type == ON_BrepLoop::outer ? 1.0 : 0.0);
            uma.push_back(L.m_ti.Count());
            for (int k = 0; k < L.m_ti.Count(); ++k) {
                const ON_BrepTrim& t = b.m_T[L.m_ti[k]];
                const bool singular = t.m_type == ON_BrepTrim::singular || t.m_ei < 0;
                uma.push_back(singular ? -1.0 : double(t.m_ei));
                uma.push_back(t.m_bRev3d ? 1.0 : 0.0);
                uma.push_back(singular ? double(t.m_vi[0]) : -1.0);
                ON_NurbsCurve c2;
                if (!t.GetNurbForm(c2)) throw std::runtime_error("aparo sem forma NURBS");
                curva2(c2, m, uma);
            }
        }
        faces.insert(faces.end(), uma.begin(), uma.end());
        ++nf;
    }
    f.push_back(nf);
    f.insert(f.end(), faces.begin(), faces.end());
    return f;
}

// ------------------------------------------------------------------ leitura do fluxo (Rust → 3DM)

struct Leitor {
    const double* p;
    size_t n, i = 0;
    double d() {
        if (i >= n) throw std::runtime_error("fluxo da B-rep terminou antes da hora");
        return p[i++];
    }
    int k() { return int(std::lround(d())); }
    // três números em ordem (os argumentos de uma chamada não têm ordem de avaliação garantida)
    ON_3dPoint ponto() {
        const double x = d(), y = d(), z = d();
        return ON_3dPoint(x, y, z);
    }
    ON_3dVector vetor() { return ON_3dVector(ponto()); }
};

std::vector<double> ler_nos(Leitor& r, int n) {
    std::vector<double> v(size_t(std::max(n, 0)));
    for (auto& x : v) x = r.d();
    return v;
}

ON_NurbsCurve* ler_curva(Leitor& r, int dim) {
    const int grau = r.k(), n = r.k();
    if (grau < 1 || n < grau + 1) throw std::runtime_error("curva do fluxo com grau ou polos inválidos");
    const auto nos = ler_nos(r, n + grau + 1);
    std::vector<ON_4dPoint> polos(size_t(n), ON_4dPoint::Zero);
    bool racional = false;
    for (auto& q : polos) {
        double x = r.d(), y = r.d(), z = dim == 3 ? r.d() : 0.0, w = r.d();
        if (std::fabs(w - 1.0) > 1e-14) racional = true;
        q = ON_4dPoint(x * w, y * w, z * w, w);
    }
    ON_NurbsCurve* c = ON_NurbsCurve::New(dim, racional, grau + 1, n);
    for (int i = 0; i < n + grau - 1; ++i) c->SetKnot(i, nos[size_t(i) + 1]);
    for (int i = 0; i < n; ++i) {
        const ON_4dPoint& q = polos[size_t(i)];
        if (racional) c->SetCV(i, q);
        else c->SetCV(i, ON_3dPoint(q.x, q.y, q.z));
    }
    return c;
}

ON_NurbsSurface* ler_superficie_nurbs(Leitor& r) {
    const int gu = r.k(), gv = r.k(), nu = r.k(), nv = r.k();
    if (gu < 1 || gv < 1 || nu < gu + 1 || nv < gv + 1) throw std::runtime_error("superfície do fluxo com grau ou polos inválidos");
    const auto ku = ler_nos(r, nu + gu + 1), kv = ler_nos(r, nv + gv + 1);
    std::vector<ON_4dPoint> polos(size_t(nu) * size_t(nv), ON_4dPoint::Zero);
    bool racional = false;
    for (auto& q : polos) {
        double x = r.d(), y = r.d(), z = r.d(), w = r.d();
        if (std::fabs(w - 1.0) > 1e-14) racional = true;
        q = ON_4dPoint(x * w, y * w, z * w, w);
    }
    ON_NurbsSurface* s = ON_NurbsSurface::New(3, racional, gu + 1, gv + 1, nu, nv);
    for (int i = 0; i < nu + gu - 1; ++i) s->SetKnot(0, i, ku[size_t(i) + 1]);
    for (int i = 0; i < nv + gv - 1; ++i) s->SetKnot(1, i, kv[size_t(i) + 1]);
    for (int i = 0; i < nu; ++i)
        for (int j = 0; j < nv; ++j) {
            const ON_4dPoint& q = polos[size_t(i) * size_t(nv) + size_t(j)];
            if (racional) s->SetCV(i, j, q);
            else s->SetCV(i, j, ON_3dPoint(q.x, q.y, q.z));
        }
    return s;
}

// Curva do fluxo (reta, círculo ou NURBS) como curva do openNURBS de mesmo parâmetro, limitada a [t0, t1]
// quando é reta ou círculo (no fluxo elas são infinitas ou fechadas).
struct Curva3 {
    int tipo = -1;
    ON_3dPoint o;
    ON_3dVector d, x;
    double raio = 0;
    std::unique_ptr<ON_NurbsCurve> nurbs;

    ON_Curve* montar(double t0, double t1) {
        if (tipo == 0) return nurbs.release();
        if (!(t1 > t0)) throw std::runtime_error("faixa de parâmetros vazia para a curva da superfície");
        if (tipo == 1) {
            auto* lc = new ON_LineCurve(ON_Line(o + t0 * d, o + t1 * d));
            lc->SetDomain(t0, t1);
            return lc;
        }
        const ON_3dVector y = ON_CrossProduct(d, x);
        const ON_Circle c(ON_Plane(o, x, y), raio);
        const double fim = (t1 - t0 >= 2.0 * ON_PI - 1e-9) ? t0 + 2.0 * ON_PI : t1;
        auto* ac = new ON_ArcCurve(ON_Arc(c, ON_Interval(t0, fim)));
        ac->SetDomain(t0, fim);
        return ac;
    }
};

Curva3 ler_curva3(Leitor& r) {
    Curva3 c;
    c.tipo = r.k();
    if (c.tipo == 0) {
        c.nurbs.reset(ler_curva(r, 3));
    } else if (c.tipo == 1) {
        c.o = r.ponto();
        c.d = r.vetor();
    } else if (c.tipo == 2) {
        c.o = r.ponto();
        c.d = r.vetor();  // normal do círculo
        c.x = r.vetor();
        c.raio = r.d();
    } else {
        throw std::runtime_error("curva de superfície sem tipo conhecido no fluxo");
    }
    return c;
}

// Superfície do fluxo: NURBS, revolução (ângulo, meridiano) ou extrusão (base, distância). As duas últimas
// viram ON_RevSurface e ON_SumSurface com o mesmo parâmetro, então os aparos entram como vieram.
struct Superficie {
    int tipo = 0;
    std::unique_ptr<ON_NurbsSurface> nurbs;
    ON_3dPoint o;
    ON_3dVector d;
    Curva3 curva;

    ON_Surface* montar(const ON_BoundingBox& faixa) {
        if (tipo == 0) return nurbs.release();
        if (!faixa.IsValid()) throw std::runtime_error("face sem aparos para limitar a superfície");
        const double u0 = faixa.m_min.x, u1 = faixa.m_max.x, v0 = faixa.m_min.y, v1 = faixa.m_max.y;
        if (tipo == 1) {
            auto* rs = new ON_RevSurface();
            rs->m_curve = curva.montar(v0, v1);
            rs->m_axis = ON_Line(o, o + d);
            const double fim = (u1 - u0 > 2.0 * ON_PI) ? u0 + 2.0 * ON_PI : u1;
            rs->m_angle = ON_Interval(u0, fim);
            rs->m_t = rs->m_angle;
            rs->m_bTransposed = false;
            return rs;
        }
        auto* ss = new ON_SumSurface();
        ss->m_curve[0] = curva.montar(u0, u1);
        auto* lc = new ON_LineCurve(ON_Line(ON_3dPoint::Origin + v0 * d, ON_3dPoint::Origin + v1 * d));
        lc->SetDomain(v0, v1);
        ss->m_curve[1] = lc;
        ss->m_basepoint = ON_3dVector::ZeroVector;
        return ss;
    }
};

Superficie ler_superficie(Leitor& r) {
    Superficie s;
    s.tipo = r.k();
    if (s.tipo == 0) {
        s.nurbs.reset(ler_superficie_nurbs(r));
    } else if (s.tipo == 1) {
        s.o = r.ponto();
        s.d = r.vetor();
        s.curva = ler_curva3(r);
    } else if (s.tipo == 2) {
        s.d = r.vetor();
        s.curva = ler_curva3(r);
    } else {
        throw std::runtime_error("superfície sem tipo conhecido no fluxo");
    }
    return s;
}

std::unique_ptr<ON_Brep> brep_de_fluxo(const rust::Vec<double>& v) {
    Leitor r{v.data(), v.size()};
    if (r.d() != MARCA || r.d() != VERSAO_FLUXO) throw std::runtime_error("fluxo da B-rep com marca ou versão desconhecida");
    std::unique_ptr<ON_Brep> b(ON_Brep::New());
    const int nv = r.k();
    for (int i = 0; i < nv; ++i) {
        double x = r.d(), y = r.d(), z = r.d(), t = r.d();
        b->NewVertex(ON_3dPoint(x, y, z), t > 0 ? t : ON_UNSET_VALUE);
    }
    const int ne = r.k();
    std::vector<int> aresta(size_t(std::max(ne, 0)), -1);
    for (int i = 0; i < ne; ++i) {
        const int v0 = r.k(), v1 = r.k();
        const double tol = r.d();
        const int tipo = r.k();
        if (tipo == -1) continue;  // degenerada: vira aparo singular
        if (tipo != 0) throw std::runtime_error("a gravação do 3DM só recebe arestas NURBS");
        if (v0 < 0 || v1 < 0 || v0 >= nv || v1 >= nv) throw std::runtime_error("aresta do fluxo com vértice fora da lista");
        const int c3 = b->AddEdgeCurve(ler_curva(r, 3));
        aresta[size_t(i)] = b->NewEdge(b->m_V[v0], b->m_V[v1], c3, nullptr, tol > 0 ? tol : ON_UNSET_VALUE).m_edge_index;
    }
    const int nf = r.k();
    for (int fi = 0; fi < nf; ++fi) {
        const bool invertida = r.k() != 0;
        Superficie sup = ler_superficie(r);
        // os aparos primeiro: a revolução e a extrusão precisam da faixa de parâmetros que eles cobrem
        struct Aparo {
            int ei, vi;
            bool invertido;
            std::unique_ptr<ON_NurbsCurve> c2;
        };
        std::vector<std::pair<bool, std::vector<Aparo>>> lacos(size_t(std::max(r.k(), 0)));
        ON_BoundingBox faixa;
        for (auto& [externo, aparos] : lacos) {
            externo = r.k() != 0;
            const int nt = r.k();
            for (int ti = 0; ti < nt; ++ti) {
                Aparo a;
                a.ei = r.k();
                a.invertido = r.k() != 0;
                a.vi = r.k();
                a.c2.reset(ler_curva(r, 2));
                a.c2->GetTightBoundingBox(faixa, true);
                aparos.push_back(std::move(a));
            }
        }
        ON_Surface* srf = sup.montar(faixa);
        const int si = b->AddSurface(srf);
        const int face = b->NewFace(si).m_face_index;
        b->m_F[face].m_bRev = invertida;
        for (auto& [externo, aparos] : lacos) {
            const int laco = b->NewLoop(externo ? ON_BrepLoop::outer : ON_BrepLoop::inner, b->m_F[face]).m_loop_index;
            for (Aparo& a : aparos) {
                const ON_Surface::ISO iso = srf->IsIsoparametric(*a.c2);
                const int c2i = b->AddTrimCurve(a.c2.release());
                if (a.ei >= 0 && a.ei < ne && aresta[size_t(a.ei)] >= 0) {
                    ON_BrepTrim& t = b->NewTrim(b->m_E[aresta[size_t(a.ei)]], a.invertido, b->m_L[laco], c2i);
                    t.m_iso = iso;
                } else {
                    if (a.vi < 0 || a.vi >= nv) throw std::runtime_error("aparo singular sem vértice");
                    b->NewSingularTrim(b->m_V[a.vi], b->m_L[laco], iso, c2i);
                }
            }
        }
    }
    // o toolkit público não calcula tolerâncias (fica "sem valor", o que o IsValid recusa): valem as do
    // kernel, que vieram no fluxo; o resto (iso, tipos de aparo e laço, caixas) ele calcula
    for (int i = 0; i < b->m_V.Count(); ++i)
        if (!(b->m_V[i].m_tolerance >= 0)) b->m_V[i].m_tolerance = 0;
    for (int i = 0; i < b->m_E.Count(); ++i)
        if (!(b->m_E[i].m_tolerance >= 0)) b->m_E[i].m_tolerance = 0;
    for (int i = 0; i < b->m_T.Count(); ++i)
        for (double& t : b->m_T[i].m_tolerance)
            if (!(t >= 0)) t = 0;
    b->SetTolerancesBoxesAndFlags(false, false, false, false, true, true, true, true);
    return b;
}

// ------------------------------------------------------------------ leitura do 3DM

struct Contexto {
    const ONX_Model& modelo;
    std::map<int, int> camada_por_indice;  // índice do openNURBS → posição na lista
    ArquivoBruto& saida;
};

void cor_do_objeto(const ONX_Model& modelo, const ON_3dmObjectAttributes& at, ObjetoBruto& o) {
    const auto fonte = at.ColorSource();
    o.cor_propria = fonte == ON::color_from_object || fonte == ON::color_from_material;
    o.cor = o.cor_propria ? rgb(modelo.WireframeColorFromAttributes(at)) : 0;
}

void traduzir(Contexto& cx, const ON_Geometry* geo, const ON_3dmObjectAttributes& at, const ON_Xform& xf, int profundidade);

void expandir_bloco(Contexto& cx, const ON_InstanceRef& ref, const ON_3dmObjectAttributes& at_ref, const ON_Xform& xf, int profundidade) {
    if (profundidade > PROFUNDIDADE_BLOCOS) {
        cx.saida.ignorados.push_back("bloco aninhado além do limite");
        return;
    }
    const ON_ModelComponentReference dr = cx.modelo.ComponentFromId(ON_ModelComponent::Type::InstanceDefinition, ref.m_instance_definition_uuid);
    const ON_InstanceDefinition* def = ON_InstanceDefinition::Cast(dr.ModelComponent());
    if (!def) {
        cx.saida.ignorados.push_back("bloco sem definição");
        return;
    }
    const ON_Xform xf_ref = xf * ref.m_xform;
    const auto& ids = def->InstanceGeometryIdList();
    for (int i = 0; i < ids.Count(); ++i) {
        const ON_ModelComponentReference gr = cx.modelo.ComponentFromId(ON_ModelComponent::Type::ModelGeometry, ids[i]);
        const ON_ModelGeometryComponent* g = ON_ModelGeometryComponent::Cast(gr.ModelComponent());
        if (!g) continue;
        const ON_Geometry* geo = g->Geometry(nullptr);
        const ON_3dmObjectAttributes* at = g->Attributes(nullptr);
        if (!geo || !at) continue;
        // o que vem "do pai" herda a camada e a cor da inserção do bloco
        ON_3dmObjectAttributes herdado(*at);
        if (at->ColorSource() == ON::color_from_parent) {
            herdado.SetColorSource(at_ref.ColorSource());
            herdado.m_color = at_ref.m_color;
        }
        if (herdado.m_layer_index == 0 && at_ref.m_layer_index != 0) herdado.m_layer_index = at_ref.m_layer_index;
        if (herdado.Name().IsEmpty()) herdado.SetName(at_ref.Name(), true);
        traduzir(cx, geo, herdado, xf_ref, profundidade + 1);
    }
}

void traduzir(Contexto& cx, const ON_Geometry* geo, const ON_3dmObjectAttributes& at, const ON_Xform& xf, int profundidade) {
    if (auto ref = ON_InstanceRef::Cast(geo)) {
        expandir_bloco(cx, *ref, at, xf, profundidade);
        return;
    }
    ObjetoBruto o{};
    o.nome = rust::String(utf8(at.Name()));
    auto c = cx.camada_por_indice.find(at.m_layer_index);
    o.camada = c == cx.camada_por_indice.end() ? -1 : c->second;
    cor_do_objeto(cx.modelo, at, o);
    const bool identidade = xf.IsIdentity();

    if (auto malha = ON_Mesh::Cast(geo)) {
        ON_Mesh m(*malha);
        if (!identidade) m.Transform(xf);
        o.tipo = 1;
        for (int i = 0; i < m.VertexCount(); ++i) {
            const ON_3dPoint p = m.Vertex(i);
            o.vertices.push_back(p.x), o.vertices.push_back(p.y), o.vertices.push_back(p.z);
        }
        for (int i = 0; i < m.FaceCount(); ++i) {
            const ON_MeshFace& f = m.m_F[i];
            o.triangulos.push_back(uint32_t(f.vi[0])), o.triangulos.push_back(uint32_t(f.vi[1])), o.triangulos.push_back(uint32_t(f.vi[2]));
            if (f.IsQuad()) o.triangulos.push_back(uint32_t(f.vi[0])), o.triangulos.push_back(uint32_t(f.vi[2])), o.triangulos.push_back(uint32_t(f.vi[3]));
        }
        cx.saida.objetos.push_back(std::move(o));
        return;
    }

    std::unique_ptr<ON_Brep> forma;
    if (auto b = ON_Brep::Cast(geo)) forma.reset(b->Duplicate());
    else if (geo->HasBrepForm()) forma.reset(geo->BrepForm());
    if (!forma) {
        const ON_ClassId* id = geo->ClassId();
        cx.saida.ignorados.push_back(rust::String(id ? id->ClassName() : "objeto sem tipo"));
        return;
    }
    if (!identidade) forma->Transform(xf);
    bool aprox = false;
    int perdidas = 0;
    Fluxo fluxo = brep_para_fluxo(*forma, aprox, perdidas);
    for (int i = 0; i < perdidas; ++i) cx.saida.ignorados.push_back("face sem superfície utilizável");
    o.tipo = 0;
    o.aproximado = aprox;
    o.brep.reserve(fluxo.size());
    for (double x : fluxo) o.brep.push_back(x);
    cx.saida.objetos.push_back(std::move(o));
}

}  // namespace

ArquivoBruto ler_3dm(rust::Str caminho) {
    iniciar();
    ON_wString msgs;
    ON_TextLog log(msgs);
    ONX_Model modelo;
    if (!modelo.Read(static_cast<const wchar_t*>(largo(std::string(caminho))), &log))
        throw std::runtime_error("o openNURBS não conseguiu ler o 3DM: " + texto_do_log(msgs));

    ArquivoBruto a{};
    a.versao = modelo.m_3dm_file_version;
    const auto& ut = modelo.m_settings.m_ModelUnitsAndTolerances;
    double escala = ON::UnitScale(ut.m_unit_system, ON::LengthUnitSystem::Millimeters);
    if (!(escala > 0) || !std::isfinite(escala)) escala = 1.0;
    a.tolerancia = ut.m_absolute_tolerance * escala;

    Contexto cx{modelo, {}, a};
    std::vector<const ON_Layer*> camadas;
    std::map<ON_UUID, int, MenorUuid> por_id;
    ONX_ModelComponentIterator ic(modelo, ON_ModelComponent::Type::Layer);
    for (const ON_ModelComponent* c = ic.FirstComponent(); c; c = ic.NextComponent()) {
        const ON_Layer* l = ON_Layer::Cast(c);
        if (!l) continue;
        cx.camada_por_indice[l->Index()] = int(camadas.size());
        por_id[l->Id()] = int(camadas.size());
        camadas.push_back(l);
    }
    for (const ON_Layer* l : camadas) {
        auto pai = por_id.find(l->ParentLayerId());
        a.camadas.push_back(CamadaBruta{rust::String(utf8(l->Name())), rgb(l->Color()), pai == por_id.end() ? -1 : pai->second, l->IsVisible()});
    }

    const ON_Xform xf = escala == 1.0 ? ON_Xform::IdentityTransformation : ON_Xform::DiagonalTransformation(escala);
    ONX_ModelComponentIterator ig(modelo, ON_ModelComponent::Type::ModelGeometry);
    for (const ON_ModelComponent* c = ig.FirstComponent(); c; c = ig.NextComponent()) {
        const ON_ModelGeometryComponent* g = ON_ModelGeometryComponent::Cast(c);
        if (!g) continue;
        const ON_Geometry* geo = g->Geometry(nullptr);
        const ON_3dmObjectAttributes* at = g->Attributes(nullptr);
        // a geometria das definições de bloco só entra pelas inserções
        if (!geo || !at || at->IsInstanceDefinitionObject()) continue;
        traduzir(cx, geo, *at, xf, 0);
    }
    return a;
}

rust::Vec<rust::String> gravar_3dm(rust::Str caminho, const ArquivoBruto& a) {
    iniciar();
    rust::Vec<rust::String> avisos;
    ONX_Model modelo;
    modelo.m_settings.m_ModelUnitsAndTolerances.m_unit_system = ON_UnitSystem(ON::LengthUnitSystem::Millimeters);
    if (a.tolerancia > 0) modelo.m_settings.m_ModelUnitsAndTolerances.m_absolute_tolerance = a.tolerancia;

    // camadas na ordem em que as mães já existem
    const size_t nc = a.camadas.size();
    std::vector<int> indice(nc, -1);
    std::vector<ON_UUID> id(nc, ON_nil_uuid);
    for (size_t feitas = 0, volta = 0; feitas < nc && volta <= nc; ++volta) {
        for (size_t i = 0; i < nc; ++i) {
            if (indice[i] >= 0) continue;
            const CamadaBruta& c = a.camadas[i];
            const int pai = c.pai;
            const bool pai_ok = pai < 0 || size_t(pai) >= nc || indice[size_t(pai)] >= 0;
            if (!pai_ok && volta < nc) continue;
            ON_Layer l;
            l.SetName(largo(std::string(c.nome)));
            l.SetColor(cor(c.cor));
            l.SetVisible(c.visivel);
            if (pai >= 0 && size_t(pai) < nc && indice[size_t(pai)] >= 0) l.SetParentLayerId(id[size_t(pai)]);
            const ON_ModelComponentReference r = modelo.AddModelComponent(l, true);
            const ON_ModelComponent* mc = r.ModelComponent();
            if (!mc) throw std::runtime_error("o openNURBS recusou a camada '" + std::string(c.nome) + "'");
            indice[i] = mc->Index();
            id[i] = mc->Id();
            ++feitas;
        }
    }
    int padrao = -1;
    for (const ObjetoBruto& o : a.objetos) {
        if (o.camada >= 0 && size_t(o.camada) < nc) continue;
        if (padrao < 0) padrao = modelo.AddDefaultLayer(L"Padrão", ON_Color::Black);
        break;
    }

    for (const ObjetoBruto& o : a.objetos) {
        auto at = std::make_unique<ON_3dmObjectAttributes>();
        at->m_layer_index = (o.camada >= 0 && size_t(o.camada) < nc) ? indice[size_t(o.camada)] : padrao;
        if (!o.nome.empty()) at->SetName(largo(std::string(o.nome)), true);
        if (o.cor_propria) {
            at->SetColorSource(ON::color_from_object);
            at->m_color = cor(o.cor);
        }
        ON_Object* geo = nullptr;
        if (o.tipo == 0) {
            std::unique_ptr<ON_Brep> b = brep_de_fluxo(o.brep);
            ON_wString msgs;
            ON_TextLog log(msgs);
            if (!b->IsValid(&log)) avisos.push_back(rust::String(std::string(o.nome) + ": B-rep que o openNURBS não considera válida: " + texto_do_log(msgs, 300)));
            geo = b.release();
        } else {
            const size_t nv = o.vertices.size() / 3, nt = o.triangulos.size() / 3;
            auto m = std::make_unique<ON_Mesh>(int(nt), int(nv), false, false);
            for (size_t i = 0; i < nv; ++i) m->SetVertex(int(i), ON_3dPoint(o.vertices[3 * i], o.vertices[3 * i + 1], o.vertices[3 * i + 2]));
            for (size_t i = 0; i < nt; ++i) m->SetTriangle(int(i), int(o.triangulos[3 * i]), int(o.triangulos[3 * i + 1]), int(o.triangulos[3 * i + 2]));
            m->ComputeVertexNormals();
            geo = m.release();
        }
        const ON_ModelComponentReference r = modelo.AddManagedModelGeometryComponent(geo, at.release(), true);
        if (r.IsEmpty()) throw std::runtime_error("o openNURBS recusou o objeto '" + std::string(o.nome) + "'");
    }

    ON_wString msgs;
    ON_TextLog log(msgs);
    if (!modelo.Write(static_cast<const wchar_t*>(largo(std::string(caminho))), 0, &log))
        throw std::runtime_error("o openNURBS não conseguiu gravar o 3DM: " + texto_do_log(msgs));
    return avisos;
}

rust::String versao_opennurbs() { return rust::String(ON::VersionQuartetAsString()); }

}  // namespace fenkra3dm
