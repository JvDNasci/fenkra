// SPDX-License-Identifier: LGPL-2.1-or-later
// Features de forma, acabamento e transformação (incluído no fim de kernel.cpp: usa guard, wrap,
// carry, fill, to_vec, unify e area_of de lá).

namespace fenkra {

namespace {

gp_Pnt P3(rust::Slice<const double> p, size_t i) { return gp_Pnt(p[3 * i], p[3 * i + 1], p[3 * i + 2]); }

// Face do sólido mais próxima do ponto (referência de face por ponto clicado). Com `limit`, o ponto
// precisa estar a no máximo essa distância da face: senão a face escolhida não existe mais.
constexpr double kFaceLimit = 1.0;  // mm

TopoDS_Face face_near(const TopoDS_Shape& s, const gp_Pnt& p, double limit = 1e300) {
    TopoDS_Face best;
    double bd = 1e300;
    TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(p).Vertex();
    for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) {
        BRepExtrema_DistShapeShape d(e.Current(), v);
        if (d.IsDone() && d.Value() < bd) {
            bd = d.Value();
            best = TopoDS::Face(e.Current());
        }
    }
    if (best.IsNull()) throw std::runtime_error("nenhuma face encontrada");
    if (bd > limit) throw std::runtime_error("a face escolhida não existe mais");
    return best;
}

std::vector<TopoDS_Face> faces_near(const TopoDS_Shape& s, rust::Slice<const double> pts) {
    std::vector<TopoDS_Face> v;
    for (size_t i = 0; i + 2 < pts.size(); i += 3) v.push_back(face_near(s, gp_Pnt(pts[i], pts[i + 1], pts[i + 2]), kFaceLimit));
    if (v.empty()) throw std::runtime_error("escolha pelo menos uma face na viewport");
    return v;
}

template <class Mk>
std::unique_ptr<Shape> tagged_result(Mk& mk, const TopoDS_Shape& src, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    fill(out, carry(src, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return mk.Modified(f); }, -1));
    return wrap(mk.Shape());
}

// B-spline da guia suave; nula quando a guia tem cantos vivos (ou só dois pontos) e deve virar polilinha.
Handle(Geom_BSplineCurve) guia_suave(rust::Slice<const double> pts) {
    const size_t n = pts.size() / 3;
    if (n < 2) throw std::runtime_error("curva guia com menos de dois pontos");
    if (n == 2) return Handle(Geom_BSplineCurve)();
    std::vector<gp_Pnt> p;
    for (size_t i = 0; i < n; ++i) p.push_back(P3(pts, i));
    for (size_t i = 1; i + 1 < n; ++i) {
        gp_Vec a(p[i - 1], p[i]), b(p[i], p[i + 1]);
        if (a.Magnitude() > 1e-9 && b.Magnitude() > 1e-9 && a.Angle(b) > 0.35) return Handle(Geom_BSplineCurve)();
    }
    TColgp_Array1OfPnt arr(1, static_cast<int>(n));
    for (size_t i = 0; i < n; ++i) arr.SetValue(static_cast<int>(i) + 1, p[i]);
    GeomAPI_PointsToBSpline bs(arr, 3, 8, GeomAbs_C2, 1e-4);
    return bs.Curve();
}

// Curva guia a partir da polilinha: com cantos vivos vira polilinha, senão B-spline suave.
TopoDS_Wire spine_wire(rust::Slice<const double> pts) {
    Handle(Geom_BSplineCurve) c = guia_suave(pts);
    BRepBuilderAPI_MakeWire w;
    if (c.IsNull()) {
        for (size_t i = 0; i + 1 < pts.size() / 3; ++i)
            if (P3(pts, i).Distance(P3(pts, i + 1)) > 1e-9) w.Add(BRepBuilderAPI_MakeEdge(P3(pts, i), P3(pts, i + 1)).Edge());
    } else {
        w.Add(BRepBuilderAPI_MakeEdge(c).Edge());
    }
    if (!w.IsDone()) throw std::runtime_error("curva guia inválida");
    return w.Wire();
}

// A mesma curva em `k` arestas de parâmetro igual. Como todas apontam para a mesma B-spline, a tangente
// e a curvatura nas emendas são as da própria curva: dividir não cria canto.
TopoDS_Wire guia_dividida(const Handle(Geom_BSplineCurve)& c, int k) {
    const double a = c->FirstParameter(), b = c->LastParameter();
    BRepBuilderAPI_MakeWire w;
    for (int i = 0; i < k; ++i) w.Add(BRepBuilderAPI_MakeEdge(c, a + (b - a) * i / k, a + (b - a) * (i + 1) / k).Edge());
    if (!w.IsDone()) throw std::runtime_error("curva guia inválida");
    return w.Wire();
}

// Menor raio de curvatura da guia, com amostras em cada vão de nós (a curvatura muda de um vão a outro).
double raio_minimo(const Handle(Geom_BSplineCurve)& c) {
    double k = 0;
    for (int i = 1; i < c->NbKnots(); ++i) {
        const double a = c->Knot(i), b = c->Knot(i + 1);
        for (int j = 0; j <= 32; ++j) k = std::max(k, GeomLProp_CLProps(c, a + (b - a) * j / 32, 2, 1e-9).Curvature());
    }
    return k > 1e-12 ? 1 / k : 1e300;
}

// Maior distância do perfil ao início da guia: o perfil viaja rígido, então é esse o braço que dobra
// para dentro de uma curva fechada. Em guia plana só conta o braço no plano dela: a altura de uma
// parede erguida sobre a curva nunca dobra.
double alcance_do_perfil(const TopoDS_Shape& perfil, const gp_Pnt& inicio, const TColgp_Array1OfPnt& guia) {
    GProp_PEquation eq(guia, 1e-6);
    const bool plana = eq.IsPlanar();
    const gp_XYZ normal = plana ? eq.Plane().Axis().Direction().XYZ() : gp_XYZ();
    double r = 0;
    for (TopExp_Explorer e(perfil, TopAbs_EDGE); e.More(); e.Next()) {
        const TopoDS_Edge& ed = TopoDS::Edge(e.Current());
        if (BRep_Tool::Degenerated(ed)) continue;
        BRepAdaptor_Curve a(ed);
        for (int j = 0; j <= 16; ++j) {
            gp_XYZ v = a.Value(a.FirstParameter() + (a.LastParameter() - a.FirstParameter()) * j / 16).XYZ() - inicio.XYZ();
            if (plana) v -= normal * v.Dot(normal);
            r = std::max(r, v.Modulus());
        }
    }
    return r;
}

// Face plana a partir de laços: o maior é o contorno, os outros viram furos.
TopoDS_Shape face_from_wires(const TopoDS_Shape& ws) {
    std::vector<TopoDS_Face> fs;
    for (TopExp_Explorer e(ws, TopAbs_WIRE); e.More(); e.Next()) {
        BRepBuilderAPI_MakeFace mf(TopoDS::Wire(e.Current()), Standard_True);
        if (mf.IsDone()) fs.push_back(mf.Face());
    }
    if (fs.empty()) throw std::runtime_error("o deslocamento do perfil não gerou contorno");
    std::sort(fs.begin(), fs.end(), [](const TopoDS_Face& a, const TopoDS_Face& b) { return area_of(a) > area_of(b); });
    TopoDS_Shape r = fs[0];
    for (size_t i = 1; i < fs.size(); ++i) r = BRepAlgoAPI_Cut(r, fs[i]).Shape();
    return r;
}

gp_Trsf trsf_of(uint8_t kind, rust::Slice<const double> p) {
    gp_Trsf t;
    auto need = [&](size_t n) {
        if (p.size() < n) throw std::runtime_error("parâmetros insuficientes para a transformação");
    };
    switch (kind) {
        case 0:
            need(3);
            t.SetTranslation(gp_Vec(p[0], p[1], p[2]));
            break;
        case 1:
            need(7);
            t.SetRotation(gp_Ax1(gp_Pnt(p[0], p[1], p[2]), gp_Dir(p[3], p[4], p[5])), p[6] * M_PI / 180.0);
            break;
        case 2:
            need(6);
            t.SetMirror(gp_Ax2(gp_Pnt(p[0], p[1], p[2]), gp_Dir(p[3], p[4], p[5])));
            break;
        default:
            need(4);
            if (std::abs(p[3]) < 1e-9) throw std::runtime_error("fator de escala não pode ser zero");
            t.SetScale(gp_Pnt(p[0], p[1], p[2]), p[3]);
    }
    return t;
}

}  // namespace

std::unique_ptr<Shape> transform_shape(const Shape& s, uint8_t kind, rust::Slice<const double> p) {
    return guard("transformação", [&] { return wrap(BRepBuilderAPI_Transform(s.s, trsf_of(kind, p), Standard_True).Shape()); });
}

std::unique_ptr<Shape> transform_t(const Shape& s, uint8_t kind, rust::Slice<const double> p, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("transformação", [&] {
        BRepBuilderAPI_Transform mk(s.s, trsf_of(kind, p), Standard_True);
        TopTools_ListOfShape tmp;
        fill(out, carry(s.s, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& {
            tmp.Clear();
            tmp.Append(mk.ModifiedShape(f));
            return tmp;
        }, -1));
        return wrap(mk.Shape());
    });
}

namespace {

// Maior distância aceita entre a superfície aproximada e a varredura exata. O bom resultado fica perto
// de 5e-5 (metade da tolerância de 1e-4 do kernel); o ruim passa de 1 mm. `BRepCheck` e o volume não
// veem essa diferença: uma varredura com 4 mm de erro pode sair válida e com o volume certo.
constexpr double kErroVarredura = 1e-3;

// Uma tentativa completa. Falha de qualquer tipo (leitura de face nula inclusive) só devolve false,
// para a próxima tentativa seguir; quem decide o erro final é `sweep_tool`.
bool varre(const TopoDS_Wire& w, const TopoDS_Shape& perfil, GeomFill_Trihedron t, TopoDS_Shape& out) {
    bool ok = false;
    try {
        protegido("", [&] {
            BRepOffsetAPI_MakePipe mk(w, perfil, t, Standard_False);
            mk.Build();
            if (!mk.IsDone() || mk.Shape().IsNull() || mk.ErrorOnSurface() > kErroVarredura) return;
            if (!BRepCheck_Analyzer(mk.Shape()).IsValid()) return;
            out = mk.Shape();
            ok = true;
        });
    } catch (const std::runtime_error&) {
    } catch (const Standard_Failure&) {
    }
    return ok;
}

// Guia com cantos: em cada canto de ângulo t a parede recua alcance * tan(t/2) nos dois trechos. Se os
// recuos das duas pontas somam mais que o trecho, ele some e a parede cruza a vizinha. O kernel aprova
// esse sólido, com menos da metade do volume. Uma spline de curva apertada cai aqui, em trechos curtos.
void confere_cantos(const TopoDS_Shape& perfil, rust::Slice<const double> spine) {
    std::vector<gp_Pnt> p;
    for (size_t i = 0; i < spine.size() / 3; ++i)
        if (p.empty() || p.back().Distance(P3(spine, i)) > 1e-9) p.push_back(P3(spine, i));
    if (p.size() < 3) return;
    TColgp_Array1OfPnt arr(1, int(p.size()));
    for (size_t i = 0; i < p.size(); ++i) arr.SetValue(int(i) + 1, p[i]);
    const double alcance = alcance_do_perfil(perfil, p[0], arr);
    std::vector<double> recuo(p.size(), 0.);
    for (size_t i = 1; i + 1 < p.size(); ++i) recuo[i] = alcance * std::tan(gp_Vec(p[i - 1], p[i]).Angle(gp_Vec(p[i], p[i + 1])) / 2);
    for (size_t i = 0; i + 1 < p.size(); ++i) {
        const double trecho = p[i].Distance(p[i + 1]);
        if (recuo[i] + recuo[i + 1] > trecho) {
            char m[200];
            std::snprintf(m, sizeof m, "varredura: um trecho de %.3g mm da guia é curto demais para o alcance do perfil (%.3g mm) nos cantos, e o sólido se dobraria sobre si mesmo; suavize a guia ou reduza o perfil", trecho, alcance);
            throw std::runtime_error(m);
        }
    }
}

}  // namespace

std::unique_ptr<Shape> sweep_tool(const Shape& face, rust::Slice<const double> spine, uint8_t mode) {
    return guard("varredura", [&] {
        Handle(Geom_BSplineCurve) c = guia_suave(spine);
        if (!c.IsNull()) {
            // Curva mais fechada que o braço do perfil: a parede passa por cima dela mesma. O kernel
            // aprova esse sólido, e nenhum triedro corrige, porque o defeito está na geometria pedida.
            const double raio = raio_minimo(c), alcance = alcance_do_perfil(face.s, c->StartPoint(), c->Poles());
            if (raio < alcance) {
                char m[200];
                std::snprintf(m, sizeof m, "varredura: a guia faz uma curva de raio %.3g mm, menor que o alcance do perfil (%.3g mm), e o sólido se dobraria sobre si mesmo; suavize a guia ou reduza o perfil", raio, alcance);
                throw std::runtime_error(m);
            }
        } else {
            confere_cantos(face.s, spine);
        }
        const GeomFill_Trihedron t = mode == 1 ? GeomFill_IsFixed : mode == 2 ? GeomFill_IsFrenet : GeomFill_IsCorrectedFrenet;
        TopoDS_Shape out;
        // O caminho de sempre vem primeiro: guia que já varria mantém as mesmas faces.
        // Sem a proteção de `protegido`, essa tentativa numa guia suave pode derrubar o processo.
        if ((kViolacaoProtegida || c.IsNull()) && varre(spine_wire(spine), face.s, t, out)) return wrap(out);
        if (c.IsNull()) throw std::runtime_error("varredura: o resultado saiu inválido; confira se o perfil cabe nos cantos da guia");
        // O Frenet corrigido amostra a torção da guia com poucos passos por aresta e, numa curva longa,
        // a lei salta dezenas de graus entre vizinhos: a aproximação não acompanha. O triedro discreto
        // amostra por vão de nós e não salta. Frenet pedido pela pessoa continua Frenet.
        const GeomFill_Trihedron robusto = mode == 2 ? GeomFill_IsFrenet : GeomFill_IsDiscreteTrihedron;
        for (int k : {4, 8, 16}) {
            falhar_se_cancelado();
            if (varre(guia_dividida(c, k), face.s, robusto, out)) return wrap(out);
        }
        throw std::runtime_error("varredura: o kernel não conseguiu seguir esta guia suave, nem dividida em 16 trechos; simplifique a curva ou divida a guia");
    });
}

std::unique_ptr<Shape> loft_tool(const Shape& sections, bool ruled, bool closed, bool alinhar) {
    return guard("multi-seções", [&] {
        std::vector<TopoDS_Wire> ws;
        for (TopExp_Explorer e(sections.s, TopAbs_FACE); e.More(); e.Next()) ws.push_back(BRepTools::OuterWire(TopoDS::Face(e.Current())));
        if (ws.size() < 2) throw std::runtime_error("são precisos pelo menos dois perfis");
        BRepOffsetAPI_ThruSections ts(Standard_True, ruled ? Standard_True : Standard_False);
        for (auto& w : ws) ts.AddWire(w);
        if (closed) ts.AddWire(ws[0]);
        ts.CheckCompatibility(alinhar ? Standard_True : Standard_False);
        ts.Build();
        if (!ts.IsDone()) throw std::runtime_error("os perfis não puderam ser ligados (confira a ordem e o sentido)");
        return wrap(ts.Shape());
    });
}

std::unique_ptr<Shape> primitive(uint8_t kind, const Frame& fr, rust::Slice<const double> p) {
    return guard("primitiva", [&] {
        auto v = [&](size_t i) {
            if (i >= p.size() || p[i] <= 0) throw std::runtime_error("as medidas da primitiva precisam ser positivas");
            return p[i];
        };
        gp_Dir n(fr.nx, fr.ny, fr.nz), x(fr.xx, fr.xy, fr.xz);
        gp_Pnt o(fr.ox, fr.oy, fr.oz);
        gp_Ax2 ax(o, n, x);
        switch (kind) {
            case 0: {  // caixa centrada no plano, altura ao longo da normal
                gp_Dir y = n.Crossed(x);
                gp_Pnt c(o.XYZ() - x.XYZ() * (v(0) / 2) - y.XYZ() * (v(1) / 2));
                return wrap(BRepPrimAPI_MakeBox(gp_Ax2(c, n, x), v(0), v(1), v(2)).Shape());
            }
            case 1:
                return wrap(BRepPrimAPI_MakeCylinder(ax, v(0), v(1)).Shape());
            case 2:
                return wrap(BRepPrimAPI_MakeSphere(ax, v(0)).Shape());
            case 3: {
                const double r2 = p.size() > 1 ? std::max(0.0, p[1]) : 0.0;
                return wrap(BRepPrimAPI_MakeCone(ax, v(0), r2, v(2)).Shape());
            }
            case 4:
                return wrap(BRepPrimAPI_MakeTorus(ax, v(0), v(1)).Shape());
            case 5: {
                const double ltx = p.size() > 3 ? std::max(0.0, p[3]) : 0.0;
                return wrap(BRepPrimAPI_MakeWedge(ax, v(0), v(1), v(2), ltx).Shape());
            }
            default: {  // prisma de polígono regular: lados, raio, altura
                const int k = static_cast<int>(v(0));
                if (k < 3) throw std::runtime_error("o polígono precisa de pelo menos 3 lados");
                gp_Dir y = n.Crossed(x);
                BRepBuilderAPI_MakePolygon poly;
                for (int i = 0; i < k; ++i) {
                    const double a = 2 * M_PI * i / k;
                    poly.Add(gp_Pnt(o.XYZ() + x.XYZ() * (v(1) * std::cos(a)) + y.XYZ() * (v(1) * std::sin(a))));
                }
                poly.Close();
                TopoDS_Face f = BRepBuilderAPI_MakeFace(poly.Wire(), Standard_True).Face();
                return wrap(BRepPrimAPI_MakePrism(f, gp_Vec(n) * v(2)).Shape());
            }
        }
    });
}

// Casca aberta por booleana (B48). O deslocador de faces do OpenCASCADE não fecha a parede em volta de
// um furo escareado de fundo plano quando a face aberta é a do próprio furo. Aqui a face aberta ganha
// uma "tampa" de material por fora (prisma da face, para fora da peça): o corpo estendido não tem mais
// aquela face, só o que sobra de contorno por dentro. Desloca-se o corpo estendido para dentro (a
// cavidade) e a casca é o corpo original menos essa cavidade; como a tampa cobre a abertura, a
// cavidade sai pela face aberta e não deixa parede nela.
static std::shared_ptr<BRepAlgoAPI_Cut> shell_open_by_boolean(const TopoDS_Shape& body, const TopTools_ListOfShape& rm, double t, bool keep_holes) {
    TopoDS_Shape ext = body;
    for (const TopoDS_Shape& fs : rm) {
        TopoDS_Face f = TopoDS::Face(fs);
        BRepAdaptor_Surface su(f);
        if (su.GetType() != GeomAbs_Plane) throw std::runtime_error("casca por booleana só abre faces planas");
        gp_Dir n = su.Plane().Axis().Direction();
        if (f.Orientation() == TopAbs_REVERSED) n.Reverse();
        TopoDS_Face base = f;
        if (!keep_holes) {
            // só o contorno externo: a tampa cobre também a boca do furo
            BRepBuilderAPI_MakeFace mf(su.Plane(), BRepTools::OuterWire(f));
            if (!mf.IsDone()) throw std::runtime_error("não foi possível cobrir a face aberta");
            base = mf.Face();
            if (base.Orientation() != f.Orientation()) base.Reverse();
        }
        TopoDS_Shape plate = BRepPrimAPI_MakePrism(base, gp_Vec(n) * (2 * t + 1.0)).Shape();
        BRepAlgoAPI_Fuse fu(ext, plate);
        if (!fu.IsDone() || fu.Shape().IsNull()) throw std::runtime_error("não foi possível cobrir a face aberta");
        ext = fu.Shape();
    }
    ext = unify(ext);
    BRepOffsetAPI_MakeOffsetShape off;
    off.PerformByJoin(ext, -t, 1e-3, BRepOffset_Skin, Standard_False, Standard_False, GeomAbs_Arc);
    if (!off.IsDone() || off.Shape().IsNull()) throw std::runtime_error("espessura grande demais para a peça");
    // o deslocamento pode voltar como casca ou composto: a booleana pede um sólido
    TopoDS_Shape cav = off.Shape();
    if (cav.ShapeType() != TopAbs_SOLID) {
        TopExp_Explorer es(cav, TopAbs_SOLID);
        if (es.More()) {
            cav = es.Current();
        } else {
            TopExp_Explorer eh(cav, TopAbs_SHELL);
            if (!eh.More()) throw std::runtime_error("o deslocamento da casca não é um sólido");
            BRepBuilderAPI_MakeSolid ms(TopoDS::Shell(eh.Current()));
            cav = ms.Solid();
        }
        GProp_GProps vc;
        BRepGProp::VolumeProperties(cav, vc);
        if (vc.Mass() < 0) cav.Reverse();
    }
    for (double fuzzy : {0.0, 1e-4, 1e-3}) {
        auto cut = std::make_shared<BRepAlgoAPI_Cut>();
        TopTools_ListOfShape a1, t1;
        a1.Append(body);
        t1.Append(cav);
        cut->SetArguments(a1);
        cut->SetTools(t1);
        if (fuzzy > 0) cut->SetFuzzyValue(fuzzy);
        cut->Build();
        if (cut->IsDone() && !cut->HasErrors() && !cut->Shape().IsNull()) return cut;
    }
    throw std::runtime_error("não foi possível esvaziar o corpo");
}

std::unique_ptr<Shape> shell_t(const Shape& s, rust::Slice<const double> pts, double thickness, bool outward,
                               rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("casca", [&] {
        if (thickness <= 0) throw std::runtime_error("a espessura precisa ser positiva");
        TopTools_ListOfShape rm;
        // sem faces = casca fechada: o sólido deslocado (para dentro, ou para fora) vira a cavidade,
        // subtraída por booleana; as faces de origem mantêm a chave pela história da booleana
        if (pts.size() < 3) {
            BRepOffsetAPI_MakeOffsetShape off;
            off.PerformByJoin(s.s, outward ? thickness : -thickness, 1e-3, BRepOffset_Skin, Standard_False, Standard_False, GeomAbs_Arc);
            if (!off.IsDone() || off.Shape().IsNull()) throw std::runtime_error("espessura grande demais para a peça");
            if (outward) {
                BRepAlgoAPI_Cut cut(off.Shape(), s.s);
                if (!cut.IsDone()) throw std::runtime_error("não foi possível esvaziar o corpo");
                return tagged_result(cut, s.s, tags, out);
            }
            BRepAlgoAPI_Cut cut(s.s, off.Shape());
            if (!cut.IsDone()) throw std::runtime_error("não foi possível esvaziar o corpo");
            return tagged_result(cut, s.s, tags, out);
        }
        for (auto& f : faces_near(s.s, pts)) rm.Append(f);
        BRepOffsetAPI_MakeThickSolid mk;
        mk.MakeThickSolidByJoin(s.s, rm, outward ? thickness : -thickness, 1e-3, BRepOffset_Skin, Standard_False, Standard_False, GeomAbs_Arc);
        mk.Build();
        // B48: o deslocador falha (ou devolve corpo inválido) em torno de furo escareado de fundo plano
        // quando a face aberta é a da boca do furo: cai para a casca por booleana
        bool bom = mk.IsDone() && !mk.Shape().IsNull();
        if (bom) {
            BRepCheck_Analyzer an(mk.Shape());
            bom = an.IsValid();
        }
        if (!bom && !outward) {
            for (bool furos : {true, false}) {
                try {
                    auto cut = shell_open_by_boolean(s.s, rm, thickness, furos);
                    BRepCheck_Analyzer an(cut->Shape());
                    if (!an.IsValid()) continue;
                    // a casca precisa ter tirado material
                    GProp_GProps a, b;
                    BRepGProp::VolumeProperties(s.s, a);
                    BRepGProp::VolumeProperties(cut->Shape(), b);
                    if (b.Mass() >= a.Mass() - 1e-9 * a.Mass() || b.Mass() <= 0) continue;
                    return tagged_result(*cut, s.s, tags, out);
                } catch (const Standard_Failure&) {
                } catch (const std::exception&) {
                }
            }
        }
        if (!mk.IsDone()) throw std::runtime_error("espessura grande demais para a peça");
        return tagged_result(mk, s.s, tags, out);
    });
}

// Casca com espessura própria em algumas faces: `fpts` tem os pontos e `vals`, a espessura de cada uma.
std::unique_ptr<Shape> shell_faces_t(const Shape& s, rust::Slice<const double> pts, double thickness, bool outward,
                                     rust::Slice<const double> fpts, rust::Slice<const double> vals,
                                     rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("casca", [&] {
        if (thickness <= 0) throw std::runtime_error("a espessura precisa ser positiva");
        double sinal = outward ? 1.0 : -1.0;
        auto abertas = faces_near(s.s, pts);
        auto proprias = faces_near(s.s, fpts);
        if (proprias.size() != vals.size()) throw std::runtime_error("uma face da lista de espessuras não foi achada");
        // faces planas vizinhas com paredes diferentes só fecham por interseção; arco fica de reserva
        for (GeomAbs_JoinType junta : {GeomAbs_Intersection, GeomAbs_Arc}) {
            BRepOffset_MakeOffset mk;
            mk.Initialize(s.s, sinal * thickness, 1e-3, BRepOffset_Skin, Standard_False, Standard_False, junta, Standard_False, Standard_False);
            for (auto& f : abertas) mk.AddFace(f);
            for (size_t i = 0; i < proprias.size(); i++) mk.SetOffsetOnFace(proprias[i], sinal * vals[i]);
            mk.MakeThickSolid();
            if (!mk.IsDone() || mk.Shape().IsNull()) continue;
            fill(out, carry(s.s, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return mk.Modified(f); }, -1));
            return wrap(mk.Shape());
        }
        throw std::runtime_error("as espessuras escolhidas não fecham a parede");
    });
}

std::unique_ptr<Shape> draft_t(const Shape& s, rust::Slice<const double> pts, double dx, double dy, double dz, double angle_deg,
                               double ox, double oy, double oz, double nx, double ny, double nz,
                               rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("ângulo de saída", [&] {
        BRepOffsetAPI_DraftAngle d(s.s);
        gp_Dir pull(dx, dy, dz);
        gp_Pln neutral(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz));
        for (auto& f : faces_near(s.s, pts)) {
            d.Add(f, pull, angle_deg * M_PI / 180.0, neutral);
            if (!d.AddDone()) throw std::runtime_error("a face não aceita ângulo de saída (precisa ser plana, cilíndrica ou cônica e cruzar o plano neutro)");
        }
        d.Build();
        if (!d.IsDone()) throw std::runtime_error("não foi possível aplicar o ângulo de saída");
        return tagged_result(d, s.s, tags, out);
    });
}

std::unique_ptr<Shape> faces_offset_tool(const Shape& s, rust::Slice<const double> pts, double value) {
    return guard("espessura de faces", [&] {
        TopoDS_Compound c;
        BRep_Builder bb;
        bb.MakeCompound(c);
        for (auto& f : faces_near(s.s, pts)) {
            BRepAdaptor_Surface su(f);
            if (su.GetType() != GeomAbs_Plane) throw std::runtime_error("espessura nesta versão só em faces planas");
            gp_Dir n = su.Plane().Axis().Direction();
            if (f.Orientation() == TopAbs_REVERSED) n.Reverse();
            bb.Add(c, BRepPrimAPI_MakePrism(f, gp_Vec(n) * value).Shape());
        }
        return wrap(c);
    });
}

// Verificação de sólido válido (B33): a análise de topologia e geometria do kernel aprova o corpo.
bool shape_valid(const Shape& s) {
    if (s.s.IsNull()) return false;
    BRepCheck_Analyzer an(s.s);
    return an.IsValid();
}

// Edição direta (B33): apaga as faces e deixa as vizinhas se estenderem até se encontrar ("cura").
std::unique_ptr<Shape> defeature_t(const Shape& s, rust::Slice<const double> pts, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("apagar face", [&] {
        BRepAlgoAPI_Defeaturing df;
        df.SetShape(s.s);
        for (auto& f : faces_near(s.s, pts)) df.AddFaceToRemove(f);
        df.SetToFillHistory(true);
        df.Build();
        if (df.HasErrors() || df.Shape().IsNull()) throw std::runtime_error("as vizinhas não conseguem fechar o corpo sem esta face");
        return tagged_result(df, s.s, tags, out);
    });
}

// Edição direta (B33): desloca as faces pela distância dada ao longo da normal, com as vizinhas
// acompanhando (também em faces cilíndricas, esféricas e cônicas: muda o raio).
std::unique_ptr<Shape> face_offset_t(const Shape& s, rust::Slice<const double> pts, double value, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("deslocar face", [&] {
        BRepOffset_MakeOffset mk;
        mk.Initialize(s.s, 0.0, 1e-5, BRepOffset_Skin, Standard_True, Standard_False, GeomAbs_Intersection, Standard_False, Standard_False);
        for (auto& f : faces_near(s.s, pts)) mk.SetOffsetOnFace(f, value);
        mk.MakeOffsetShape();
        if (!mk.IsDone() || mk.Shape().IsNull()) throw std::runtime_error("o deslocamento desfaz o corpo: reduza a distância");
        fill(out, carry(s.s, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return mk.Modified(f); }, -1));
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> extrude_taper(const Shape& face, double dx, double dy, double dz, double len, double angle_deg) {
    return guard("extrusão com ângulo", [&] {
        if (len <= 1e-9) throw std::runtime_error("comprimento da extrusão deve ser maior que zero");
        gp_Vec d(dx, dy, dz);
        d.Normalize();
        const double off = len * std::tan(angle_deg * M_PI / 180.0);
        gp_Trsf mv;
        mv.SetTranslation(d * len);
        auto loft = [&](const TopoDS_Wire& w, double o) {
            TopoDS_Face f = BRepBuilderAPI_MakeFace(w, Standard_True).Face();
            BRepOffsetAPI_MakeOffset mo(f, GeomAbs_Intersection);
            mo.Perform(o);
            if (!mo.IsDone()) throw std::runtime_error("ângulo grande demais para o perfil");
            TopExp_Explorer e(mo.Shape(), TopAbs_WIRE);
            if (!e.More()) throw std::runtime_error("ângulo grande demais para o perfil");
            TopoDS_Wire top = TopoDS::Wire(BRepBuilderAPI_Transform(e.Current(), mv, Standard_True).Shape());
            BRepOffsetAPI_ThruSections ts(Standard_True, Standard_True);
            ts.AddWire(w);
            ts.AddWire(top);
            ts.Build();
            if (!ts.IsDone()) throw std::runtime_error("falha na extrusão com ângulo");
            return ts.Shape();
        };
        TopoDS_Shape result;
        for (TopExp_Explorer fe(face.s, TopAbs_FACE); fe.More(); fe.Next()) {
            TopoDS_Face f = TopoDS::Face(fe.Current());
            TopoDS_Wire outer = BRepTools::OuterWire(f);
            TopoDS_Shape r = loft(outer, -off);
            for (TopExp_Explorer we(f, TopAbs_WIRE); we.More(); we.Next()) {
                if (we.Current().IsSame(outer)) continue;
                r = BRepAlgoAPI_Cut(r, loft(TopoDS::Wire(we.Current().Reversed()), off)).Shape();
            }
            result = result.IsNull() ? r : BRepAlgoAPI_Fuse(result, r).Shape();
        }
        return wrap(unify(result));
    });
}

std::unique_ptr<Shape> thin_profile(const Shape& face, double t) {
    return guard("parede fina", [&] {
        if (t <= 0) throw std::runtime_error("a espessura precisa ser positiva");
        TopoDS_Shape result;
        for (TopExp_Explorer fe(face.s, TopAbs_FACE); fe.More(); fe.Next()) {
            TopoDS_Face f = TopoDS::Face(fe.Current());
            BRepOffsetAPI_MakeOffset mo(f, GeomAbs_Arc);
            mo.Perform(t / 2);
            if (!mo.IsDone()) throw std::runtime_error("não foi possível engrossar o perfil");
            TopoDS_Shape ring = face_from_wires(mo.Shape());
            BRepOffsetAPI_MakeOffset mi(f, GeomAbs_Arc);
            mi.Perform(-t / 2);
            if (mi.IsDone() && TopExp_Explorer(mi.Shape(), TopAbs_WIRE).More()) ring = BRepAlgoAPI_Cut(ring, face_from_wires(mi.Shape())).Shape();
            result = result.IsNull() ? ring : BRepAlgoAPI_Fuse(result, ring).Shape();
        }
        return wrap(unify(result));
    });
}

std::unique_ptr<Shape> thick_segments(rust::Slice<const double> segs, double nx, double ny, double nz, double t) {
    return guard("nervura", [&] {
        if (t <= 0) throw std::runtime_error("a espessura precisa ser positiva");
        gp_Dir n(nx, ny, nz);
        TopoDS_Shape acc;
        auto add = [&](const TopoDS_Shape& f) { acc = acc.IsNull() ? f : BRepAlgoAPI_Fuse(acc, f).Shape(); };
        const size_t k = segs.size() / 6;
        for (size_t i = 0; i < k; ++i) {
            gp_Pnt a = P3(segs, 2 * i), b = P3(segs, 2 * i + 1);
            gp_Vec d(a, b);
            if (d.Magnitude() < 1e-9) continue;
            gp_Vec side = gp_Vec(n).Crossed(d).Normalized() * (t / 2);
            BRepBuilderAPI_MakePolygon poly(a.Translated(side), b.Translated(side), b.Translated(-side), a.Translated(-side), Standard_True);
            add(BRepBuilderAPI_MakeFace(poly.Wire(), Standard_True).Face());
            for (const gp_Pnt& q : {a, b}) {
                gp_Circ c(gp_Ax2(q, n), t / 2);
                TopoDS_Wire w = BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(c).Edge()).Wire();
                add(BRepBuilderAPI_MakeFace(w, Standard_True).Face());
            }
        }
        if (acc.IsNull()) throw std::runtime_error("o sketch não tem linhas");
        return wrap(unify(acc));
    });
}

namespace {

// Ferramenta do filete assimétrico (B289) numa aresta reta entre duas faces planas.
//
// No plano normal à aresta, u e w saem da aresta e correm sobre cada face. Um filete simétrico de
// "comprimento de tangência" 1 nesse par de eixos oblíquos é um arco de círculo; esticar o eixo u
// por d1 e o eixo w por d2 (transformação afim) leva círculo em elipse e tangente em tangente. Então
// o filete com distâncias d1 e d2 é o arco elíptico tangente às duas faces a d1 e d2 da aresta.
// A região entre o arco e a quina, extrudada pelo comprimento da aresta, é o que sai (aresta
// convexa, `fuse = false`) ou o que entra (aresta côncava, `fuse = true`).
struct AsymTool {
    TopoDS_Shape tool;
    bool fuse = false;
};

AsymTool asym_tool(const TopoDS_Shape& solid, const TopoDS_Edge& e, const TopoDS_Face& fa, const TopoDS_Face& fb, double d1, double d2) {
    BRepAdaptor_Curve c(e);
    if (c.GetType() != GeomAbs_Line) throw std::runtime_error("o filete assimétrico precisa de aresta reta");
    BRepAdaptor_Surface sa(fa), sb(fb);
    if (sa.GetType() != GeomAbs_Plane || sb.GetType() != GeomAbs_Plane) throw std::runtime_error("o filete assimétrico precisa de duas faces planas");
    const double t0 = c.FirstParameter(), t1 = c.LastParameter();
    const gp_Pnt p0 = c.Value(t0), p1 = c.Value(t1), mid = c.Value((t0 + t1) / 2);
    gp_Vec t(p0, p1);
    const double len = t.Magnitude();
    if (len < 1e-9) throw std::runtime_error("aresta degenerada");
    t.Normalize();
    const double probe = 1e-3 * std::min(len, d1 + d2);
    auto inward = [&](const TopoDS_Face& f, const gp_Pln& pl) {
        gp_Vec u = gp_Vec(pl.Axis().Direction()).Crossed(t);
        u.Normalize();
        BRepClass_FaceClassifier fc(f, mid.Translated(u * probe), 1e-7);
        return fc.State() == TopAbs_IN ? u : u.Reversed();
    };
    const gp_Vec u = inward(fa, sa.Plane()), w = inward(fb, sb.Plane());
    const double cs = u.Dot(w);
    if (std::abs(cs) > 0.9962) throw std::runtime_error("as duas faces são quase tangentes: não há quina para arredondar");
    const double phi = std::acos(cs);  // ângulo entre as duas faces, no plano normal à aresta
    gp_Vec bis = u + w;
    bis.Normalize();
    BRepClass3d_SolidClassifier sc(solid, mid.Translated(bis * probe), 1e-7);
    AsymTool out;
    out.fuse = sc.State() != TopAbs_IN;  // a cunha entre as faces é vazia: aresta côncava
    // coordenadas oblíquas (alfa ao longo de u, beta ao longo de w) e o alongamento L
    const double den = 1 - cs * cs;
    auto L = [&](const gp_Vec& x) {
        const double al = (x.Dot(u) - cs * x.Dot(w)) / den, be = (x.Dot(w) - cs * x.Dot(u)) / den;
        return u * (al * d1) + w * (be * d2);
    };
    const double kappa = 1.0 / (2 * std::cos(phi / 2) * std::cos(phi / 2));
    const gp_Vec cc0 = (u + w) * kappa, a0 = u - cc0, b0 = w - cc0;
    gp_Vec e2 = t.Crossed(a0);
    if (e2.Dot(b0) < 0) e2.Reverse();
    const gp_Vec A = L(a0), B = L(e2);
    const gp_Pnt cc = p0.Translated(L(cc0));
    const double gamma = M_PI - phi;
    const double ts = 0.5 * std::atan2(2 * A.Dot(B), A.SquareMagnitude() - B.SquareMagnitude());
    const gp_Vec M = A * std::cos(ts) + B * std::sin(ts), m = B * std::cos(ts) - A * std::sin(ts);
    if (M.Magnitude() < 1e-9 || m.Magnitude() < 1e-9) throw std::runtime_error("distâncias do filete assimétrico degeneradas");
    gp_Ax2 ax(cc, gp_Dir(t), gp_Dir(M));
    double s0 = -ts, s1 = gamma - ts;
    if (m.Dot(gp_Vec(ax.YDirection())) < 0) {
        const double a = -s1, b = -s0;
        s0 = a;
        s1 = b;
    }
    gp_Elips el(ax, M.Magnitude(), m.Magnitude());
    TopoDS_Edge arc = BRepBuilderAPI_MakeEdge(el, s0, s1).Edge();
    // margem fora da peça (corte) ou dentro do material (preenchimento): sem faces coincidentes
    gp_Vec nu = w - u * cs, nw = u - w * cs;
    nu.Reverse();
    nw.Reverse();
    nu.Normalize();
    nw.Normalize();
    const double g = 0.02 * std::min(d1, d2);
    const gp_Pnt ta = p0.Translated(u * d1), tb = p0.Translated(w * d2);
    const gp_Pnt qb = tb.Translated(nw * g), qc = p0.Translated((nu + nw) * g), qa = ta.Translated(nu * g);
    BRepBuilderAPI_MakeWire mw;
    mw.Add(arc);
    mw.Add(BRepBuilderAPI_MakeEdge(tb, qb).Edge());
    mw.Add(BRepBuilderAPI_MakeEdge(qb, qc).Edge());
    mw.Add(BRepBuilderAPI_MakeEdge(qc, qa).Edge());
    mw.Add(BRepBuilderAPI_MakeEdge(qa, ta).Edge());
    if (!mw.IsDone()) throw std::runtime_error("não foi possível montar o perfil do filete assimétrico");
    TopoDS_Face face = BRepBuilderAPI_MakeFace(mw.Wire(), Standard_True).Face();
    out.tool = BRepPrimAPI_MakePrism(face, gp_Vec(p0, p1)).Shape();
    return out;
}

}  // namespace

std::unique_ptr<Shape> edges_mode_t(const Shape& s, uint8_t mode, double v1, double v2, rust::Slice<const double> mids, rust::Slice<const uint8_t> flips,
                                    double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard(mode == 0 ? "filete variável" : mode == 3 ? "filete assimétrico" : "chanfro", [&] {
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        TopTools_IndexedDataMapOfShapeListOfShape e2f;
        TopExp::MapShapesAndAncestors(s.s, TopAbs_EDGE, TopAbs_FACE, e2f);
        std::vector<TopoDS_Edge> chosen;
        int missing = 0;
        for (size_t k = 0; k + 2 < mids.size(); k += 3) {
            gp_Pnt ref(mids[k], mids[k + 1], mids[k + 2]);
            int best = 0;
            double bd = 1e300;
            for (int i = 1; i <= edges.Extent(); ++i) {
                const TopoDS_Edge& e = TopoDS::Edge(edges(i));
                if (BRep_Tool::Degenerated(e)) continue;
                BRepAdaptor_Curve c(e);
                double d = c.Value((c.FirstParameter() + c.LastParameter()) / 2).Distance(ref);
                if (d < bd) {
                    bd = d;
                    best = i;
                }
            }
            if (best > 0 && bd <= tol) chosen.push_back(TopoDS::Edge(edges(best)));
            else ++missing;
        }
        if (missing > 0) throw std::runtime_error(std::to_string(missing) + " das arestas escolhidas não existe(m) mais — reedite a seleção de arestas");
        if (chosen.empty()) throw std::runtime_error("escolha as arestas na viewport");
        // inverter por aresta: a 1ª distância (ou raio) passa para a outra face da aresta
        auto flipped = [&](size_t i) { return i < flips.size() && flips[i] != 0; };
        auto faces_of = [&](const TopoDS_Edge& e, size_t i) {
            const TopTools_ListOfShape& l = e2f.FindFromKey(e);
            if (l.Extent() != 2) throw std::runtime_error("a aresta não separa exatamente duas faces");
            TopoDS_Face a = TopoDS::Face(l.First()), b = TopoDS::Face(l.Last());
            if (flipped(i)) std::swap(a, b);
            return std::make_pair(a, b);
        };
        if (mode == 0) {
            BRepFilletAPI_MakeFillet mk(s.s);
            for (auto& e : chosen) mk.Add(v1, v2, e);
            mk.Build();
            if (!mk.IsDone()) throw std::runtime_error("raios grandes demais para as arestas");
            return tagged_result(mk, s.s, tags, out);
        }
        if (mode == 3) {
            TopTools_ListOfShape cuts, fuses;
            for (size_t i = 0; i < chosen.size(); ++i) {
                auto fs = faces_of(chosen[i], i);
                AsymTool at = asym_tool(s.s, chosen[i], fs.first, fs.second, v1, v2);
                (at.fuse ? fuses : cuts).Append(at.tool);
            }
            TopoDS_Shape cur = s.s;
            std::vector<int> tg = to_vec(tags);
            auto step = [&](TopTools_ListOfShape& tools, bool fuse) {
                if (tools.IsEmpty()) return;
                TopTools_ListOfShape args;
                args.Append(cur);
                std::unique_ptr<BRepAlgoAPI_BooleanOperation> op;
                if (fuse) op = std::make_unique<BRepAlgoAPI_Fuse>();
                else op = std::make_unique<BRepAlgoAPI_Cut>();
                op->SetArguments(args);
                op->SetTools(tools);
                op->Build();
                if (!op->IsDone() || op->Shape().IsNull()) throw std::runtime_error("distâncias grandes demais para as arestas");
                tg = carry(cur, tg, op->Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return op->Modified(f); }, -1);
                cur = op->Shape();
            };
            step(cuts, false);
            step(fuses, true);
            fill(out, tg);
            return wrap(cur);
        }
        BRepFilletAPI_MakeChamfer mk(s.s);
        for (size_t i = 0; i < chosen.size(); ++i) {
            const TopoDS_Face f = faces_of(chosen[i], i).first;
            if (mode == 1) mk.Add(v1, v2, chosen[i], f);
            else mk.AddDA(v1, v2 * M_PI / 180.0, chosen[i], f);
        }
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("valores grandes demais para as arestas");
        return tagged_result(mk, s.s, tags, out);
    });
}

// Filete variável com pontos de controle (B49): raio r0 no começo da aresta, r1 no fim e os pares
// (posição relativa 0 a 1, raio) de `ctrl` no meio. `smooth`: a lei passa suave (spline) pelos pontos;
// senão o raio varia em retas de um ponto ao outro.
std::unique_ptr<Shape> fillet_var_t(const Shape& s, double r0, double r1, rust::Slice<const double> ctrl, bool smooth, rust::Slice<const double> mids,
                                    double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("filete variável", [&] {
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        std::vector<TopoDS_Edge> chosen;
        int missing = 0;
        for (size_t k = 0; k + 2 < mids.size(); k += 3) {
            gp_Pnt ref(mids[k], mids[k + 1], mids[k + 2]);
            int best = 0;
            double bd = 1e300;
            for (int i = 1; i <= edges.Extent(); ++i) {
                const TopoDS_Edge& e = TopoDS::Edge(edges(i));
                if (BRep_Tool::Degenerated(e)) continue;
                BRepAdaptor_Curve c(e);
                double d = c.Value((c.FirstParameter() + c.LastParameter()) / 2).Distance(ref);
                if (d < bd) {
                    bd = d;
                    best = i;
                }
            }
            if (best > 0 && bd <= tol) chosen.push_back(TopoDS::Edge(edges(best)));
            else ++missing;
        }
        if (missing > 0) throw std::runtime_error(std::to_string(missing) + " das arestas escolhidas não existe(m) mais — reedite a seleção de arestas");
        if (chosen.empty()) throw std::runtime_error("escolha as arestas na viewport");
        std::vector<std::pair<double, double>> pts;
        pts.emplace_back(0.0, r0);
        for (size_t k = 0; k + 1 < ctrl.size(); k += 2) pts.emplace_back(ctrl[k], ctrl[k + 1]);
        pts.emplace_back(1.0, r1);
        for (size_t i = 0; i < pts.size(); ++i) {
            if (pts[i].second <= 0) throw std::runtime_error("todo ponto de controle precisa de raio maior que zero");
            if (i > 0 && pts[i].first <= pts[i - 1].first + 1e-6) throw std::runtime_error("os pontos de controle precisam de posições diferentes, entre 0 e 100%");
        }
        // sem transição suave: a lei vira retas de um ponto ao outro. A interpolação do kernel passa por
        // pontos, então os vãos são amostrados na reta (10 passos): a curva segue a reta sem ondular
        if (!smooth) {
            std::vector<std::pair<double, double>> fino;
            for (size_t i = 0; i + 1 < pts.size(); ++i)
                for (int k = 0; k < 10; ++k) {
                    const double t = k / 10.0;
                    fino.emplace_back(pts[i].first + t * (pts[i + 1].first - pts[i].first), pts[i].second + t * (pts[i + 1].second - pts[i].second));
                }
            fino.push_back(pts.back());
            pts = fino;
        }
        const int n = static_cast<int>(pts.size());
        BRepFilletAPI_MakeFillet mk(s.s);
        for (auto& e : chosen) {
            TColgp_Array1OfPnt2d arr(1, n);
            for (int i = 0; i < n; ++i) arr.SetValue(i + 1, gp_Pnt2d(pts[i].first, pts[i].second));
            mk.Add(arr, e);
        }
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("raios grandes demais para as arestas");
        return tagged_result(mk, s.s, tags, out);
    });
}

rust::Vec<double> cylinder_at(const Shape& s, double px, double py, double pz) {
    return guard("face cilíndrica", [&] {
        TopoDS_Face f = face_near(s.s, gp_Pnt(px, py, pz), kFaceLimit);
        BRepAdaptor_Surface su(f);
        if (su.GetType() != GeomAbs_Cylinder) throw std::runtime_error("a face escolhida não é cilíndrica");
        gp_Cylinder c = su.Cylinder();
        gp_Ax1 ax = c.Axis();
        // extensão da face ao longo do eixo
        double lo = 1e300, hi = -1e300;
        for (TopExp_Explorer e(f, TopAbs_VERTEX); e.More(); e.Next()) {
            gp_Pnt q = BRep_Tool::Pnt(TopoDS::Vertex(e.Current()));
            double h = gp_Vec(ax.Location(), q).Dot(gp_Vec(ax.Direction()));
            lo = std::min(lo, h);
            hi = std::max(hi, h);
        }
        if (lo > hi) {
            lo = su.FirstVParameter();
            hi = su.LastVParameter();
        }
        const bool internal = f.Orientation() == TopAbs_REVERSED;
        rust::Vec<double> v;
        for (double x : {ax.Location().X(), ax.Location().Y(), ax.Location().Z(), ax.Direction().X(), ax.Direction().Y(), ax.Direction().Z(),
                         c.Radius(), lo, hi, internal ? 1.0 : 0.0})
            v.push_back(x);
        return v;
    });
}

std::unique_ptr<Shape> thread_tool_ext(double ox, double oy, double oz, double dx, double dy, double dz,
                                       double major_d, double pitch, double length, double angle_deg) {
    return guard("rosca externa", [&] {
        if (pitch <= 0 || length <= 0 || major_d <= 0) throw std::runtime_error("rosca: passo, comprimento e diâmetro devem ser positivos");
        const double depth = 0.6134 * pitch;
        const double r_maj = major_d / 2;
        const double r_min = r_maj - depth;
        const double half = std::tan(angle_deg / 2 * M_PI / 180.0) * (depth + 0.1 * pitch);
        gp_Dir axis(dx, dy, dz);
        gp_Pnt o(ox, oy, oz);
        gp_Ax3 ax(o, axis);
        Handle(Geom_CylindricalSurface) cyl = new Geom_CylindricalSurface(ax, r_maj);
        const double turns = length / pitch;
        gp_Lin2d l2(gp_Pnt2d(0, 0), gp_Dir2d(2 * M_PI, pitch));
        Handle(Geom2d_Line) line = new Geom2d_Line(l2);
        const double span = turns * std::sqrt(4 * M_PI * M_PI + pitch * pitch);
        TopoDS_Edge helix = BRepBuilderAPI_MakeEdge(line, cyl, 0.0, span).Edge();
        BRepLib::BuildCurves3d(helix);
        TopoDS_Wire spine = BRepBuilderAPI_MakeWire(helix).Wire();
        gp_Dir xd = ax.XDirection();
        auto P = [&](double r, double h) {
            return gp_Pnt(o.X() + xd.X() * r + axis.X() * h, o.Y() + xd.Y() * r + axis.Y() * h, o.Z() + xd.Z() * r + axis.Z() * h);
        };
        const double r_out = r_maj + 0.1 * pitch;  // começa um pouco fora do eixo
        BRepBuilderAPI_MakeWire pw;
        pw.Add(BRepBuilderAPI_MakeEdge(P(r_out, -half), P(r_min, 0)).Edge());
        pw.Add(BRepBuilderAPI_MakeEdge(P(r_min, 0), P(r_out, half)).Edge());
        pw.Add(BRepBuilderAPI_MakeEdge(P(r_out, half), P(r_out, -half)).Edge());
        BRepOffsetAPI_MakePipeShell sweep(spine);
        sweep.SetMode(axis);
        sweep.Add(pw.Wire());
        sweep.Build();
        if (!sweep.IsDone()) throw std::runtime_error("não foi possível gerar a hélice da rosca");
        sweep.MakeSolid();
        return wrap(sweep.Shape());
    });
}

}  // namespace fenkra
