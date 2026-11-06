// SPDX-License-Identifier: LGPL-2.1-or-later
// Ponte XDE (B87): lê e grava STEP com o documento XCAF, que é o que carrega nomes, cores, camadas e
// a árvore de produtos. O `STEPControl` básico de `import_step` devolve só a forma.
//
// O XCAF separa definição de ocorrência: um produto (rótulo com forma) aparece N vezes como
// componente de uma montagem, cada um com a sua posição. A ponte mantém essa separação, senão quatro
// parafusos iguais voltariam como quatro peças.
//
// Gancho do PMI: `XdeReadOpts::gdt` conta o que o arquivo traz e `XdeWriteOpts::gdt` liga
// `SetDimTolMode`; `Impl::dimtol` é a ferramenta onde a escrita semântica entra.

#include <BRepCheck_Analyzer.hxx>
#include <BRep_Tool.hxx>
#include <DESTEP_Parameters.hxx>
#include <IGESCAFControl_Reader.hxx>
#include <IGESCAFControl_Writer.hxx>
#include <IGESControl_Controller.hxx>
#include <IGESData_GlobalSection.hxx>
#include <IGESData_IGESEntity.hxx>
#include <IGESData_IGESModel.hxx>
#include <Interface_Static.hxx>
#include <Quantity_ColorRGBA.hxx>
#include <STEPCAFControl_Controller.hxx>
#include <STEPCAFControl_Reader.hxx>
#include <STEPCAFControl_Writer.hxx>
#include <ShapeAnalysis_ShapeTolerance.hxx>
#include <ShapeBuild_ReShape.hxx>
#include <TDF_Tool.hxx>
#include <TDataStd_Name.hxx>
#include <TDocStd_Document.hxx>
#include <TopoDS_Iterator.hxx>
#include <Transfer_Binder.hxx>
#include <Transfer_TransientProcess.hxx>
#include <XCAFApp_Application.hxx>
#include <XCAFDoc_ColorTool.hxx>
#include <XCAFDoc_DimTolTool.hxx>
#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_LayerTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>
#include <XSControl_TransferReader.hxx>
#include <XSControl_WorkSession.hxx>
#include <map>
#include <vector>

namespace fenkra {

struct XdeDoc::Impl {
    occ::handle<TDocStd_Document> doc;
    occ::handle<XCAFDoc_ShapeTool> shapes;
    occ::handle<XCAFDoc_ColorTool> colors;
    occ::handle<XCAFDoc_LayerTool> layers;
    occ::handle<XCAFDoc_DimTolTool> dimtol;
    std::vector<TDF_Label> labels;
    std::map<std::string, uint32_t> index;
    std::vector<uint32_t> roots;
    std::string schema, units;
    // o que o arquivo trazia e a leitura deixou para trás; só o IGES preenche
    std::vector<std::string> skipped;

    Impl() {
        auto app = XCAFApp_Application::GetApplication();
        app->NewDocument("MDTV-XCAF", doc);
        // o leitor converte da unidade do arquivo para a do documento: o Fenkra trabalha em milímetros
        XCAFDoc_DocumentTool::SetLengthUnit(doc, 1.0, UnitsMethods_LengthUnit_Millimeter);
        shapes = XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        colors = XCAFDoc_DocumentTool::ColorTool(doc->Main());
        layers = XCAFDoc_DocumentTool::LayerTool(doc->Main());
        dimtol = XCAFDoc_DocumentTool::DimTolTool(doc->Main());
    }
    ~Impl() {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XCAFApp_Application::GetApplication()->Close(doc);
    }

    uint32_t visit(const TDF_Label& l) {
        TCollection_AsciiString entry;
        TDF_Tool::Entry(l, entry);
        auto found = index.find(entry.ToCString());
        if (found != index.end()) return found->second;
        uint32_t i = static_cast<uint32_t>(labels.size());
        labels.push_back(l);
        index.emplace(entry.ToCString(), i);
        if (XCAFDoc_ShapeTool::IsAssembly(l)) {
            NCollection_Sequence<TDF_Label> comps;
            XCAFDoc_ShapeTool::GetComponents(l, comps, false);
            for (const TDF_Label& c : comps) {
                TDF_Label ref;
                if (XCAFDoc_ShapeTool::GetReferredShape(c, ref)) visit(ref);
            }
        }
        return i;
    }
};

namespace {

// Parâmetros de uma chamada. O controlador registra os valores padrão em `Interface_Static`; quem
// lê antes disso recebe zeros, e a primeira gravação do processo sai sem nomes, sem camadas e com cada
// ocorrência como uma peça nova. Os construtores do leitor e do gravador registram tarde demais.
DESTEP_Parameters step_params() {
    STEPCAFControl_Controller::Init();
    DESTEP_Parameters p;
    p.InitFromStatic();
    return p;
}

rust::String label_name(const TDF_Label& l) {
    occ::handle<TDataStd_Name> n;
    if (!l.FindAttribute(TDataStd_Name::GetID(), n)) return rust::String();
    return rust::String(TCollection_AsciiString(n->Get()).ToCString());
}

void set_label_name(const TDF_Label& l, const rust::String& name) {
    if (name.empty()) return;
    TDataStd_Name::Set(l, TCollection_ExtendedString(std::string(name).c_str(), true));
}

// Os números do arquivo são cor de tela (sRGB); o `Quantity_Color` guarda valores lineares.
void push_rgba(const Quantity_ColorRGBA& c, rust::Vec<float>& out) {
    double r, g, b;
    c.GetRGB().Values(r, g, b, Quantity_TOC_sRGB);
    out.push_back(float(r));
    out.push_back(float(g));
    out.push_back(float(b));
    out.push_back(c.Alpha());
}

Quantity_ColorRGBA rgba_at(const rust::Vec<float>& v, size_t i) {
    return Quantity_ColorRGBA(Quantity_Color(double(v[i]), double(v[i + 1]), double(v[i + 2]), Quantity_TOC_sRGB), v[i + 3]);
}

bool label_color(const TDF_Label& l, Quantity_ColorRGBA& c) {
    return XCAFDoc_ColorTool::GetColor(l, XCAFDoc_ColorSurf, c) || XCAFDoc_ColorTool::GetColor(l, XCAFDoc_ColorGen, c);
}

gp_Trsf trsf_of(const rust::Vec<double>& m) {
    gp_Trsf t;
    if (m.size() == 12) t.SetValues(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11]);
    return t;
}

// Cor por subforma: vale para a face e para o corpo de uma peça multicorpo, que cobre as faces dele.
void sub_colors(const TDF_Label& l, XdeProduct& p) {
    NCollection_Sequence<TDF_Label> subs;
    if (!XCAFDoc_ShapeTool::GetSubShapes(l, subs)) return;
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(XCAFDoc_ShapeTool::GetShape(l), TopAbs_FACE, faces);
    for (const TDF_Label& sub : subs) {
        Quantity_ColorRGBA c;
        if (!label_color(sub, c)) continue;
        for (TopExp_Explorer e(XCAFDoc_ShapeTool::GetShape(sub), TopAbs_FACE); e.More(); e.Next()) {
            int i = faces.FindIndex(e.Current());
            if (i == 0) continue;
            p.face_ids.push_back(uint32_t(i - 1));
            push_rgba(c, p.face_rgba);
        }
    }
}

}  // namespace

std::unique_ptr<XdeDoc> xde_new() {
    return guard("XDE", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        auto d = std::make_unique<XdeDoc>();
        d->p = std::make_shared<XdeDoc::Impl>();
        return d;
    });
}

std::unique_ptr<XdeDoc> xde_read(rust::Str path, const XdeReadOpts& o) {
    return guard("STEP", [&] {
        auto d = xde_new();
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d->p;
        DESTEP_Parameters params = step_params();
        STEPCAFControl_Reader r;
        r.SetColorMode(o.colors);
        r.SetNameMode(o.names);
        r.SetLayerMode(o.layers);
        r.SetGDTMode(o.gdt);
        if (!r.Perform(to_std(path).c_str(), x.doc, params)) throw std::runtime_error("não foi possível ler o STEP");
        STEPControl_Reader& base = r.ChangeReader();
        occ::handle<StepData_StepModel> model = base.StepModel();
        if (!model.IsNull()) {
            occ::handle<HeaderSection_FileSchema> fs =
                occ::handle<HeaderSection_FileSchema>::DownCast(model->HeaderEntity(STANDARD_TYPE(HeaderSection_FileSchema)));
            if (!fs.IsNull() && fs->NbSchemaIdentifiers() > 0) x.schema = fs->SchemaIdentifiersValue(1)->ToCString();
        }
        NCollection_Sequence<TCollection_AsciiString> lengths, angles, solid_angles;
        base.FileUnits(lengths, angles, solid_angles);
        if (!lengths.IsEmpty()) x.units = lengths.First().ToCString();
        NCollection_Sequence<TDF_Label> free;
        x.shapes->GetFreeShapes(free);
        for (const TDF_Label& l : free) x.roots.push_back(x.visit(l));
        if (x.labels.empty()) throw std::runtime_error("o STEP não tem geometria");
        return d;
    });
}

namespace {

// Número da versão no cabeçalho do IGES (campo 23 da seção global) para o nome que as pessoas usam.
std::string iges_schema(int version) {
    switch (version) {
        case 8: return "IGES 5.0";
        case 9: return "IGES 5.1";
        case 10: return "IGES 5.2";
        case 11: return "IGES 5.3";
        default: return "IGES";
    }
}

}  // namespace

// Lê IGES para o mesmo documento XCAF do STEP. O IGES não tem árvore de produtos: cada raiz do
// arquivo vira um rótulo solto, e quem junta as superfícies numa peça é o conversor.
// Tudo atrás de `g_step_gate`: o tradutor de IGES não é reentrante e usa os mesmos `Interface_Static`.
std::unique_ptr<XdeDoc> xde_read_iges(rust::Str path, const XdeReadOpts& o) {
    return guard("IGES", [&] {
        auto d = xde_new();
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d->p;
        IGESControl_Controller::Init();
        IGESCAFControl_Reader r;
        r.SetColorMode(o.colors);
        r.SetNameMode(o.names);
        r.SetLayerMode(o.layers);
        if (r.ReadFile(to_std(path).c_str()) != IFSelect_RetDone) throw std::runtime_error("não foi possível ler o IGES");
        occ::handle<IGESData_IGESModel> model = r.IGESModel();
        if (model.IsNull()) throw std::runtime_error("não foi possível ler o IGES");
        const IGESData_GlobalSection& g = model->GlobalSection();
        x.schema = iges_schema(g.IGESVersion());
        if (!g.UnitName().IsNull()) x.units = g.UnitName()->ToCString();
        const int roots = r.NbRootsForTransfer();
        std::vector<occ::handle<Standard_Transient>> entities;
        for (int i = 1; i <= roots; ++i) entities.push_back(r.RootForTransfer(i));
        if (!r.Transfer(x.doc)) throw std::runtime_error("o IGES não tem geometria");
        occ::handle<Transfer_TransientProcess> tp = r.WS()->TransferReader()->TransientProcess();
        for (const occ::handle<Standard_Transient>& e : entities) {
            occ::handle<Transfer_Binder> b = tp.IsNull() ? occ::handle<Transfer_Binder>() : tp->Find(e);
            if (b.IsNull() || !b->HasResult()) x.skipped.push_back(e->DynamicType()->Name());
        }
        // cotas e notas (tipos 202 a 230) nem chegam às raízes transferíveis: contam por fora
        for (int i = 1; i <= model->NbEntities(); ++i) {
            occ::handle<IGESData_IGESEntity> e = model->Entity(i);
            if (!e.IsNull() && e->TypeNumber() >= 202 && e->TypeNumber() <= 230) x.skipped.push_back(e->DynamicType()->Name());
        }
        NCollection_Sequence<TDF_Label> free;
        x.shapes->GetFreeShapes(free);
        for (const TDF_Label& l : free) x.roots.push_back(x.visit(l));
        if (x.labels.empty()) throw std::runtime_error("o IGES não tem geometria");
        return d;
    });
}

XdeInfo xde_info(const XdeDoc& d) {
    const XdeDoc::Impl& x = *d.p;
    NCollection_Sequence<TDF_Label> dims, tols, datums;
    x.dimtol->GetDimensionLabels(dims);
    x.dimtol->GetGeomToleranceLabels(tols);
    x.dimtol->GetDatumLabels(datums);
    XdeInfo i;
    i.schema = rust::String(x.schema);
    i.units = rust::String(x.units);
    for (uint32_t r : x.roots) i.roots.push_back(r);
    for (const std::string& k : x.skipped) i.skipped.push_back(rust::String(k));
    i.dims = uint32_t(dims.Length());
    i.tols = uint32_t(tols.Length());
    i.datums = uint32_t(datums.Length());
    return i;
}

rust::Vec<XdeProduct> xde_products(const XdeDoc& d) {
    return guard("XDE", [&] {
        const XdeDoc::Impl& x = *d.p;
        rust::Vec<XdeProduct> out;
        for (const TDF_Label& l : x.labels) {
            XdeProduct p;
            p.name = label_name(l);
            p.is_assembly = XCAFDoc_ShapeTool::IsAssembly(l);
            Quantity_ColorRGBA c;
            if (label_color(l, c)) push_rgba(c, p.rgba);
            occ::handle<NCollection_HSequence<TCollection_ExtendedString>> names;
            if (x.layers->GetLayers(l, names) && !names.IsNull()) {
                for (const TCollection_ExtendedString& n : *names) p.layers.push_back(rust::String(TCollection_AsciiString(n).ToCString()));
            }
            if (p.is_assembly) {
                NCollection_Sequence<TDF_Label> comps;
                XCAFDoc_ShapeTool::GetComponents(l, comps, false);
                for (const TDF_Label& k : comps) {
                    TDF_Label ref;
                    if (!XCAFDoc_ShapeTool::GetReferredShape(k, ref)) continue;
                    TCollection_AsciiString entry;
                    TDF_Tool::Entry(ref, entry);
                    XdeInst inst;
                    inst.product = x.index.at(entry.ToCString());
                    inst.name = label_name(k);
                    const gp_Trsf t = XCAFDoc_ShapeTool::GetLocation(k).Transformation();
                    for (int i = 1; i <= 3; ++i)
                        for (int j = 1; j <= 4; ++j) inst.m.push_back(t.Value(i, j));
                    Quantity_ColorRGBA ic;
                    if (label_color(k, ic)) push_rgba(ic, inst.rgba);
                    p.comps.push_back(std::move(inst));
                }
            } else {
                sub_colors(l, p);
            }
            out.push_back(std::move(p));
        }
        return out;
    });
}

std::unique_ptr<Shape> xde_shape(const XdeDoc& d, uint32_t i) {
    return guard("XDE", [&] {
        if (i >= d.p->labels.size()) throw std::runtime_error("produto inexistente");
        return wrap(XCAFDoc_ShapeTool::GetShape(d.p->labels[i]));
    });
}

// Registra a definição; os componentes entram depois, quando todos os produtos têm rótulo.
uint32_t xde_add(XdeDoc& d, const XdeProduct& p, const Shape& s) {
    return guard("XDE", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d.p;
        TDF_Label l = p.is_assembly ? x.shapes->NewShape() : x.shapes->AddShape(s.s, false, false);
        if (l.IsNull()) throw std::runtime_error("o produto não tem forma");
        set_label_name(l, p.name);
        if (p.rgba.size() == 4) x.colors->SetColor(l, rgba_at(p.rgba, 0), XCAFDoc_ColorSurf);
        for (const rust::String& n : p.layers) x.layers->SetLayer(l, TCollection_ExtendedString(std::string(n).c_str(), true));
        if (!p.face_ids.empty()) {
            TopTools_IndexedMapOfShape faces;
            TopExp::MapShapes(s.s, TopAbs_FACE, faces);
            for (size_t k = 0; k < p.face_ids.size() && 4 * k + 3 < p.face_rgba.size(); ++k) {
                int i = int(p.face_ids[k]) + 1;
                if (i > faces.Extent()) continue;
                TDF_Label sub = x.shapes->AddSubShape(l, faces(i));
                if (!sub.IsNull()) x.colors->SetColor(sub, rgba_at(p.face_rgba, 4 * k), XCAFDoc_ColorSurf);
            }
        }
        x.labels.push_back(l);
        return uint32_t(x.labels.size() - 1);
    });
}

void xde_add_component(XdeDoc& d, uint32_t assembly, const XdeInst& c) {
    guard("XDE", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d.p;
        if (assembly >= x.labels.size() || c.product >= x.labels.size()) throw std::runtime_error("produto inexistente");
        TDF_Label k = x.shapes->AddComponent(x.labels[assembly], x.labels[c.product], TopLoc_Location(trsf_of(c.m)));
        if (k.IsNull()) throw std::runtime_error("componente recusado");
        set_label_name(k, c.name);
        if (c.rgba.size() == 4) x.colors->SetColor(k, rgba_at(c.rgba, 0), XCAFDoc_ColorSurf);
    });
}

void xde_write(XdeDoc& d, rust::Str path, const XdeWriteOpts& o) {
    guard("STEP", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d.p;
        x.shapes->UpdateAssemblies();
        DESTEP_Parameters params = step_params();
        switch (o.schema) {
            case 0: params.WriteSchema = DESTEP_Parameters::WriteMode_StepSchema_AP203; break;
            case 1: params.WriteSchema = DESTEP_Parameters::WriteMode_StepSchema_AP214IS; break;
            case 2: params.WriteSchema = DESTEP_Parameters::WriteMode_StepSchema_AP242DIS; break;
            default: throw std::runtime_error("esquema STEP desconhecido");
        }
        switch (o.unit) {
            case 0: params.WriteUnit = UnitsMethods_LengthUnit_Millimeter; break;
            case 1: params.WriteUnit = UnitsMethods_LengthUnit_Inch; break;
            case 2: params.WriteUnit = UnitsMethods_LengthUnit_Meter; break;
            default: throw std::runtime_error("unidade desconhecida");
        }
        STEPCAFControl_Writer w;
        w.SetColorMode(o.colors);
        w.SetNameMode(o.names);
        w.SetLayerMode(o.layers);
        w.SetDimTolMode(o.gdt);
        if (!w.Transfer(x.doc, params, STEPControl_AsIs)) throw std::runtime_error("falha ao converter para STEP");
        if (w.Write(to_std(path).c_str()) != IFSelect_RetDone) throw std::runtime_error("falha ao gravar o arquivo STEP");
    });
}

// `brep`: entidades 186 e 514 (sólido com topologia) em vez de faces aparadas soltas (144). O modo e a
// unidade só existem como `Interface_Static`, lidos no construtor do gravador: por isso vêm antes dele.
void xde_write_iges(XdeDoc& d, rust::Str path, const XdeIgesOpts& o) {
    guard("IGES", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d.p;
        x.shapes->UpdateAssemblies();
        IGESControl_Controller::Init();
        const char* unit = nullptr;
        switch (o.unit) {
            case 0: unit = "MM"; break;
            case 1: unit = "INCH"; break;
            case 2: unit = "M"; break;
            default: throw std::runtime_error("unidade desconhecida");
        }
        if (!Interface_Static::SetCVal("write.iges.unit", unit)) throw std::runtime_error("unidade recusada pelo gravador IGES");
        Interface_Static::SetIVal("write.iges.brep.mode", o.brep ? 1 : 0);
        IGESCAFControl_Writer w;
        w.SetColorMode(o.colors);
        w.SetNameMode(o.names);
        w.SetLayerMode(o.layers);
        if (!w.Transfer(x.doc)) throw std::runtime_error("falha ao converter para IGES");
        if (!w.Write(to_std(path).c_str())) throw std::runtime_error("falha ao gravar o arquivo IGES");
    });
}

// Coloca a forma numa posição sem copiar a geometria: a matriz vira localização.
std::unique_ptr<Shape> xde_moved(const Shape& s, rust::Slice<const double> m) {
    return guard("XDE", [&] {
        if (m.size() != 12) throw std::runtime_error("matriz inválida");
        gp_Trsf t;
        t.SetValues(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11]);
        return wrap(s.s.Moved(TopLoc_Location(t)));
    });
}

uint32_t xde_solid_count(const Shape& s) {
    uint32_t n = 0;
    for (TopExp_Explorer e(s.s, TopAbs_SOLID); e.More(); e.Next()) ++n;
    return n;
}

std::unique_ptr<Shape> xde_solid_at(const Shape& s, uint32_t i) {
    return guard("XDE", [&] {
        uint32_t n = 0;
        for (TopExp_Explorer e(s.s, TopAbs_SOLID); e.More(); e.Next(), ++n)
            if (n == i) return wrap(e.Current());
        throw std::runtime_error("sólido inexistente");
    });
}

namespace {

struct Survey {
    int faces = 0, bad_faces = 0, solids = 0, open_shells = 0, loose_faces = 0, free_edges = 0;
    double tolerance = 0;
    bool defect() const { return bad_faces > 0 || open_shells > 0 || loose_faces > 0; }
};

// Arestas de borda de uma casca: as que só uma face usa. A costura de uma face consigo mesma aparece
// duas vezes na lista de ancestrais, e aresta degenerada (polo de esfera) não é borda.
void free_edges_of(const TopoDS_Shape& shell, std::vector<TopoDS_Edge>& out) {
    TopTools_IndexedDataMapOfShapeListOfShape e2f;
    TopExp::MapShapesAndAncestors(shell, TopAbs_EDGE, TopAbs_FACE, e2f);
    for (int i = 1; i <= e2f.Extent(); ++i) {
        const TopoDS_Edge& e = TopoDS::Edge(e2f.FindKey(i));
        if (e2f(i).Extent() == 1 && !BRep_Tool::Degenerated(e)) out.push_back(e);
    }
}

// Fotografia dos defeitos: `BRepCheck` julga cada face, e a casca fechada é a que usa cada aresta
// duas vezes. Casca que não forma sólido conta como aberta, mesmo fechada: ainda não é volume.
Survey survey(const TopoDS_Shape& s) {
    Survey v;
    BRepCheck_Analyzer an(s);
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(s, TopAbs_FACE, faces);
    v.faces = faces.Extent();
    for (int i = 1; i <= faces.Extent(); ++i)
        if (!an.IsValid(faces(i))) ++v.bad_faces;
    for (TopExp_Explorer e(s, TopAbs_SOLID); e.More(); e.Next()) ++v.solids;
    for (TopExp_Explorer e(s, TopAbs_SHELL); e.More(); e.Next())
        if (!BRep_Tool::IsClosed(e.Current())) ++v.open_shells;
    for (TopExp_Explorer e(s, TopAbs_SHELL, TopAbs_SOLID); e.More(); e.Next())
        if (BRep_Tool::IsClosed(e.Current())) ++v.open_shells;
    for (TopExp_Explorer e(s, TopAbs_FACE, TopAbs_SHELL); e.More(); e.Next()) ++v.loose_faces;
    std::vector<TopoDS_Edge> borders;
    for (TopExp_Explorer e(s, TopAbs_SHELL); e.More(); e.Next()) free_edges_of(e.Current(), borders);
    v.free_edges = int(borders.size());
    v.tolerance = v.faces > 0 ? ShapeAnalysis_ShapeTolerance().Tolerance(s, 1) : 0;
    return v;
}

// Costura só o que está solto: passar os sólidos bons pela costura refaria a topologia deles à toa.
TopoDS_Shape sew_loose(const TopoDS_Shape& s, double tol, int& sewn) {
    BRepBuilderAPI_Sewing sw(tol);
    sewn = 0;
    for (TopExp_Explorer e(s, TopAbs_FACE, TopAbs_SOLID); e.More(); e.Next(), ++sewn) sw.Add(e.Current());
    if (sewn == 0) return s;
    sw.Perform();
    TopoDS_Compound out;
    BRep_Builder b;
    b.MakeCompound(out);
    int parts = 0;
    for (TopExp_Explorer e(s, TopAbs_SOLID); e.More(); e.Next(), ++parts) b.Add(out, e.Current());
    const TopoDS_Shape sewed = sw.SewedShape();
    for (TopExp_Explorer e(sewed, TopAbs_SHELL); e.More(); e.Next(), ++parts) {
        if (!BRep_Tool::IsClosed(e.Current())) {
            b.Add(out, e.Current());
            continue;
        }
        BRepBuilderAPI_MakeSolid ms(TopoDS::Shell(e.Current()));
        occ::handle<ShapeFix_Solid> fx = new ShapeFix_Solid(ms.Solid());
        fx->Perform();
        b.Add(out, fx->Solid());
    }
    for (TopExp_Explorer e(sewed, TopAbs_FACE, TopAbs_SHELL); e.More(); e.Next(), ++parts) b.Add(out, e.Current());
    // curvas soltas não entram na costura, mas continuam na peça: o IGES costuma trazê-las junto
    for (TopExp_Explorer e(s, TopAbs_EDGE, TopAbs_FACE); e.More(); e.Next(), ++parts) b.Add(out, e.Current());
    if (parts == 1) return TopoDS_Iterator(out).Value();
    return out;
}

}  // namespace

// `level`: 0 só analisa, 1 repara na tolerância dada, 2 deixa o reparo subir a tolerância em 100 vezes.
// Forma sã sai intacta: reparar sem defeito só arriscaria a fidelidade da ida e volta.
// `stats`: faces, faces ruins, sólidos, abertas, faces soltas, arestas livres e tolerância máxima (antes
// e depois), faces tocadas e costuradas.
std::unique_ptr<Shape> xde_heal(const Shape& s, uint8_t level, double tol, rust::Vec<double>& stats) {
    return guard("reparo", [&] {
        const Survey before = survey(s.s);
        TopoDS_Shape r = s.s;
        int touched = 0, sewn = 0;
        if (level > 0 && before.defect()) {
            occ::handle<ShapeFix_Shape> fx = new ShapeFix_Shape(r);
            fx->SetPrecision(tol);
            fx->SetMaxTolerance(level > 1 ? tol * 100 : tol);
            fx->Perform();
            occ::handle<ShapeBuild_ReShape> ctx = fx->Context();
            TopTools_IndexedMapOfShape faces;
            TopExp::MapShapes(r, TopAbs_FACE, faces);
            for (int i = 1; i <= faces.Extent(); ++i)
                if (ctx->IsRecorded(faces(i))) ++touched;
            r = sew_loose(fx->Shape(), level > 1 ? tol * 100 : tol, sewn);
        }
        const Survey after = level > 0 && before.defect() ? survey(r) : before;
        for (const Survey* v : {&before, &after})
            for (double n : {double(v->faces), double(v->bad_faces), double(v->solids), double(v->open_shells), double(v->loose_faces), double(v->free_edges), v->tolerance}) stats.push_back(n);
        stats.push_back(touched);
        stats.push_back(sewn);
        return wrap(r);
    });
}

namespace {

// Traço de uma aresta em segmentos, para realçar na tela: 6 números por segmento. Aresta só com
// curva no espaço da face ainda tem traço, porque `BRepAdaptor_Curve` avalia pela face.
void trace_edge(const TopoDS_Edge& e, int samples, rust::Vec<double>& out) {
    if (BRep_Tool::Degenerated(e)) return;
    BRepAdaptor_Curve c(e);
    const double a = c.FirstParameter(), b = c.LastParameter();
    gp_Pnt prev = c.Value(a);
    for (int i = 1; i <= samples; ++i) {
        const gp_Pnt p = c.Value(a + (b - a) * i / samples);
        for (double v : {prev.X(), prev.Y(), prev.Z(), p.X(), p.Y(), p.Z()}) out.push_back(v);
        prev = p;
    }
}

void trace_face(const TopoDS_Shape& f, rust::Vec<double>& out) {
    int edges = 0;
    for (TopExp_Explorer e(f, TopAbs_EDGE); e.More() && edges < 64; e.Next(), ++edges) trace_edge(TopoDS::Edge(e.Current()), 8, out);
}

}  // namespace

// Lista de defeitos que a tela realça, até `limit`. `kind`: 0 face inválida, 1 aresta livre (borda de
// casca aberta), 2 face solta. O total pode passar do limite: a contagem vem de `xde_heal`.
rust::Vec<XdeDefect> xde_defects(const Shape& s, uint32_t limit) {
    return guard("reparo", [&] {
        rust::Vec<XdeDefect> out;
        BRepCheck_Analyzer an(s.s);
        TopTools_IndexedMapOfShape faces;
        TopExp::MapShapes(s.s, TopAbs_FACE, faces);
        auto add = [&](uint8_t kind, uint32_t index, const TopoDS_Shape& f) {
            XdeDefect d;
            d.kind = kind;
            d.index = index;
            if (f.ShapeType() == TopAbs_EDGE) trace_edge(TopoDS::Edge(f), 12, d.segs);
            else trace_face(f, d.segs);
            out.push_back(std::move(d));
        };
        for (int i = 1; i <= faces.Extent() && out.size() < limit; ++i)
            if (!an.IsValid(faces(i))) add(0, uint32_t(i - 1), faces(i));
        for (TopExp_Explorer e(s.s, TopAbs_FACE, TopAbs_SHELL); e.More() && out.size() < limit; e.Next())
            add(2, uint32_t(faces.FindIndex(e.Current()) - 1), e.Current());
        std::vector<TopoDS_Edge> borders;
        for (TopExp_Explorer e(s.s, TopAbs_SHELL); e.More(); e.Next()) free_edges_of(e.Current(), borders);
        for (size_t i = 0; i < borders.size() && out.size() < limit; ++i) add(1, uint32_t(i), borders[i]);
        return out;
    });
}

}  // namespace fenkra
