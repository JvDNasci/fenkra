// Fachada C++ do SolidCadCraft sobre o OpenCASCADE. Ver kernel.h.
#include "kernel.h"
#include "scc-kernel/src/ffi.rs.h"

#include <algorithm>
#include <cmath>
#include <stdexcept>
#include <string>
#include <vector>

#include <BRepAdaptor_Curve.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBndLib.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepClass_FaceClassifier.hxx>
#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepPrimAPI_MakeCone.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepTools_History.hxx>
#include <TopTools_ListOfShape.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <GCPnts_TangentialDeflection.hxx>
#include <GC_MakeArcOfCircle.hxx>
#include <GProp_GProps.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Poly_Triangulation.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <ShapeUpgrade_UnifySameDomain.hxx>
#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <StlAPI_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <gp_Ax2.hxx>
#include <gp_Circ.hxx>
#include <gp_Pln.hxx>
#include <gp_Trsf.hxx>

namespace scc {

namespace {

// Converte exceções do OCCT em std::runtime_error (que o cxx transforma em Err no Rust).
template <class F>
auto guard(const char* what, F&& f) -> decltype(f()) {
    try {
        return f();
    } catch (const Standard_Failure& e) {
        std::string m = e.GetMessageString() ? e.GetMessageString() : "";
        throw std::runtime_error(std::string(what) + ": " + (m.empty() ? std::string("erro geométrico do kernel") : m));
    }
}

std::unique_ptr<Shape> wrap(const TopoDS_Shape& s) {
    auto r = std::make_unique<Shape>();
    r->s = s;
    return r;
}

std::string to_std(rust::Str s) { return std::string(s.data(), s.size()); }

// Une faces/arestas coplanares e colineares geradas pelas booleanas.
TopoDS_Shape unify(const TopoDS_Shape& s) {
    ShapeUpgrade_UnifySameDomain u(s, Standard_True, Standard_True, Standard_False);
    u.Build();
    return u.Shape();
}

double area_of(const TopoDS_Shape& f) {
    GProp_GProps p;
    BRepGProp::SurfaceProperties(f, p);
    return std::abs(p.Mass());
}

}  // namespace

std::unique_ptr<Shape> shape_clone(const Shape& s) { return wrap(s.s); }
bool shape_is_null(const Shape& s) { return s.s.IsNull(); }
std::unique_ptr<Shape> shape_empty() { return std::make_unique<Shape>(); }

std::unique_ptr<Shape> make_face(const Frame& fr, rust::Slice<const Seg> segs, rust::Slice<const uint32_t> loop_ends) {
    return guard("perfil", [&] {
        gp_Pnt O(fr.ox, fr.oy, fr.oz);
        gp_Dir N(fr.nx, fr.ny, fr.nz);
        gp_Dir X(fr.xx, fr.xy, fr.xz);
        gp_Dir Y = N.Crossed(X);
        auto P = [&](double u, double v) {
            return gp_Pnt(O.X() + u * X.X() + v * Y.X(), O.Y() + u * X.Y() + v * Y.Y(), O.Z() + u * X.Z() + v * Y.Z());
        };

        // um face por laço
        std::vector<TopoDS_Face> faces;
        size_t start = 0;
        for (uint32_t end : loop_ends) {
            BRepBuilderAPI_MakeWire mw;
            for (size_t i = start; i < end; ++i) {
                const Seg& g = segs[i];
                if (g.kind == 0) {
                    mw.Add(BRepBuilderAPI_MakeEdge(P(g.x1, g.y1), P(g.x2, g.y2)).Edge());
                } else if (g.kind == 1) {
                    Handle(Geom_TrimmedCurve) arc = GC_MakeArcOfCircle(P(g.x1, g.y1), P(g.x2, g.y2), P(g.x3, g.y3)).Value();
                    mw.Add(BRepBuilderAPI_MakeEdge(arc).Edge());
                } else {
                    gp_Circ c(gp_Ax2(P(g.x1, g.y1), N, X), g.x2);
                    mw.Add(BRepBuilderAPI_MakeEdge(c).Edge());
                }
            }
            start = end;
            if (!mw.IsDone()) throw std::runtime_error("laço do sketch não está fechado ou conectado");
            BRepBuilderAPI_MakeFace mf(gp_Pln(O, N), mw.Wire(), Standard_True);
            if (!mf.IsDone()) throw std::runtime_error("não foi possível criar face do laço");
            faces.push_back(mf.Face());
        }
        if (faces.empty()) throw std::runtime_error("sketch sem perfis fechados");

        // laços internos viram furos: profundidade par = material, ímpar = vazio
        std::vector<double> area;
        for (auto& f : faces) area.push_back(area_of(f));
        std::vector<size_t> order(faces.size());
        for (size_t i = 0; i < order.size(); ++i) order[i] = i;
        std::sort(order.begin(), order.end(), [&](size_t a, size_t b) { return area[a] > area[b]; });

        TopoDS_Shape result;
        for (size_t k = 0; k < order.size(); ++k) {
            const TopoDS_Face& fi = faces[order[k]];
            TopExp_Explorer vx(fi, TopAbs_VERTEX);
            gp_Pnt p = BRep_Tool::Pnt(TopoDS::Vertex(vx.Current()));
            int depth = 0;
            for (size_t j = 0; j < k; ++j) {
                BRepClass_FaceClassifier cls(faces[order[j]], p, 1e-6);
                if (cls.State() == TopAbs_IN || cls.State() == TopAbs_ON) depth++;
            }
            if (result.IsNull()) {
                result = fi;
            } else if (depth % 2 == 0) {
                result = BRepAlgoAPI_Fuse(result, fi).Shape();
            } else {
                result = BRepAlgoAPI_Cut(result, fi).Shape();
            }
        }
        return wrap(unify(result));
    });
}

std::unique_ptr<Shape> extrude(const Shape& face, double dx, double dy, double dz, double front, double back) {
    return guard("extrusão", [&] {
        if (front + back <= 1e-9) throw std::runtime_error("comprimento da extrusão deve ser maior que zero");
        gp_Vec d(dx, dy, dz);
        d.Normalize();
        gp_Trsf t;
        t.SetTranslation(-back * d);
        TopoDS_Shape moved = BRepBuilderAPI_Transform(face.s, t, Standard_True).Shape();
        BRepPrimAPI_MakePrism mk(moved, (front + back) * d);
        if (!mk.IsDone()) throw std::runtime_error("falha na extrusão");
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> revolve(const Shape& face, double ox, double oy, double oz, double dx, double dy, double dz,
                               double angle_deg, double start_deg) {
    return guard("revolução", [&] {
        gp_Ax1 ax(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz));
        TopoDS_Shape f = face.s;
        if (std::abs(start_deg) > 1e-9) {
            gp_Trsf r;
            r.SetRotation(ax, start_deg * M_PI / 180.0);
            f = BRepBuilderAPI_Transform(f, r, Standard_True).Shape();
        }
        if (std::abs(angle_deg) >= 360.0 - 1e-9) {
            BRepPrimAPI_MakeRevol mk(f, ax);
            return wrap(mk.Shape());
        }
        if (std::abs(angle_deg) < 1e-9) throw std::runtime_error("ângulo da revolução deve ser maior que zero");
        BRepPrimAPI_MakeRevol mk(f, ax, angle_deg * M_PI / 180.0);
        if (!mk.IsDone()) throw std::runtime_error("falha na revolução");
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> boolean(const Shape& a, const Shape& b, uint8_t op) {
    return guard("booleana", [&] {
        TopoDS_Shape r;
        bool err = false;
        if (op == 0) {
            BRepAlgoAPI_Fuse m(a.s, b.s);
            err = m.HasErrors();
            r = m.Shape();
        } else if (op == 1) {
            BRepAlgoAPI_Cut m(a.s, b.s);
            err = m.HasErrors();
            r = m.Shape();
        } else {
            BRepAlgoAPI_Common m(a.s, b.s);
            err = m.HasErrors();
            r = m.Shape();
        }
        if (err || r.IsNull()) throw std::runtime_error("a operação booleana falhou");
        return wrap(unify(r));
    });
}

std::unique_ptr<Shape> hole_tool(double ox, double oy, double oz, double dx, double dy, double dz,
                                 double diameter, double depth, uint8_t kind, double d2, double p2) {
    return guard("furo", [&] {
        if (diameter <= 0 || depth <= 0) throw std::runtime_error("diâmetro e profundidade devem ser positivos");
        gp_Dir d(dx, dy, dz);
        const double lead = 0.05;  // começa um pouco fora da face para cortar a superfície com folga
        gp_Pnt o(ox - d.X() * lead, oy - d.Y() * lead, oz - d.Z() * lead);
        gp_Ax2 ax(o, d);
        TopoDS_Shape tool = BRepPrimAPI_MakeCylinder(ax, diameter / 2, depth + lead).Shape();
        if (kind == 1 && d2 > diameter && p2 > 0) {  // rebaixo
            tool = BRepAlgoAPI_Fuse(tool, BRepPrimAPI_MakeCylinder(ax, d2 / 2, p2 + lead).Shape()).Shape();
        } else if (kind == 2 && d2 > diameter && p2 > 0 && p2 < 180) {  // escareado
            double h = (d2 - diameter) / 2 / std::tan(p2 / 2 * M_PI / 180.0);
            gp_Pnt oc(ox, oy, oz);
            TopoDS_Shape cone = BRepPrimAPI_MakeCone(gp_Ax2(oc, d), d2 / 2, diameter / 2, h).Shape();
            TopoDS_Shape cap = BRepPrimAPI_MakeCylinder(ax, d2 / 2, lead).Shape();
            tool = BRepAlgoAPI_Fuse(BRepAlgoAPI_Fuse(tool, cone).Shape(), cap).Shape();
        }
        return wrap(tool);
    });
}

std::unique_ptr<Shape> fillet_all(const Shape& s, double radius) {
    return guard("filete", [&] {
        BRepFilletAPI_MakeFillet mk(s.s);
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        for (int i = 1; i <= edges.Extent(); ++i) mk.Add(radius, TopoDS::Edge(edges(i)));
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("raio grande demais para a geometria");
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> chamfer_all(const Shape& s, double dist) {
    return guard("chanfro", [&] {
        BRepFilletAPI_MakeChamfer mk(s.s);
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        for (int i = 1; i <= edges.Extent(); ++i) mk.Add(dist, TopoDS::Edge(edges(i)));
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("distância grande demais para a geometria");
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> compound(const Shape& a, const Shape& b) {
    TopoDS_Compound c;
    BRep_Builder bb;
    bb.MakeCompound(c);
    if (!a.s.IsNull()) bb.Add(c, a.s);
    if (!b.s.IsNull()) bb.Add(c, b.s);
    return wrap(c);
}

namespace {

// Propaga etiquetas (dono de cada face) de `src` para `res` usando o histórico da operação.
// Faces sem origem em `src` recebem `fresh` (criadas pela operação).
template <class Mod>
std::vector<int> carry(const TopoDS_Shape& src, const std::vector<int>& tags, const TopoDS_Shape& res, Mod&& modified, int fresh) {
    TopTools_IndexedMapOfShape sm, rm;
    TopExp::MapShapes(src, TopAbs_FACE, sm);
    TopExp::MapShapes(res, TopAbs_FACE, rm);
    std::vector<int> out(rm.Extent(), fresh);
    for (int i = 1; i <= sm.Extent(); ++i) {
        const int tag = i - 1 < static_cast<int>(tags.size()) ? tags[i - 1] : fresh;
        const TopoDS_Shape& f = sm(i);
        const int k = rm.FindIndex(f);
        if (k > 0) out[k - 1] = tag;
        for (const TopoDS_Shape& img : modified(f)) {
            const int j = rm.FindIndex(img);
            if (j > 0) out[j - 1] = tag;
        }
    }
    return out;
}

std::vector<int> to_vec(rust::Slice<const int32_t> s) { return std::vector<int>(s.begin(), s.end()); }

void fill(rust::Vec<int32_t>& out, const std::vector<int>& v) {
    out.clear();
    for (int x : v) out.push_back(x);
}

template <class Mk>
std::unique_ptr<Shape> edge_op_t(const char* what, const Shape& s, double v, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard(what, [&] {
        Mk mk(s.s);
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        for (int i = 1; i <= edges.Extent(); ++i) mk.Add(v, TopoDS::Edge(edges(i)));
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("valor grande demais para a geometria");
        fill(out, carry(s.s, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return mk.Modified(f); }, -1));
        return wrap(mk.Shape());
    });
}

}  // namespace

std::unique_ptr<Shape> boolean_t(const Shape& a, const Shape& b, uint8_t op, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard("booleana", [&] {
        std::unique_ptr<BRepAlgoAPI_BooleanOperation> m;
        if (op == 0) m.reset(new BRepAlgoAPI_Fuse(a.s, b.s));
        else if (op == 1) m.reset(new BRepAlgoAPI_Cut(a.s, b.s));
        else m.reset(new BRepAlgoAPI_Common(a.s, b.s));
        if (m->HasErrors() || m->Shape().IsNull()) throw std::runtime_error("a operação booleana falhou");
        const TopoDS_Shape r = m->Shape();
        auto t1 = carry(a.s, to_vec(tags), r, [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return m->Modified(f); }, -1);
        ShapeUpgrade_UnifySameDomain u(r, Standard_True, Standard_True, Standard_False);
        u.Build();
        Handle(BRepTools_History) h = u.History();
        auto t2 = carry(r, t1, u.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return h->Modified(f); }, -1);
        fill(out, t2);
        return wrap(u.Shape());
    });
}

std::unique_ptr<Shape> fillet_all_t(const Shape& s, double r, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return edge_op_t<BRepFilletAPI_MakeFillet>("filete", s, r, tags, out);
}

std::unique_ptr<Shape> chamfer_all_t(const Shape& s, double d, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return edge_op_t<BRepFilletAPI_MakeChamfer>("chanfro", s, d, tags, out);
}

MeshData tessellate(const Shape& s, double deflection) {
    return guard("tesselação", [&] {
        MeshData m;
        if (s.s.IsNull()) return m;
        BRepMesh_IncrementalMesh(s.s, deflection, Standard_False, 0.35, Standard_True);

        // mesma ordem de faces usada pelas etiquetas (IndexedMap)
        TopTools_IndexedMapOfShape fmap;
        TopExp::MapShapes(s.s, TopAbs_FACE, fmap);
        for (int fi = 1; fi <= fmap.Extent(); ++fi) {
            m.face_ranges.push_back(static_cast<uint32_t>(m.indices.size()));
            const TopoDS_Face& face = TopoDS::Face(fmap(fi));
            TopLoc_Location loc;
            Handle(Poly_Triangulation) tri = BRep_Tool::Triangulation(face, loc);
            if (tri.IsNull()) continue;
            const gp_Trsf tr = loc.Transformation();
            const bool rev = face.Orientation() == TopAbs_REVERSED;
            const uint32_t base = static_cast<uint32_t>(m.positions.size() / 3);
            const int nn = tri->NbNodes();
            std::vector<gp_Vec> nrm(nn, gp_Vec(0, 0, 0));
            std::vector<gp_Pnt> pts(nn);
            for (int i = 1; i <= nn; ++i) pts[i - 1] = tri->Node(i).Transformed(tr);
            for (int t = 1; t <= tri->NbTriangles(); ++t) {
                int a, b, c;
                tri->Triangle(t).Get(a, b, c);
                if (rev) std::swap(b, c);
                gp_Vec n = gp_Vec(pts[a - 1], pts[b - 1]).Crossed(gp_Vec(pts[a - 1], pts[c - 1]));
                nrm[a - 1] += n;
                nrm[b - 1] += n;
                nrm[c - 1] += n;
                m.indices.push_back(base + a - 1);
                m.indices.push_back(base + b - 1);
                m.indices.push_back(base + c - 1);
            }
            for (int i = 0; i < nn; ++i) {
                m.positions.push_back(float(pts[i].X()));
                m.positions.push_back(float(pts[i].Y()));
                m.positions.push_back(float(pts[i].Z()));
                gp_Vec n = nrm[i];
                if (n.Magnitude() > 1e-12) n.Normalize();
                m.normals.push_back(float(n.X()));
                m.normals.push_back(float(n.Y()));
                m.normals.push_back(float(n.Z()));
            }
        }

        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        for (int i = 1; i <= edges.Extent(); ++i) {
            const TopoDS_Edge& e = TopoDS::Edge(edges(i));
            if (BRep_Tool::Degenerated(e)) continue;
            BRepAdaptor_Curve c(e);
            GCPnts_TangentialDeflection d(c, 0.2, deflection);
            for (int k = 1; k < d.NbPoints(); ++k) {
                gp_Pnt p = d.Value(k), q = d.Value(k + 1);
                for (double v : {p.X(), p.Y(), p.Z(), q.X(), q.Y(), q.Z()}) m.edges.push_back(float(v));
            }
        }
        return m;
    });
}

rust::Vec<double> bounding_box(const Shape& s) {
    rust::Vec<double> v;
    if (s.s.IsNull()) return v;
    Bnd_Box b;
    BRepBndLib::Add(s.s, b);
    if (b.IsVoid()) return v;
    double x0, y0, z0, x1, y1, z1;
    b.Get(x0, y0, z0, x1, y1, z1);
    for (double x : {x0, y0, z0, x1, y1, z1}) v.push_back(x);
    return v;
}

rust::Vec<double> mass_props(const Shape& s) {
    rust::Vec<double> v;
    if (s.s.IsNull()) return v;
    GProp_GProps vp, sp;
    BRepGProp::VolumeProperties(s.s, vp);
    BRepGProp::SurfaceProperties(s.s, sp);
    gp_Pnt c = vp.CentreOfMass();
    for (double x : {vp.Mass(), sp.Mass(), c.X(), c.Y(), c.Z()}) v.push_back(x);
    return v;
}

uint32_t count_faces(const Shape& s) {
    TopTools_IndexedMapOfShape f;
    TopExp::MapShapes(s.s, TopAbs_FACE, f);
    return static_cast<uint32_t>(f.Extent());
}

void export_step(const Shape& s, rust::Str path) {
    guard("STEP", [&] {
        STEPControl_Writer w;
        if (w.Transfer(s.s, STEPControl_AsIs) != IFSelect_RetDone) throw std::runtime_error("falha ao converter para STEP");
        if (w.Write(to_std(path).c_str()) != IFSelect_RetDone) throw std::runtime_error("falha ao gravar o arquivo STEP");
    });
}

void export_stl(const Shape& s, rust::Str path) {
    guard("STL", [&] {
        BRepMesh_IncrementalMesh(s.s, 0.05, Standard_False, 0.3, Standard_True);
        StlAPI_Writer w;
        if (!w.Write(s.s, to_std(path).c_str())) throw std::runtime_error("falha ao gravar o arquivo STL");
    });
}

std::unique_ptr<Shape> import_step(rust::Str path) {
    return guard("STEP", [&] {
        STEPControl_Reader r;
        if (r.ReadFile(to_std(path).c_str()) != IFSelect_RetDone) throw std::runtime_error("não foi possível ler o STEP");
        r.TransferRoots();
        return wrap(r.OneShape());
    });
}

rust::String occt_version() { return rust::String(OCC_VERSION_COMPLETE); }

}  // namespace scc
