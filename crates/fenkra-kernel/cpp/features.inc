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

// Curva guia a partir da polilinha: com cantos vivos vira polilinha, senão B-spline suave.
TopoDS_Wire spine_wire(rust::Slice<const double> pts) {
    const size_t n = pts.size() / 3;
    if (n < 2) throw std::runtime_error("curva guia com menos de dois pontos");
    std::vector<gp_Pnt> p;
    for (size_t i = 0; i < n; ++i) p.push_back(P3(pts, i));
    bool corners = false;
    for (size_t i = 1; i + 1 < n; ++i) {
        gp_Vec a(p[i - 1], p[i]), b(p[i], p[i + 1]);
        if (a.Magnitude() > 1e-9 && b.Magnitude() > 1e-9 && a.Angle(b) > 0.35) corners = true;
    }
    BRepBuilderAPI_MakeWire w;
    if (n == 2 || corners) {
        for (size_t i = 0; i + 1 < n; ++i)
            if (p[i].Distance(p[i + 1]) > 1e-9) w.Add(BRepBuilderAPI_MakeEdge(p[i], p[i + 1]).Edge());
    } else {
        TColgp_Array1OfPnt arr(1, static_cast<int>(n));
        for (size_t i = 0; i < n; ++i) arr.SetValue(static_cast<int>(i) + 1, p[i]);
        GeomAPI_PointsToBSpline bs(arr, 3, 8, GeomAbs_C2, 1e-4);
        w.Add(BRepBuilderAPI_MakeEdge(bs.Curve()).Edge());
    }
    if (!w.IsDone()) throw std::runtime_error("curva guia inválida");
    return w.Wire();
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

std::unique_ptr<Shape> sweep_tool(const Shape& face, rust::Slice<const double> spine, uint8_t mode) {
    return guard("varredura", [&] {
        TopoDS_Wire w = spine_wire(spine);
        GeomFill_Trihedron t = mode == 1 ? GeomFill_IsFixed : mode == 2 ? GeomFill_IsFrenet : GeomFill_IsCorrectedFrenet;
        BRepOffsetAPI_MakePipe mk(w, face.s, t, Standard_False);
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("não foi possível varrer o perfil ao longo da curva");
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> loft_tool(const Shape& sections, bool ruled, bool closed) {
    return guard("multi-seções", [&] {
        std::vector<TopoDS_Wire> ws;
        for (TopExp_Explorer e(sections.s, TopAbs_FACE); e.More(); e.Next()) ws.push_back(BRepTools::OuterWire(TopoDS::Face(e.Current())));
        if (ws.size() < 2) throw std::runtime_error("são precisos pelo menos dois perfis");
        BRepOffsetAPI_ThruSections ts(Standard_True, ruled ? Standard_True : Standard_False);
        for (auto& w : ws) ts.AddWire(w);
        if (closed) ts.AddWire(ws[0]);
        ts.CheckCompatibility(Standard_True);
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

std::unique_ptr<Shape> shell_t(const Shape& s, rust::Slice<const double> pts, double thickness, bool outward,
                               rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("casca", [&] {
        if (thickness <= 0) throw std::runtime_error("a espessura precisa ser positiva");
        TopTools_ListOfShape rm;
        for (auto& f : faces_near(s.s, pts)) rm.Append(f);
        BRepOffsetAPI_MakeThickSolid mk;
        mk.MakeThickSolidByJoin(s.s, rm, outward ? thickness : -thickness, 1e-3, BRepOffset_Skin, Standard_False, Standard_False, GeomAbs_Arc);
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("espessura grande demais para a peça");
        return tagged_result(mk, s.s, tags, out);
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

std::unique_ptr<Shape> edges_mode_t(const Shape& s, uint8_t mode, double v1, double v2, rust::Slice<const double> mids, double tol,
                                    rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard(mode == 0 ? "filete variável" : "chanfro", [&] {
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
        if (mode == 0) {
            BRepFilletAPI_MakeFillet mk(s.s);
            for (auto& e : chosen) mk.Add(v1, v2, e);
            mk.Build();
            if (!mk.IsDone()) throw std::runtime_error("raios grandes demais para as arestas");
            return tagged_result(mk, s.s, tags, out);
        }
        BRepFilletAPI_MakeChamfer mk(s.s);
        for (auto& e : chosen) {
            const TopoDS_Face& f = TopoDS::Face(e2f.FindFromKey(e).First());
            if (mode == 1) mk.Add(v1, v2, e, f);
            else mk.AddDA(v1, v2 * M_PI / 180.0, e, f);
        }
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("valores grandes demais para as arestas");
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
