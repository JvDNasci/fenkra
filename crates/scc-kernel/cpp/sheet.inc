// Chapa metálica: aba de aresta (dobra + parede reta) e sua versão planificada.
// Incluído no fim de kernel.cpp, depois de hybrid.inc.

namespace scc {

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
        auto normal_of = [](const TopoDS_Face& f) {
            BRepAdaptor_Surface s(f);
            if (s.GetType() != GeomAbs_Plane) throw std::runtime_error("a aba precisa de faces planas na aresta (parede de chapa)");
            double u1, u2, v1, v2;
            BRepTools::UVBounds(f, u1, u2, v1, v2);
            gp_Pnt q;
            gp_Vec n;
            BRepGProp_Face(f).Normal((u1 + u2) / 2, (v1 + v2) / 2, q, n);
            n.Normalize();
            return n;
        };
        gp_Vec n = normal_of(fa), u = normal_of(fb);
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
        for (double x : {l, t, n.X(), n.Y(), n.Z(), u.X(), u.Y(), u.Z(), c0.X(), c0.Y(), c0.Z(), c1.X(), c1.Y(), c1.Z()}) info.push_back(x);
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

}  // namespace scc
