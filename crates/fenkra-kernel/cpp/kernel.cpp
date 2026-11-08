// SPDX-License-Identifier: LGPL-2.1-or-later
// Fachada C++ do Fenkra sobre o OpenCASCADE. Ver kernel.h.
#include "kernel.h"
#include "fenkra-kernel/src/ffi.rs.h"

#include <algorithm>
#include <array>
#include <functional>
#include <map>
#include <atomic>
#include <cstdio>
#include <cmath>
#include <fstream>
#include <limits>
#include <mutex>
#include <optional>
#include <unordered_map>
#include <shared_mutex>
#include <stdexcept>
#include <string>
#include <vector>

#include <BRepAdaptor_Curve.hxx>
#include <BRepAlgoAPI_Splitter.hxx>
#include <BRepAlgoAPI_Section.hxx>
#include <IntCurvesFace_ShapeIntersector.hxx>
#include <gp_Lin.hxx>
#include <sstream>
#include <HLRBRep_Algo.hxx>
#include <HLRBRep_HLRToShape.hxx>
#include <HLRAlgo_Projector.hxx>
#include <HLRBRep_PolyAlgo.hxx>
#include <HLRBRep_PolyHLRToShape.hxx>
#include <BRepBuilderAPI_GTransform.hxx>
#include <gp_GTrsf.hxx>
#include <gp_Mat.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <BOPAlgo_MakerVolume.hxx>
#include <GeomConvert.hxx>
#include <GeomLib.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Geom_BoundedSurface.hxx>
#include <ShapeAnalysis_FreeBounds.hxx>
#include <ShapeFix_Shape.hxx>
#include <ShapeFix_Solid.hxx>
#include <TopTools_HSequenceOfShape.hxx>
#include <TopoDS_Iterator.hxx>
#include <Geom_RectangularTrimmedSurface.hxx>
#include <Precision.hxx>
#include <gp_Pln.hxx>
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepOffsetAPI_MakeOffsetShape.hxx>
#include <BRepOffsetAPI_MakeFilling.hxx>
#include <BRepGProp_Face.hxx>
#include <GeomAPI_ProjectPointOnSurf.hxx>
#include <Geom_Surface.hxx>
#include <BRepExtrema_DistShapeShape.hxx>
#include <BRepTopAdaptor_FClass2d.hxx>
#include <GeomAPI_Interpolate.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BSplineSurface.hxx>
#include <GeomLProp_SLProps.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepOffsetAPI_MakePipe.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <BRepOffsetAPI_DraftAngle.hxx>
#include <BRepAlgoAPI_Defeaturing.hxx>
#include <TColgp_Array1OfPnt2d.hxx>
#include <TColStd_Array1OfInteger.hxx>
#include <TColStd_Array1OfReal.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepOffset_MakeOffset.hxx>
#include <BRepOffsetAPI_MakeOffset.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>
#include <BRepPrimAPI_MakeWedge.hxx>
#include <BRepTools.hxx>
#include <BinTools.hxx>
#include <GeomAPI_PointsToBSpline.hxx>
#include <GeomAPI_Interpolate.hxx>
#include <GeomAPI_IntCS.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <gp_Circ.hxx>
#include <gp_Elips.hxx>
#include <TColgp_HArray1OfPnt.hxx>
#include <Geom_BSplineCurve.hxx>
#include <GeomFill_Trihedron.hxx>
#include <GeomLProp_CLProps.hxx>
#include <GProp_PEquation.hxx>
#include <TColgp_Array1OfPnt.hxx>
#include <TopoDS_Vertex.hxx>
#include <gp_Cylinder.hxx>
#include <type_traits>
#if defined(_MSC_VER)
#include <excpt.h>
#endif
#include <vector>
#include <BRepAdaptor_Surface.hxx>
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
#include <IMeshTools_MeshAlgoType.hxx>
#include <BRepPrimAPI_MakeCone.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakeHalfSpace.hxx>
#include <BRepOffsetAPI_MakePipeShell.hxx>
#include <BRepLib.hxx>
#include <Geom_CylindricalSurface.hxx>
#include <Geom2d_Line.hxx>
#include <gp_Ax3.hxx>
#include <gp_Lin2d.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepTools_History.hxx>
#include <TopTools_ListOfShape.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <GCPnts_TangentialDeflection.hxx>
#include <GCPnts_AbscissaPoint.hxx>
#include <GeomAdaptor_Curve.hxx>
#include <GC_MakeArcOfCircle.hxx>
#include <GProp_GProps.hxx>
#include <GProp_PrincipalProps.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Geom_Plane.hxx>
#include <Geom2d_Curve.hxx>
#include <IMeshTools_Parameters.hxx>
#include <OSD_Parallel.hxx>
#include <OSD_ThreadPool.hxx>
#include <Poly_Triangulation.hxx>
#include <Poly_TriangulationParameters.hxx>
#include <HeaderSection_FileSchema.hxx>
#include <NCollection_Sequence.hxx>
#include <STEPControl_Reader.hxx>
#include <StepData_StepModel.hxx>
#include <TCollection_AsciiString.hxx>
#include <TCollection_HAsciiString.hxx>
#include <STEPControl_Writer.hxx>
#include <ShapeUpgrade_UnifySameDomain.hxx>
#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <OSD_OpenFile.hxx>
#include <StlAPI_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <gp_Ax2.hxx>
#include <gp_Pnt2d.hxx>
#include <gp_Circ.hxx>
#include <gp_Pln.hxx>
#include <gp_Trsf.hxx>
#include <Message_ProgressIndicator.hxx>
#include <Message_ProgressRange.hxx>

namespace fenkra {

#include <chrono>

namespace {

// Cancelamento (B259): a thread de cálculo registra a bandeira do pedido em andamento; as operações
// longas do OpenCASCADE recebem uma Message_ProgressRange ligada a ela e abandonam o cálculo quando
// a bandeira sobe. Fora de uma thread registrada nada muda (a faixa é vazia).
thread_local const std::atomic<bool>* g_cancelar = nullptr;

// Limite de tempo (B337): mesma saída do cancelamento, para o comando que pode demorar sem fim útil.
using Relogio = std::chrono::steady_clock;
thread_local bool g_tem_prazo = false;
thread_local Relogio::time_point g_prazo;
thread_local double g_prazo_s = 0;

bool cancelado_agora() { return g_cancelar && g_cancelar->load(std::memory_order_relaxed); }
bool prazo_vencido() { return g_tem_prazo && Relogio::now() >= g_prazo; }

// A booleana chama `UserBreak` também dos seus fios paralelos, onde os `thread_local` acima estão
// vazios: por isso o indicador guarda a própria cópia da bandeira e do prazo.
class IndicadorCancelavel : public Message_ProgressIndicator {
public:
    const std::atomic<bool>* bandeira = nullptr;
    bool tem_prazo = false;
    Relogio::time_point prazo;
    bool UserBreak() override {
        return (bandeira && bandeira->load(std::memory_order_relaxed)) || (tem_prazo && Relogio::now() >= prazo);
    }
    void Show(const Message_ProgressScope&, const bool) override {}
};

Message_ProgressRange faixa_cancelavel() {
    if (!g_cancelar && !g_tem_prazo) return Message_ProgressRange();
    thread_local Handle(IndicadorCancelavel) ind;
    if (ind.IsNull()) ind = new IndicadorCancelavel();
    ind->bandeira = g_cancelar;
    ind->tem_prazo = g_tem_prazo;
    ind->prazo = g_prazo;
    return ind->Start();
}

// Depois de uma operação com faixa: se foi interrompida, o resultado (parcial) não pode seguir.
void falhar_se_cancelado() {
    if (cancelado_agora()) throw std::runtime_error("cancelado");
    if (prazo_vencido()) {
        char m[120];
        std::snprintf(m, sizeof m, "limite de tempo: a operação passou de %.0f s e foi interrompida; simplifique a geometria", g_prazo_s);
        throw std::runtime_error(m);
    }
}

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

// Alguns algoritmos do OCCT seguem depois de uma etapa interna falhar e leem uma forma nula: isso é
// violação de acesso, não exceção C++, e nem `guard` nem o cxx a enxergam, então o processo morreria.
// `sem_violacao` devolve false nesse caso; outras exceções (C++ inclusive) seguem o caminho normal.
#if defined(_MSC_VER)
constexpr bool kViolacaoProtegida = true;
bool sem_violacao(void (*f)(void*), void* ctx) {
    __try {
        f(ctx);
        return true;
    } __except (GetExceptionCode() == 0xC0000005u ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        return false;
    }
}
#else
// Sem proteção aqui: a violação derruba o processo. Quem pode evitar o caminho arriscado deve evitar.
// A saída no Linux é `OSD::SetSignal` com `OCC_CATCH_SIGNALS`, ainda não escrita por falta de como testar.
constexpr bool kViolacaoProtegida = false;
bool sem_violacao(void (*f)(void*), void* ctx) {
    f(ctx);
    return true;
}
#endif

// Roda `f`; se o OCCT tentar ler memória inválida lá dentro, vira erro com `mensagem`.
template <class F>
void protegido(const char* mensagem, F&& f) {
    using Fn = std::remove_reference_t<F>;
    if (!sem_violacao([](void* c) { (*static_cast<Fn*>(c))(); }, &f)) throw std::runtime_error(mensagem);
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

// Fórmula da aresta, em kEdgeGeom números: tipo (0 sem fórmula, 1 reta, 2 cônica) e dados.
// Reta: as duas pontas. Cônica: centro, u, v, t0 e t1 de P(t) = c + u cos t + v sin t.
constexpr int kEdgeGeom = 12;
static void push_edge_geom(const BRepAdaptor_Curve& c, rust::Vec<double>& out) {
    double g[kEdgeGeom] = {0};
    const double t0 = c.FirstParameter(), t1 = c.LastParameter();
    auto put = [&](int at, const gp_XYZ& v) {
        g[at] = v.X();
        g[at + 1] = v.Y();
        g[at + 2] = v.Z();
    };
    switch (c.GetType()) {
        case GeomAbs_Line:
            g[0] = 1;
            put(1, c.Value(t0).XYZ());
            put(4, c.Value(t1).XYZ());
            break;
        case GeomAbs_Circle: {
            const gp_Circ k = c.Circle();
            g[0] = 2;
            put(1, k.Location().XYZ());
            put(4, k.Position().XDirection().XYZ() * k.Radius());
            put(7, k.Position().YDirection().XYZ() * k.Radius());
            g[10] = t0;
            g[11] = t1;
            break;
        }
        case GeomAbs_Ellipse: {
            const gp_Elips k = c.Ellipse();
            g[0] = 2;
            put(1, k.Location().XYZ());
            put(4, k.Position().XDirection().XYZ() * k.MajorRadius());
            put(7, k.Position().YDirection().XYZ() * k.MinorRadius());
            g[10] = t0;
            g[11] = t1;
            break;
        }
        default:
            break;
    }
    for (double v : g) out.push_back(v);
}

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
            std::vector<gp_Pnt> spl;
            for (size_t i = start; i < end; ++i) {
                const Seg& g = segs[i];
                if (g.kind == 3) {
                    // pontos da spline até o marcador de fim: B-spline interpolada por eles
                    spl.push_back(P(g.x1, g.y1));
                    if (g.x2 > 0.5) {
                        if (spl.size() == 2) {
                            mw.Add(BRepBuilderAPI_MakeEdge(spl[0], spl[1]).Edge());
                        } else if (spl.size() > 2) {
                            Handle(TColgp_HArray1OfPnt) arr = new TColgp_HArray1OfPnt(1, static_cast<int>(spl.size()));
                            for (size_t k = 0; k < spl.size(); ++k) arr->SetValue(static_cast<int>(k) + 1, spl[k]);
                            GeomAPI_Interpolate it(arr, Standard_False, 1e-7);
                            it.Perform();
                            if (!it.IsDone()) throw std::runtime_error("spline do sketch não pôde ser criada");
                            mw.Add(BRepBuilderAPI_MakeEdge(it.Curve()).Edge());
                        }
                        spl.clear();
                    }
                } else if (g.kind == 0) {
                    mw.Add(BRepBuilderAPI_MakeEdge(P(g.x1, g.y1), P(g.x2, g.y2)).Edge());
                } else if (g.kind == 1) {
                    Handle(Geom_TrimmedCurve) arc = GC_MakeArcOfCircle(P(g.x1, g.y1), P(g.x2, g.y2), P(g.x3, g.y3)).Value();
                    mw.Add(BRepBuilderAPI_MakeEdge(arc).Edge());
                } else if (g.kind == 4) {
                    // elipse: centro, semieixo maior (vetor), semieixo menor e, no registro seguinte, t0 e t1
                    if (i + 1 >= end) throw std::runtime_error("elipse do sketch incompleta");
                    const Seg& h = segs[++i];
                    const gp_XYZ d = X.XYZ() * g.x2 + Y.XYZ() * g.y2;
                    gp_Elips el(gp_Ax2(P(g.x1, g.y1), N, gp_Dir(d)), d.Modulus(), g.x3);
                    if (h.x2 - h.x1 >= 2 * M_PI - 1e-9)
                        mw.Add(BRepBuilderAPI_MakeEdge(el).Edge());
                    else
                        mw.Add(BRepBuilderAPI_MakeEdge(el, h.x1, h.x2).Edge());
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

namespace {

// Papel de cada face do sólido gerado a partir de um perfil (na ordem de faces do resultado, a mesma
// de face_ranges): 0 = tampa inicial, 1 = tampa final, 2+i = lateral gerada pela aresta i do perfil
// (ordem de TopExp::MapShapes das arestas do perfil), -1 = desconhecido.
template <class Mk>
void sweep_roles(Mk& mk, const TopoDS_Shape& profile, rust::Vec<int32_t>& out) {
    TopTools_IndexedMapOfShape rm, pe;
    TopExp::MapShapes(mk.Shape(), TopAbs_FACE, rm);
    TopExp::MapShapes(profile, TopAbs_EDGE, pe);
    std::vector<int> role(rm.Extent(), -1);
    auto mark = [&](const TopoDS_Shape& s, int r) {
        if (s.IsNull()) return;
        for (TopExp_Explorer e(s, TopAbs_FACE); e.More(); e.Next()) {
            const int k = rm.FindIndex(e.Current());
            if (k > 0) role[k - 1] = r;
        }
    };
    mark(mk.FirstShape(), 0);
    mark(mk.LastShape(), 1);
    TopTools_IndexedDataMapOfShapeListOfShape e2f;
    TopExp::MapShapesAndAncestors(mk.Shape(), TopAbs_EDGE, TopAbs_FACE, e2f);
    std::vector<int> pending;
    for (int i = 1; i <= pe.Extent(); ++i) {
        const TopTools_ListOfShape& gen = mk.Generated(pe(i));
        for (const TopoDS_Shape& g : gen) mark(g, 2 + i - 1);
        if (gen.IsEmpty()) pending.push_back(i);
    }
    // o OCCT não informa a face gerada por algumas arestas (ex.: perpendiculares ao eixo numa
    // revolução). Nesse caso a lateral é a face ainda sem papel que encosta nos arcos gerados pelas
    // duas pontas da aresta (ou na única ponta que gerou arco, se a outra está sobre o eixo).
    auto faces_of_vertex = [&](const TopoDS_Shape& v) {
        std::vector<int> ids;
        for (const TopoDS_Shape& g : mk.Generated(v)) {
            const int k = e2f.FindIndex(g);
            if (k == 0) continue;
            for (const TopoDS_Shape& f : e2f(k)) {
                const int j = rm.FindIndex(f);
                if (j > 0 && role[j - 1] < 0) ids.push_back(j - 1);
            }
        }
        return ids;
    };
    for (int i : pending) {
        TopoDS_Vertex v1, v2;
        TopExp::Vertices(TopoDS::Edge(pe(i)), v1, v2);
        std::vector<int> a = faces_of_vertex(v1), b = faces_of_vertex(v2), c;
        if (a.empty()) c = b;
        else if (b.empty()) c = a;
        else
            for (int j : a)
                if (std::find(b.begin(), b.end(), j) != b.end()) c.push_back(j);
        for (int j : c) role[j] = 2 + i - 1;
    }
    out.clear();
    for (int r : role) out.push_back(r);
}

}  // namespace

std::unique_ptr<Shape> extrude(const Shape& face, double dx, double dy, double dz, double front, double back, rust::Vec<int32_t>& roles) {
    return guard("extrusão", [&] {
        if (front + back <= 1e-9) throw std::runtime_error("comprimento da extrusão deve ser maior que zero");
        gp_Vec d(dx, dy, dz);
        d.Normalize();
        gp_Trsf t;
        t.SetTranslation(-back * d);
        TopoDS_Shape moved = BRepBuilderAPI_Transform(face.s, t, Standard_True).Shape();
        BRepPrimAPI_MakePrism mk(moved, (front + back) * d);
        if (!mk.IsDone()) throw std::runtime_error("falha na extrusão");
        sweep_roles(mk, moved, roles);
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> revolve(const Shape& face, double ox, double oy, double oz, double dx, double dy, double dz,
                               double angle_deg, double start_deg, rust::Vec<int32_t>& roles) {
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
            sweep_roles(mk, f, roles);
            return wrap(mk.Shape());
        }
        if (std::abs(angle_deg) < 1e-9) throw std::runtime_error("ângulo da revolução deve ser maior que zero");
        BRepPrimAPI_MakeRevol mk(f, ax, angle_deg * M_PI / 180.0);
        if (!mk.IsDone()) throw std::runtime_error("falha na revolução");
        sweep_roles(mk, f, roles);
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> boolean(const Shape& a, const Shape& b, uint8_t op) {
    return guard("booleana", [&] {
        TopoDS_Shape r;
        bool err = false;
        if (op == 0) {
            BRepAlgoAPI_Fuse m(a.s, b.s, faixa_cancelavel());
            err = m.HasErrors();
            r = m.Shape();
        } else if (op == 1) {
            BRepAlgoAPI_Cut m(a.s, b.s, faixa_cancelavel());
            err = m.HasErrors();
            r = m.Shape();
        } else {
            BRepAlgoAPI_Common m(a.s, b.s, faixa_cancelavel());
            err = m.HasErrors();
            r = m.Shape();
        }
        falhar_se_cancelado();
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
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        TopTools_IndexedDataMapOfShapeListOfShape e2f;
        TopExp::MapShapesAndAncestors(s.s, TopAbs_EDGE, TopAbs_FACE, e2f);
        // "todas": só arestas simples (reta, círculo, elipse) entre superfícies analíticas;
        // hélices e faces livres (ex.: rosca modelada) ficam de fora
        auto simple = [&](const TopoDS_Edge& e) {
            if (BRep_Tool::Degenerated(e)) return false;
            const GeomAbs_CurveType t = BRepAdaptor_Curve(e).GetType();
            if (t != GeomAbs_Line && t != GeomAbs_Circle && t != GeomAbs_Ellipse) return false;
            if (!e2f.Contains(e)) return false;
            for (const TopoDS_Shape& f : e2f.FindFromKey(e)) {
                const GeomAbs_SurfaceType st = BRepAdaptor_Surface(TopoDS::Face(f)).GetType();
                if (st != GeomAbs_Plane && st != GeomAbs_Cylinder && st != GeomAbs_Cone && st != GeomAbs_Sphere && st != GeomAbs_Torus) return false;
            }
            return true;
        };
        std::vector<int> cand;
        for (int i = 1; i <= edges.Extent(); ++i)
            if (simple(TopoDS::Edge(edges(i)))) cand.push_back(i);
        if (cand.empty()) throw std::runtime_error("nenhuma aresta adequada");
        TopTools_IndexedMapOfShape skip;
        for (int pass = 0; pass < 6; ++pass) {
            Mk mk(s.s);
            int added = 0;
            for (int i : cand) {
                if (skip.Contains(edges(i))) continue;
                mk.Add(v, TopoDS::Edge(edges(i)));
                ++added;
            }
            if (added == 0) break;
            mk.Build();
            if (mk.IsDone()) {
                fill(out, carry(s.s, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return mk.Modified(f); }, -1));
                return wrap(mk.Shape());
            }
            bool more = false;
            if constexpr (std::is_same_v<Mk, BRepFilletAPI_MakeFillet>) {
                // contornos que falham saem e o resto é refeito do zero
                for (int c = 1; c <= mk.NbFaultyContours(); ++c) {
                    const int ic = mk.FaultyContour(c);
                    for (int k = 1; k <= mk.NbEdges(ic); ++k) {
                        skip.Add(mk.Edge(ic, k));
                        more = true;
                    }
                }
            }
            if (!more) break;
        }
        throw std::runtime_error("valor grande demais para a geometria");
    });
}

}  // namespace

std::unique_ptr<Shape> boolean_t(const Shape& a, const Shape& b, uint8_t op, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out,
                                 rust::Vec<int32_t>& out_b) {
    return guard("booleana", [&] {
        std::unique_ptr<BRepAlgoAPI_BooleanOperation> m;
        if (op == 0) m.reset(new BRepAlgoAPI_Fuse(a.s, b.s, faixa_cancelavel()));
        else if (op == 1) m.reset(new BRepAlgoAPI_Cut(a.s, b.s, faixa_cancelavel()));
        else m.reset(new BRepAlgoAPI_Common(a.s, b.s, faixa_cancelavel()));
        falhar_se_cancelado();
        if (m->HasErrors() || m->Shape().IsNull()) throw std::runtime_error("a operação booleana falhou");
        const TopoDS_Shape r = m->Shape();
        auto modified = [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return m->Modified(f); };
        auto t1 = carry(a.s, to_vec(tags), r, modified, -1);
        // de qual face da ferramenta (índice na ordem de faces de `b`) vem cada face do resultado
        TopTools_IndexedMapOfShape bm;
        TopExp::MapShapes(b.s, TopAbs_FACE, bm);
        std::vector<int> bi(bm.Extent());
        for (int i = 0; i < bm.Extent(); ++i) bi[i] = i;
        auto tb1 = carry(b.s, bi, r, modified, -1);
        ShapeUpgrade_UnifySameDomain u(r, Standard_True, Standard_True, Standard_False);
        u.Build();
        Handle(BRepTools_History) h = u.History();
        auto hist = [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return h->Modified(f); };
        auto t2 = carry(r, t1, u.Shape(), hist, -1);
        auto tb2 = carry(r, tb1, u.Shape(), hist, -1);
        fill(out, t2);
        fill(out_b, tb2);
        return wrap(u.Shape());
    });
}

// Filete/chanfro só nas arestas cujo ponto médio é o mais próximo de cada referência (xyz).
template <class Mk>
std::unique_ptr<Shape> edge_op_sel(const char* what, const Shape& s, double v, rust::Slice<const double> mids, double tol,
                                   rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return guard(what, [&] {
        Mk mk(s.s);
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(s.s, TopAbs_EDGE, edges);
        int added = 0, missing = 0;
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
            if (best > 0 && bd <= tol) {
                mk.Add(v, TopoDS::Edge(edges(best)));
                added++;
            } else {
                missing++;
            }
        }
        // nunca descarta em silêncio: se alguma aresta pedida não existe mais, o comando fica em erro
        if (missing > 0) {
            const std::string n = std::to_string(missing);
            throw std::runtime_error(added == 0 ? "nenhuma das arestas escolhidas existe mais — reedite a seleção de arestas"
                                                : n + " das arestas escolhidas não existe(m) mais — reedite a seleção de arestas");
        }
        if (added == 0) throw std::runtime_error("escolha as arestas na viewport");
        mk.Build();
        if (!mk.IsDone()) throw std::runtime_error("valor grande demais para as arestas escolhidas");
        fill(out, carry(s.s, to_vec(tags), mk.Shape(), [&](const TopoDS_Shape& f) -> const TopTools_ListOfShape& { return mk.Modified(f); }, -1));
        return wrap(mk.Shape());
    });
}

std::unique_ptr<Shape> fillet_sel_t(const Shape& s, double r, rust::Slice<const double> mids, double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return edge_op_sel<BRepFilletAPI_MakeFillet>("filete", s, r, mids, tol, tags, out);
}

std::unique_ptr<Shape> chamfer_sel_t(const Shape& s, double d, rust::Slice<const double> mids, double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return edge_op_sel<BRepFilletAPI_MakeChamfer>("chanfro", s, d, mids, tol, tags, out);
}

// Ferramenta de rosca modelada: perfil em V varrido ao longo de uma hélice.
// Interna (furo roscado): o sulco vai do diâmetro menor (furo) até o maior, removendo material.
std::unique_ptr<Shape> thread_tool(double ox, double oy, double oz, double dx, double dy, double dz,
                                   double major_d, double pitch, double length, double angle_deg) {
    return thread_tool_x(ox, oy, oz, dx, dy, dz, major_d, pitch, length, angle_deg, 0, 0, 0);
}

// `xx,xy,xz`: direção onde a hélice começa (ângulo 0); precisa ser perpendicular ao eixo. Nula = a padrão do OpenCASCADE.
std::unique_ptr<Shape> thread_tool_x(double ox, double oy, double oz, double dx, double dy, double dz,
                                     double major_d, double pitch, double length, double angle_deg, double xx, double xy, double xz) {
    return guard("rosca", [&] {
        if (pitch <= 0 || length <= 0 || major_d <= 0) throw std::runtime_error("rosca: passo, comprimento e diâmetro devem ser positivos");
        const double depth = 0.6134 * pitch;              // profundidade da rosca (ISO)
        const double r_maj = major_d / 2;
        const double r_min = r_maj - depth;
        const double half = std::tan(angle_deg / 2 * M_PI / 180.0) * (depth + 0.1 * pitch);
        gp_Dir axis(dx, dy, dz);
        gp_Pnt o(ox, oy, oz);
        const bool com_x = std::abs(xx) + std::abs(xy) + std::abs(xz) > 1e-12;
        gp_Ax3 ax = com_x ? gp_Ax3(o, axis, gp_Dir(xx, xy, xz)) : gp_Ax3(o, axis);
        Handle(Geom_CylindricalSurface) cyl = new Geom_CylindricalSurface(ax, r_min);
        // hélice: linha no espaço (u = ângulo, v = altura) do cilindro
        const double turns = length / pitch;
        gp_Lin2d l2(gp_Pnt2d(0, 0), gp_Dir2d(2 * M_PI, pitch));
        Handle(Geom2d_Line) line = new Geom2d_Line(l2);
        const double span = turns * std::sqrt(4 * M_PI * M_PI + pitch * pitch);
        TopoDS_Edge helix = BRepBuilderAPI_MakeEdge(line, cyl, 0.0, span).Edge();
        BRepLib::BuildCurves3d(helix);
        TopoDS_Wire spine = BRepBuilderAPI_MakeWire(helix).Wire();
        // perfil em V no plano que contém o eixo, no início da hélice
        gp_Dir xd = ax.XDirection();
        auto P = [&](double r, double h) {
            return gp_Pnt(o.X() + xd.X() * r + axis.X() * h, o.Y() + xd.Y() * r + axis.Y() * h, o.Z() + xd.Z() * r + axis.Z() * h);
        };
        const double r_in = r_min - 0.1 * pitch;          // começa um pouco dentro do furo
        BRepBuilderAPI_MakeWire pw;
        pw.Add(BRepBuilderAPI_MakeEdge(P(r_in, -half), P(r_maj, 0)).Edge());
        pw.Add(BRepBuilderAPI_MakeEdge(P(r_maj, 0), P(r_in, half)).Edge());
        pw.Add(BRepBuilderAPI_MakeEdge(P(r_in, half), P(r_in, -half)).Edge());
        BRepOffsetAPI_MakePipeShell sweep(spine);
        sweep.SetMode(axis);                               // binormal fixa: o perfil fica no plano do eixo
        sweep.Add(pw.Wire());
        sweep.Build();
        if (!sweep.IsDone()) throw std::runtime_error("não foi possível gerar a hélice da rosca");
        sweep.MakeSolid();
        return wrap(sweep.Shape());
    });
}

// Extrusão até um plano: prisma longo cortado pelo semi-espaço do lado do perfil.
std::unique_ptr<Shape> extrude_until(const Shape& face, double dx, double dy, double dz, double len,
                                     double ox, double oy, double oz, double nx, double ny, double nz) {
    return guard("extrusão até plano", [&] {
        gp_Vec d(dx, dy, dz);
        d.Normalize();
        TopoDS_Shape prism = BRepPrimAPI_MakePrism(face.s, len * d).Shape();
        GProp_GProps g;
        BRepGProp::SurfaceProperties(face.s, g);
        gp_Pnt ref = g.CentreOfMass();
        TopoDS_Face pf = BRepBuilderAPI_MakeFace(gp_Pln(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz))).Face();
        TopoDS_Solid hs = BRepPrimAPI_MakeHalfSpace(pf, ref).Solid();
        BRepAlgoAPI_Common c(prism, hs);
        if (c.HasErrors() || c.Shape().IsNull()) throw std::runtime_error("não foi possível limitar a extrusão no plano");
        return wrap(unify(c.Shape()));
    });
}

std::unique_ptr<Shape> fillet_all_t(const Shape& s, double r, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return edge_op_t<BRepFilletAPI_MakeFillet>("filete", s, r, tags, out);
}

std::unique_ptr<Shape> chamfer_all_t(const Shape& s, double d, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out) {
    return edge_op_t<BRepFilletAPI_MakeChamfer>("chanfro", s, d, tags, out);
}

namespace {

constexpr uint32_t kNoFace = std::numeric_limits<uint32_t>::max();

// Assinatura geométrica de uma face (referências v2): [tipo, nx, ny, nz, área, px, py, pz].
// tipo: 0 plano, 1 cilindro, 2 cone, 3 esfera, 4 toro, 5 livre; n = normal externa (plano) ou eixo
// (cilindro/cone/toro, com o sinal normalizado); p = ponto SOBRE a face.
void face_signature(const TopoDS_Face& f, double out[8]) {
    BRepAdaptor_Surface su(f);
    double kind = 5;
    gp_Dir n(0, 0, 1);
    bool axis = false;
    switch (su.GetType()) {
        case GeomAbs_Plane:
            kind = 0;
            n = su.Plane().Axis().Direction();
            if (f.Orientation() == TopAbs_REVERSED) n.Reverse();
            break;
        case GeomAbs_Cylinder:
            kind = 1;
            n = su.Cylinder().Axis().Direction();
            axis = true;
            break;
        case GeomAbs_Cone:
            kind = 2;
            n = su.Cone().Axis().Direction();
            axis = true;
            break;
        case GeomAbs_Sphere:
            kind = 3;
            break;
        case GeomAbs_Torus:
            kind = 4;
            n = su.Torus().Axis().Direction();
            axis = true;
            break;
        default:
            break;
    }
    if (axis) {  // eixo não tem sentido: a maior componente fica positiva
        const double ax = std::abs(n.X()), ay = std::abs(n.Y()), az = std::abs(n.Z());
        const double big = ax >= ay && ax >= az ? n.X() : (ay >= az ? n.Y() : n.Z());
        if (big < 0) n.Reverse();
    }
    GProp_GProps g;
    BRepGProp::SurfaceProperties(f, g);
    const gp_Pnt c = g.CentreOfMass();
    // ponto garantidamente na face: o centroide projetado na superfície, se cair dentro do contorno;
    // senão o ponto de uma grade UV mais perto do centroide; senão um vértice
    gp_Pnt p = c;
    bool ok = false;
    try {
        Handle(Geom_Surface) gs = BRep_Tool::Surface(f);
        GeomAPI_ProjectPointOnSurf pr(c, gs);
        if (pr.NbPoints() > 0) {
            double u = 0, v = 0;
            pr.LowerDistanceParameters(u, v);
            if (BRepClass_FaceClassifier(f, gp_Pnt2d(u, v), 1e-6).State() == TopAbs_IN) {
                p = gs->Value(u, v);
                ok = true;
            }
        }
        if (!ok) {
            double u1, u2, v1, v2;
            BRepTools::UVBounds(f, u1, u2, v1, v2);
            // do mais perto ao mais longe: o primeiro dentro da face é o escolhido. Em furo e filete o
            // centroide cai no eixo e a projeção falha, então este caminho é comum, e classificar a
            // grade inteira era a maior parte do custo da assinatura
            struct Cell {
                double dist, u, v;
            };
            std::vector<Cell> grid;
            grid.reserve(81);
            for (int i = 1; i < 10; ++i)
                for (int j = 1; j < 10; ++j) {
                    const double u = u1 + (u2 - u1) * i / 10, v = v1 + (v2 - v1) * j / 10;
                    grid.push_back({gs->Value(u, v).Distance(c), u, v});
                }
            std::stable_sort(grid.begin(), grid.end(), [](const Cell& a, const Cell& b) { return a.dist < b.dist; });
            for (const Cell& g2 : grid) {
                if (BRepClass_FaceClassifier(f, gp_Pnt2d(g2.u, g2.v), 1e-6).State() != TopAbs_IN) continue;
                p = gs->Value(g2.u, g2.v);
                ok = true;
                break;
            }
        }
    } catch (...) {
    }
    if (!ok) {
        TopExp_Explorer vx(f, TopAbs_VERTEX);
        if (vx.More()) p = BRep_Tool::Pnt(TopoDS::Vertex(vx.Current()));
    }
    const double vals[8] = {kind, n.X(), n.Y(), n.Z(), std::abs(g.Mass()), p.X(), p.Y(), p.Z()};
    for (int i = 0; i < 8; ++i) out[i] = vals[i];
}

// As (até) duas faces vizinhas da aresta, pelo índice na ordem de faces; sem vizinha = kNoFace.
// Costura (a mesma face dos dois lados) conta uma vez só.
void edge_face_pair(const TopTools_IndexedMapOfShape& fmap, const TopTools_IndexedDataMapOfShapeListOfShape& e2f, const TopoDS_Shape& e,
                    uint32_t out[2]) {
    out[0] = out[1] = kNoFace;
    const int k = e2f.FindIndex(e);
    if (k == 0) return;
    int n = 0;
    for (const TopoDS_Shape& f : e2f(k)) {
        const int fi = fmap.FindIndex(f);
        if (fi <= 0) continue;
        const uint32_t id = static_cast<uint32_t>(fi - 1);
        if (n > 0 && out[0] == id) continue;
        if (n < 2) out[n++] = id;
    }
}

bool edge_is_seam(const TopTools_IndexedDataMapOfShapeListOfShape& e2f, const TopoDS_Edge& e) {
    if (BRep_Tool::Degenerated(e)) return true;
    const int k = e2f.FindIndex(e);
    if (k == 0) return false;
    for (const TopoDS_Shape& f : e2f(k))
        if (BRep_Tool::IsClosed(e, TopoDS::Face(f))) return true;
    return false;
}

}  // namespace

// Topologia sem malha: assinatura de cada face (8 por face), ponto médio de cada aresta (3 por aresta;
// zeros em costuras/degeneradas) e as duas faces vizinhas de cada aresta (2 por aresta).
void topology(const Shape& s, rust::Vec<double>& face_info, rust::Vec<double>& edge_mids, rust::Vec<uint32_t>& edge_faces) {
    guard("topologia", [&] {
        face_info.clear();
        edge_mids.clear();
        edge_faces.clear();
        if (s.s.IsNull()) return;
        TopTools_IndexedMapOfShape fmap, emap;
        TopExp::MapShapes(s.s, TopAbs_FACE, fmap);
        TopExp::MapShapes(s.s, TopAbs_EDGE, emap);
        TopTools_IndexedDataMapOfShapeListOfShape e2f;
        TopExp::MapShapesAndAncestors(s.s, TopAbs_EDGE, TopAbs_FACE, e2f);
        for (int i = 1; i <= fmap.Extent(); ++i) {
            double sig[8];
            face_signature(TopoDS::Face(fmap(i)), sig);
            for (double x : sig) face_info.push_back(x);
        }
        for (int i = 1; i <= emap.Extent(); ++i) {
            const TopoDS_Edge& e = TopoDS::Edge(emap(i));
            uint32_t pr[2];
            if (edge_is_seam(e2f, e)) {
                pr[0] = pr[1] = kNoFace;
                for (int k = 0; k < 3; ++k) edge_mids.push_back(0.0);
            } else {
                edge_face_pair(fmap, e2f, e, pr);
                BRepAdaptor_Curve c(e);
                const gp_Pnt mp = c.Value((c.FirstParameter() + c.LastParameter()) / 2);
                for (double v : {mp.X(), mp.Y(), mp.Z()}) edge_mids.push_back(v);
            }
            edge_faces.push_back(pr[0]);
            edge_faces.push_back(pr[1]);
        }
    });
}

// Índice (na ordem de faces da malha) da face mais próxima do ponto; -1 se nenhuma está a menos de `limit`.
int32_t face_index(const Shape& s, double px, double py, double pz, double limit) {
    return guard("face", [&] {
        TopTools_IndexedMapOfShape fmap;
        TopExp::MapShapes(s.s, TopAbs_FACE, fmap);
        TopoDS_Vertex v = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz)).Vertex();
        int best = -1;
        double bd = 1e300;
        for (int i = 1; i <= fmap.Extent(); ++i) {
            BRepExtrema_DistShapeShape d(fmap(i), v);
            if (d.IsDone() && d.Value() < bd) {
                bd = d.Value();
                best = i - 1;
            }
        }
        return bd <= limit ? best : -1;
    });
}

// Teto de fios do malhador: 0 = padrão (metade dos processadores lógicos, no mínimo 2).
static std::atomic<int32_t> g_mesh_threads{0};

int32_t mesh_threads() {
    const int32_t logical = std::max(1, OSD_Parallel::NbLogicalProcessors());
    const int32_t asked = g_mesh_threads.load();
    return std::min(logical, asked > 0 ? asked : std::max(2, logical / 2));
}

void set_mesh_threads(int32_t n) { g_mesh_threads.store(std::max(0, n)); }
void set_cancel_flag(size_t p) { g_cancelar = reinterpret_cast<const std::atomic<bool>*>(p); }

void set_deadline(double segundos) {
    g_tem_prazo = segundos >= 0;
    g_prazo_s = segundos;
    if (g_tem_prazo) g_prazo = Relogio::now() + std::chrono::duration_cast<Relogio::duration>(std::chrono::duration<double>(segundos));
}

// Algoritmo do malhador (B235, só medição): 0 = padrão do OpenCASCADE, 1 = Delabella.
static std::atomic<int32_t> g_mesh_algo{0};
void set_mesh_algo(int32_t a) { g_mesh_algo.store(a == 1 ? 1 : 0); }
int32_t mesh_algo() { return g_mesh_algo.load(); }

int32_t mesh_pool_threads() { return OSD_ThreadPool::DefaultPool()->NbThreads(); }

// Malhas em andamento seguram esta trava em modo compartilhado.
static std::shared_mutex g_mesh_gate;

// Entrada do malhador: aplica o teto e devolve a trava que dura até o fim da malha. O teto precisa
// mudar o tamanho do pool, porque `OSD_Parallel::For` pede um fio por item e ignora
// `SetNbDefaultThreadsToLaunch`. Só que `OSD_ThreadPool::Init` não aceita tarefa ativa, e a interface
// também malha; por isso espera as malhas em andamento terminarem.
static std::shared_lock<std::shared_mutex> enter_mesher() {
    const int32_t want = mesh_threads();
    {
        std::shared_lock<std::shared_mutex> running(g_mesh_gate);
        if (mesh_pool_threads() == want) return running;
    }
    {
        std::unique_lock<std::shared_mutex> alone(g_mesh_gate);
        OSD_ThreadPool::DefaultPool()->Init(want);
    }
    return std::shared_lock<std::shared_mutex>(g_mesh_gate);
}

// O malhador só confere o desvio linear de uma malha pronta, nunca o ângulo. Sem esta conferência,
// quem pede outro ângulo herdaria a malha de outro pedido.
static bool mesh_angle_differs(const TopTools_IndexedMapOfShape& fmap, double angle) {
    for (int i = 1; i <= fmap.Extent(); ++i) {
        TopLoc_Location loc;
        const Handle(Poly_Triangulation)& tri = BRep_Tool::Triangulation(TopoDS::Face(fmap(i)), loc);
        if (tri.IsNull() || tri->Parameters().IsNull() || !tri->Parameters()->HasAngle()) continue;
        if (std::abs(tri->Parameters()->Angle() - angle) > 1e-9) return true;
    }
    return false;
}

void discard_mesh(const Shape& s) {
    guard("tesselação", [&] {
        if (!s.s.IsNull()) BRepTools::Clean(s.s);
    });
}

// Normal da face no meio da aresta. Lê (u, v) pela curva da aresta na face e usa a superfície direto:
// `BRepAdaptor_Surface` varre todas as arestas da face para achar os limites, o que aqui, por aresta,
// daria custo quadrático nas faces com muitos contornos.
static gp_Dir face_normal_at_edge(const TopoDS_Edge& e, const TopoDS_Face& f, const gp_Pnt& mid) {
    TopLoc_Location loc;
    const Handle(Geom_Surface)& su = BRep_Tool::Surface(f, loc);
    double u = 0, v = 0;
    // no plano a normal é a mesma em todo ponto, e a curva da aresta teria de ser calculada
    if (!su->IsKind(STANDARD_TYPE(Geom_Plane))) {
        double t0 = 0, t1 = 0;
        Handle(Geom2d_Curve) pc = BRep_Tool::CurveOnSurface(e, f, t0, t1);
        if (!pc.IsNull()) {
            const gp_Pnt2d uv = pc->Value((t0 + t1) / 2);
            u = uv.X();
            v = uv.Y();
        } else {
            GeomAPI_ProjectPointOnSurf pr(mid, BRep_Tool::Surface(f));
            if (pr.NbPoints() > 0) pr.LowerDistanceParameters(u, v);
        }
    }
    gp_Pnt q;
    gp_Vec du, dv;
    su->D1(u, v, q, du, dv);
    gp_Vec n = du.Crossed(dv);
    if (n.Magnitude() < 1e-12) return gp_Dir(0, 0, 1);
    n.Transform(loc.Transformation());
    gp_Dir d(n);
    if (f.Orientation() == TopAbs_REVERSED) d.Reverse();
    return d;
}

// Nó da malha de uma face: tudo vem da superfície exata no (u, v) do nó.
struct NoFace {
    gp_Pnt2d uv;
    gp_Pnt p;
    gp_Vec n;
    float k1 = 0, k2 = 0;
};

// Lê normal e curvaturas da superfície. Polo e ápice degeneram: ali a derivada some, e a normal e a
// curvatura só existem como limite. Por isso, se o ponto não responde, a leitura anda um passo
// pequeno para dentro da face, na direção de `dentro` (um vizinho na malha).
struct LeitorFace {
    BRepAdaptor_Surface& su;
    GeomLProp_SLProps* props;  // nulo no plano
    bool normal_em(const gp_Pnt2d& uv, gp_Vec& e) const {
        gp_Pnt p;
        gp_Vec du, dv;
        su.D1(uv.X(), uv.Y(), p, du, dv);
        e = du.Crossed(dv);
        return e.Magnitude() > 1e-12;
    }
    bool curvatura_em(const gp_Pnt2d& uv, float& k1, float& k2) const {
        props->SetParameters(uv.X(), uv.Y());
        if (!props->IsCurvatureDefined()) return false;
        k1 = float(props->MaxCurvature());
        k2 = float(props->MinCurvature());
        return true;
    }
    // `ref` diz o lado da normal; `dentro` é o (u, v) para onde andar se o ponto degenerar
    void ler(NoFace& no, const gp_Vec& ref, const gp_Pnt2d& dentro) const {
        const auto passo = [&](double t) { return gp_Pnt2d(no.uv.X() + t * (dentro.X() - no.uv.X()), no.uv.Y() + t * (dentro.Y() - no.uv.Y())); };
        gp_Vec e = ref;
        bool ok = normal_em(no.uv, e);
        for (double t : {1e-4, 1e-2}) {
            if (ok) break;
            ok = normal_em(passo(t), e);
        }
        if (ok) e = e.Dot(ref) < 0 ? -e : e;
        else e = ref;
        if (e.Magnitude() > 1e-12) e.Normalize();
        no.n = e;
        if (!props) return;
        bool kok = curvatura_em(no.uv, no.k1, no.k2);
        for (double t : {1e-4, 1e-2}) {
            if (kok) break;
            kok = curvatura_em(passo(t), no.k1, no.k2);
        }
    }
};

// Valor que o mapa de curvatura lê: maior curvatura em módulo e média.
static inline float k_maior(const NoFace& n) { return std::max(std::abs(n.k1), std::abs(n.k2)); }
static inline float k_media(const NoFace& n) { return std::abs(n.k1 + n.k2) / 2; }

// Curvatura amostrada onde ela muda (B346). O malhador só olha a flecha: numa superfície quase plana
// sobra triângulo longo, e a curvatura, que varia muito mesmo sem flecha, sai interpolada entre vértices
// distantes, com a forma dos triângulos. Aqui cada aresta interna é conferida no meio, na superfície
// exata, e dividida se a interpolação errar mais que `tol`. A borda da face fica como está: é o que
// garante que a face continua encostada na vizinha, sem fresta. A divisão é por aresta, então os dois
// triângulos de uma aresta recebem o mesmo nó.
static void refinar_por_curvatura(std::vector<NoFace>& nos, std::vector<std::array<int, 3>>& tris, const LeitorFace& leitor, double tol, double k_ref, double menor_aresta) {
    constexpr size_t kMaxNos = 400000;
    const auto chave = [](int a, int b) { return (uint64_t(uint32_t(std::min(a, b))) << 32) | uint32_t(std::max(a, b)); };
    for (int nivel = 0; nivel < 5 && nos.size() < kMaxNos; ++nivel) {
        std::unordered_map<uint64_t, int> uso;
        uso.reserve(tris.size() * 2);
        for (const auto& t : tris) {
            for (int e = 0; e < 3; ++e) uso[chave(t[e], t[(e + 1) % 3])]++;
        }
        std::unordered_map<uint64_t, int> meio;
        for (const auto& [k, n] : uso) {
            if (n != 2) continue;  // borda da face ou costura
            const NoFace a = nos[int(k >> 32)];
            const NoFace b = nos[int(k & 0xffffffffu)];
            if (a.p.Distance(b.p) < menor_aresta) continue;
            NoFace m;
            m.uv = gp_Pnt2d((a.uv.X() + b.uv.X()) / 2, (a.uv.Y() + b.uv.Y()) / 2);
            m.p = leitor.su.Value(m.uv.X(), m.uv.Y());
            leitor.ler(m, a.n + b.n, a.uv);
            const auto erro = [&](float vm, float va, float vb) { return std::abs(vm - (va + vb) / 2) / (std::abs(vm) + k_ref); };
            if (std::max(erro(k_maior(m), k_maior(a), k_maior(b)), erro(k_media(m), k_media(a), k_media(b))) > tol) {
                meio.emplace(k, int(nos.size()));
                nos.push_back(m);
            }
        }
        if (meio.empty()) return;
        using Tri = std::array<int, 3>;
        std::vector<Tri> novos;
        novos.reserve(tris.size() * 2);
        const auto corte = [&](int a, int b) {
            const auto i = meio.find(chave(a, b));
            return i == meio.end() ? -1 : i->second;
        };
        const auto por = [&](Tri x) { novos.push_back(x); };
        for (const Tri& t : tris) {
            const int a = t[0], b = t[1], c = t[2];
            const int ab = corte(a, b), bc = corte(b, c), ca = corte(c, a);
            const int n = (ab >= 0) + (bc >= 0) + (ca >= 0);
            if (n == 0) {
                por(t);
            } else if (n == 3) {
                por({a, ab, ca}); por({ab, b, bc}); por({ca, bc, c}); por({ab, bc, ca});
            } else if (n == 1) {
                if (ab >= 0) { por({a, ab, c}); por({ab, b, c}); }
                else if (bc >= 0) { por({b, bc, a}); por({bc, c, a}); }
                else { por({c, ca, b}); por({ca, a, b}); }
            } else if (ca < 0) {
                por({ab, b, bc}); por({a, ab, bc}); por({a, bc, c});
            } else if (ab < 0) {
                por({bc, c, ca}); por({b, bc, ca}); por({b, ca, a});
            } else {
                por({ca, a, ab}); por({c, ca, ab}); por({c, ab, b});
            }
        }
        tris.swap(novos);
    }
}

// Lê a malha da forma. `curv_tol` > 0 refina a curvatura (ver `refinar_por_curvatura`) e `k_ref` é a
// curvatura abaixo da qual a diferença já não aparece na escala.
static MeshData malha_da_forma(const TopoDS_Shape& forma, double deflection, double angle, double curv_tol, double k_ref) {
        MeshData m;
        if (forma.IsNull()) return m;
        // mesma ordem de faces usada pelas etiquetas (IndexedMap)
        TopTools_IndexedMapOfShape fmap;
        TopExp::MapShapes(forma, TopAbs_FACE, fmap);
        // sem limpar: a forma nova divide faces intactas com a versão anterior, e o malhador só refaz
        // a face cujo desvio passa de 10% do pedido, para mais ou para menos. A folga absorve a caixa
        // envolvente que muda um pouco na edição; quem precisa de mudança menor usa `discard_mesh`
        if (mesh_angle_differs(fmap, angle)) BRepTools::Clean(forma);
        IMeshTools_Parameters mp;
        mp.Deflection = deflection;
        mp.Angle = angle;
        mp.InParallel = true;
        mp.AllowQualityDecrease = true;
        if (g_mesh_algo.load() == 1) mp.MeshAlgo = IMeshTools_MeshAlgoType_Delabella;
        {
            auto gate = enter_mesher();
            BRepMesh_IncrementalMesh(forma, mp, faixa_cancelavel());
        }
        // malha pela metade não pode ir para o cache nem para a tela
        falhar_se_cancelado();

        for (int fi = 1; fi <= fmap.Extent(); ++fi) {
            m.face_ranges.push_back(static_cast<uint32_t>(m.indices.size()));
            const TopoDS_Face& face = TopoDS::Face(fmap(fi));
            {
                double sig[8];
                face_signature(face, sig);
                for (double x : sig) m.face_info.push_back(x);
            }
            TopLoc_Location loc;
            Handle(Poly_Triangulation) tri = BRep_Tool::Triangulation(face, loc);
            if (tri.IsNull()) continue;
            const gp_Trsf tr = loc.Transformation();
            const bool rev = face.Orientation() == TopAbs_REVERSED;
            const uint32_t base = static_cast<uint32_t>(m.positions.size() / 3);
            const int nn = tri->NbNodes();
            std::vector<gp_Vec> nrm(nn, gp_Vec(0, 0, 0));
            std::vector<NoFace> nos(nn);
            for (int i = 1; i <= nn; ++i) nos[i - 1].p = tri->Node(i).Transformed(tr);
            std::vector<std::array<int, 3>> tris;
            tris.reserve(tri->NbTriangles());
            // para onde andar se o nó degenerar: o meio dos outros dois nós de um triângulo dele
            std::vector<gp_Pnt2d> dentro(tri->HasUVNodes() ? nn : 0);
            for (int t = 1; t <= tri->NbTriangles(); ++t) {
                int a, b, c;
                tri->Triangle(t).Get(a, b, c);
                if (rev) std::swap(b, c);
                gp_Vec n = gp_Vec(nos[a - 1].p, nos[b - 1].p).Crossed(gp_Vec(nos[a - 1].p, nos[c - 1].p));
                nrm[a - 1] += n;
                nrm[b - 1] += n;
                nrm[c - 1] += n;
                tris.push_back({a - 1, b - 1, c - 1});
                if (!dentro.empty()) {
                    const gp_Pnt2d ua = tri->UVNode(a), ub = tri->UVNode(b), uc = tri->UVNode(c);
                    dentro[a - 1] = gp_Pnt2d((ub.X() + uc.X()) / 2, (ub.Y() + uc.Y()) / 2);
                    dentro[b - 1] = gp_Pnt2d((ua.X() + uc.X()) / 2, (ua.Y() + uc.Y()) / 2);
                    dentro[c - 1] = gp_Pnt2d((ua.X() + ub.X()) / 2, (ua.Y() + ub.Y()) / 2);
                }
            }
            std::optional<BRepAdaptor_Surface> su;
            if (tri->HasUVNodes()) su.emplace(face);
            // o plano não tem o que medir, e é a face mais comum
            const bool curva = su && su->GetType() != GeomAbs_Plane;
            std::optional<GeomLProp_SLProps> props;
            if (curva) props.emplace(BRep_Tool::Surface(face), 2, 1e-7);
            if (su) {
                // normal exata da superfície: a média dos triângulos revela a malha no sombreamento
                const LeitorFace leitor{*su, props ? &*props : nullptr};
                for (int i = 0; i < nn; ++i) {
                    nos[i].uv = tri->UVNode(i + 1);
                    leitor.ler(nos[i], nrm[i], dentro[i]);
                }
                if (curva && curv_tol > 0) refinar_por_curvatura(nos, tris, leitor, curv_tol, k_ref, 2 * deflection);
            } else {
                for (int i = 0; i < nn; ++i) {
                    if (nrm[i].Magnitude() > 1e-12) nrm[i].Normalize();
                    nos[i].n = nrm[i];
                }
            }
            for (const auto& t : tris) {
                for (int i : t) m.indices.push_back(base + i);
            }
            for (const NoFace& no : nos) {
                m.positions.push_back(float(no.p.X()));
                m.positions.push_back(float(no.p.Y()));
                m.positions.push_back(float(no.p.Z()));
                m.normals.push_back(float(no.n.X()));
                m.normals.push_back(float(no.n.Y()));
                m.normals.push_back(float(no.n.Z()));
                // curvaturas principais da superfície: dependem só da geometria, nunca da malha ou do zoom
                m.curvature.push_back(no.k1);
                m.curvature.push_back(no.k2);
            }
        }

        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(forma, TopAbs_EDGE, edges);
        // arestas de costura (seam) de cilindros/cones: existem na topologia mas não são arestas visuais
        TopTools_IndexedDataMapOfShapeListOfShape edge_faces;
        TopExp::MapShapesAndAncestors(forma, TopAbs_EDGE, TopAbs_FACE, edge_faces);
        auto is_seam = [&](const TopoDS_Edge& e) {
            const int k = edge_faces.FindIndex(e);
            if (k == 0) return false;
            for (const TopoDS_Shape& f : edge_faces(k)) {
                if (BRep_Tool::IsClosed(e, TopoDS::Face(f))) return true;
            }
            return false;
        };
        for (int i = 1; i <= edges.Extent(); ++i) {
            const TopoDS_Edge& e = TopoDS::Edge(edges(i));
            m.edge_ranges.push_back(static_cast<uint32_t>(m.edges.size() / 6));
            // suave: as duas faces vizinhas se encontram com tangência (G1 ou mais)
            {
                uint8_t smooth = 0;
                const int k = edge_faces.FindIndex(e);
                if (k != 0 && edge_faces(k).Extent() == 2) {
                    const TopoDS_Face& f1 = TopoDS::Face(edge_faces(k).First());
                    const TopoDS_Face& f2 = TopoDS::Face(edge_faces(k).Last());
                    try {
                        if (BRep_Tool::HasContinuity(e, f1, f2) && BRep_Tool::Continuity(e, f1, f2) != GeomAbs_C0) smooth = 1;
                        else {
                            // sem informação (ou só C0 gravado, comum em filetes e superfícies): compara as
                            // normais em cinco pontos da aresta. Só é suave se for tangente em todos; um
                            // ponto no meio aceitava aresta que só se encosta ali e escondia aresta viva
                            BRepAdaptor_Curve cc(e);
                            const double t0 = cc.FirstParameter(), t1 = cc.LastParameter();
                            smooth = 1;
                            for (int j = 0; j < 5 && smooth; ++j) {
                                const gp_Pnt pm = cc.Value(t0 + (t1 - t0) * (0.1 + 0.2 * j));
                                if (face_normal_at_edge(e, f1, pm).Angle(face_normal_at_edge(e, f2, pm)) >= 2.0 * M_PI / 180.0) smooth = 0;
                            }
                        }
                    } catch (...) {}
                }
                m.edge_smooth.push_back(smooth);
            }
            const bool skip = BRep_Tool::Degenerated(e) || is_seam(e);
            {
                uint32_t pr[2] = {kNoFace, kNoFace};
                if (!skip) edge_face_pair(fmap, edge_faces, e, pr);
                m.edge_faces.push_back(pr[0]);
                m.edge_faces.push_back(pr[1]);
            }
            if (skip) {
                for (int k = 0; k < 3; ++k) m.edge_mids.push_back(0.f);
                for (int k = 0; k < kEdgeGeom; ++k) m.edge_geom.push_back(0.);
                continue;
            }
            BRepAdaptor_Curve c(e);
            push_edge_geom(c, m.edge_geom);
            gp_Pnt mp = c.Value((c.FirstParameter() + c.LastParameter()) / 2);
            for (double v : {mp.X(), mp.Y(), mp.Z()}) m.edge_mids.push_back(float(v));
            GCPnts_TangentialDeflection d(c, std::min(0.2, angle), deflection);
            for (int k = 1; k < d.NbPoints(); ++k) {
                gp_Pnt p = d.Value(k), q = d.Value(k + 1);
                for (double v : {p.X(), p.Y(), p.Z(), q.X(), q.Y(), q.Z()}) m.edges.push_back(float(v));
            }
        }
        TopTools_IndexedMapOfShape verts;
        TopExp::MapShapes(forma, TopAbs_VERTEX, verts);
        for (int i = 1; i <= verts.Extent(); ++i) {
            gp_Pnt p = BRep_Tool::Pnt(TopoDS::Vertex(verts(i)));
            for (double v : {p.X(), p.Y(), p.Z()}) m.vertices.push_back(float(v));
        }
        return m;
}

MeshData tessellate(const Shape& s, double deflection, double angle) {
    return guard("tesselação", [&] { return malha_da_forma(s.s, deflection, angle, 0, 0); });
}

// Malha só para a tela (B346): sai de uma cópia sem triângulos, com a mesma ordem de faces e arestas.
// A malha guardada na forma é a de trabalho (seleção, medidas, desenho) e não pode mudar com o zoom.
MeshData tessellate_view(const Shape& s, double deflection, double angle, double curv_tol, double k_ref) {
    return guard("tesselação", [&] {
        if (s.s.IsNull()) return MeshData();
        const TopoDS_Shape copia = BRepBuilderAPI_Copy(s.s, Standard_False, Standard_False).Shape();
        return malha_da_forma(copia, deflection, angle, curv_tol, k_ref);
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
    // superfície aberta não tem volume: o centro de massa do volume seria zero, longe da peça
    gp_Pnt c = TopExp_Explorer(s.s, TopAbs_SOLID).More() ? vp.CentreOfMass() : sp.CentreOfMass();
    for (double x : {vp.Mass(), sp.Mass(), c.X(), c.Y(), c.Z()}) v.push_back(x);
    return v;
}

// B92: inércia do volume (densidade 1) em torno do centro de massa: matriz simétrica [xx,yy,zz,xy,xz,yz],
// momentos principais [I1,I2,I3] e os três eixos principais (um vetor unitário por momento).
rust::Vec<double> inertia_props(const Shape& s) {
    rust::Vec<double> v;
    if (s.s.IsNull() || !TopExp_Explorer(s.s, TopAbs_SOLID).More()) return v;
    GProp_GProps vp;
    BRepGProp::VolumeProperties(s.s, vp);
    gp_Mat m = vp.MatrixOfInertia();
    for (double x : {m(1, 1), m(2, 2), m(3, 3), m(1, 2), m(1, 3), m(2, 3)}) v.push_back(x);
    GProp_PrincipalProps pp = vp.PrincipalProperties();
    double i1, i2, i3;
    pp.Moments(i1, i2, i3);
    for (double x : {i1, i2, i3}) v.push_back(x);
    for (const gp_Vec& a : {pp.FirstAxisOfInertia(), pp.SecondAxisOfInertia(), pp.ThirdAxisOfInertia()}) {
        v.push_back(a.X());
        v.push_back(a.Y());
        v.push_back(a.Z());
    }
    return v;
}

uint32_t count_faces(const Shape& s) {
    TopTools_IndexedMapOfShape f;
    TopExp::MapShapes(s.s, TopAbs_FACE, f);
    return static_cast<uint32_t>(f.Extent());
}

// Leitor e gravador de STEP dividem as tabelas globais de `Interface_Static`, que não têm trava.
// O acesso é um por vez: uma corrida ali derruba o processo, e o `guard` não pega isso.
static std::mutex g_step_gate;

void export_step(const Shape& s, rust::Str path) {
    guard("STEP", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        STEPControl_Writer w;
        if (w.Transfer(s.s, STEPControl_AsIs) != IFSelect_RetDone) throw std::runtime_error("falha ao converter para STEP");
        if (w.Write(to_std(path).c_str()) != IFSelect_RetDone) throw std::runtime_error("falha ao gravar o arquivo STEP");
    });
}

// A malha do STL é feita numa cópia sem triângulos: BRepMesh aproveita a malha que já está na forma
// quando ela atende ao desvio pedido, então a saída dependeria do que a tela malhou antes (e a
// exportação trocaria a malha de exibição, que é a mesma TShape). O StlAPI_Writer nasce em modo texto
// e, com nome de arquivo, abre um std::ofstream com char*: no Windows isso lê o caminho no código de
// página do sistema, e o UTF-8 que vem do Rust vira outro nome ou falha. Por isso o fluxo é aberto aqui.
void export_stl(const Shape& s, rust::Str path, double deflection, double angle) {
    guard("STL", [&] {
        TopoDS_Shape copy = BRepBuilderAPI_Copy(s.s, Standard_True, Standard_False).Shape();
        {
            auto gate = enter_mesher();
            BRepMesh_IncrementalMesh(copy, deflection, Standard_False, angle, Standard_True);
        }
        std::ofstream out;
        OSD_OpenStream(out, to_std(path).c_str(), std::ios::binary);
        if (!out.is_open()) throw std::runtime_error("não foi possível criar o arquivo STL");
        StlAPI_Writer w;
        w.ASCIIMode() = false;
        const bool ok = w.Write(copy, out);
        out.close();
        if (!ok || out.fail()) throw std::runtime_error("falha ao gravar o arquivo STL");
    });
}

// `info` recebe o esquema do cabeçalho e a unidade de comprimento declarada no arquivo. A forma sai
// sempre em milímetros: o leitor converte da unidade do arquivo para a do sistema.
std::unique_ptr<Shape> import_step(rust::Str path, rust::Vec<rust::String>& info) {
    return guard("STEP", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        STEPControl_Reader r;
        if (r.ReadFile(to_std(path).c_str()) != IFSelect_RetDone) throw std::runtime_error("não foi possível ler o STEP");
        std::string schema;
        Handle(StepData_StepModel) model = r.StepModel();
        if (!model.IsNull()) {
            Handle(HeaderSection_FileSchema) fs =
                Handle(HeaderSection_FileSchema)::DownCast(model->HeaderEntity(STANDARD_TYPE(HeaderSection_FileSchema)));
            if (!fs.IsNull() && fs->NbSchemaIdentifiers() > 0) schema = fs->SchemaIdentifiersValue(1)->ToCString();
        }
        NCollection_Sequence<TCollection_AsciiString> lengths, angles, solid_angles;
        r.FileUnits(lengths, angles, solid_angles);
        r.TransferRoots();
        TopoDS_Shape shape = r.OneShape();
        if (shape.IsNull()) throw std::runtime_error("o STEP não tem geometria");
        info.push_back(rust::String(schema));
        info.push_back(rust::String(lengths.IsEmpty() ? "" : lengths.First().ToCString()));
        return wrap(shape);
    });
}

rust::String occt_version() { return rust::String(OCC_VERSION_COMPLETE); }

}  // namespace fenkra

#include "features.inc"
#include "surfaces.inc"
#include "hybrid.inc"
#include "sheet.inc"
#include "drawing.inc"
#include "xde.inc"
#include "xde_pmi.inc"
