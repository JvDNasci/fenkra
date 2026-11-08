// SPDX-License-Identifier: LGPL-2.1-or-later
// PMI semântico no documento XCAF (B191): dimensões, tolerâncias geométricas e datums ligados a faces.
//
// O que o OCCT 8.0.1 grava de fato (conferido nos cabeçalhos instalados e por ida e volta):
//  - o gravador só escreve PMI se `SetDimTolMode(true)` e o esquema for AP242; em AP203/AP214 as
//    anotações do documento simplesmente não saem;
//  - os tipos vêm dos enums de `XCAFDimTolObjects_*` (32 de dimensão, 15 de tolerância); não há nome de
//    entidade STEP na API: quem decide a entidade é o gravador a partir do enum;
//  - cada anotação é um rótulo na tabela do `XCAFDoc_DimTolTool` com um objeto (`DimensionObject`,
//    `GeomToleranceObject`, `DatumObject`) e uma ligação (`XCAFDoc_GraphNode`) aos rótulos de forma;
//  - a ligação com a face é por subforma: `AddSubShape` cria o rótulo da face, e o arquivo guarda o
//    `shape_aspect` correspondente.

#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <NCollection_HArray1.hxx>
#include <StepData_StepModel.hxx>
#include <StepGeom_Axis2Placement.hxx>
#include <StepGeom_Axis2Placement3d.hxx>
#include <StepGeom_CartesianPoint.hxx>
#include <StepGeom_Direction.hxx>
#include <StepRepr_RepresentationContext.hxx>
#include <StepRepr_RepresentationItem.hxx>
#include <StepShape_ShapeRepresentation.hxx>
#include <StepVisual_CameraModelD3.hxx>
#include <StepVisual_DraughtingCallout.hxx>
#include <StepVisual_DraughtingModel.hxx>
#include <StepVisual_PlanarBox.hxx>
#include <StepVisual_ViewVolume.hxx>
#include <TCollection_HAsciiString.hxx>
#include <XCAFDoc_View.hxx>
#include <XCAFDoc_ViewTool.hxx>
#include <XCAFView_Object.hxx>
#include <XCAFDimTolObjects_DatumObject.hxx>
#include <XCAFDimTolObjects_DimensionObject.hxx>
#include <XCAFDimTolObjects_GeomToleranceObject.hxx>
#include <XCAFDoc_Datum.hxx>
#include <XCAFDoc_Dimension.hxx>
#include <XCAFDoc_GeomTolerance.hxx>

namespace fenkra {

namespace {

occ::handle<TCollection_HAsciiString> ascii(const rust::String& s) {
    return new TCollection_HAsciiString(std::string(s).c_str());
}

// Rótulo de forma para o grupo de faces: sem faces, o produto inteiro.
void shape_labels(XdeDoc::Impl& x, const TDF_Label& product, const rust::Vec<uint32_t>& faces, NCollection_Sequence<TDF_Label>& out) {
    if (faces.empty()) {
        out.Append(product);
        return;
    }
    TopTools_IndexedMapOfShape fmap;
    TopExp::MapShapes(XCAFDoc_ShapeTool::GetShape(product), TopAbs_FACE, fmap);
    for (uint32_t f : faces) {
        if (f >= uint32_t(fmap.Extent())) throw std::runtime_error("PMI: face inexistente no produto");
        TDF_Label sub;
        if (!x.shapes->FindSubShape(product, fmap(int(f) + 1), sub) || sub.IsNull()) sub = x.shapes->AddSubShape(product, fmap(int(f) + 1));
        if (sub.IsNull()) throw std::runtime_error("PMI: a face não aceitou anotação");
        out.Append(sub);
    }
}

// PMI gráfico: a apresentação é um composto de arestas retas; o gravador a escreve como
// `tessellated_annotation_occurrence` (polilinhas) dentro de um `draughting_callout` com o nome dado.
template <class Obj>
void set_presentation(const occ::handle<Obj>& o, const XdePmi& m) {
    if (m.prs.size() >= 6) {
        BRep_Builder b;
        TopoDS_Compound c;
        b.MakeCompound(c);
        for (size_t i = 0; i + 5 < m.prs.size(); i += 6) {
            gp_Pnt p(m.prs[i], m.prs[i + 1], m.prs[i + 2]), q(m.prs[i + 3], m.prs[i + 4], m.prs[i + 5]);
            if (p.Distance(q) < 1e-6) continue;
            b.Add(c, BRepBuilderAPI_MakeEdge(p, q).Edge());
        }
        o->SetPresentation(c, ascii(m.prs_name));
    }
    if (m.plane.size() == 9) {
        o->SetPlane(gp_Ax2(gp_Pnt(m.plane[0], m.plane[1], m.plane[2]), gp_Dir(m.plane[3], m.plane[4], m.plane[5]), gp_Dir(m.plane[6], m.plane[7], m.plane[8])));
    }
    if (m.text_at.size() == 3) o->SetPointTextAttach(gp_Pnt(m.text_at[0], m.text_at[1], m.text_at[2]));
}

}  // namespace

void xde_pmi_add(XdeDoc& d, const XdePmi& m) {
    guard("PMI", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d.p;
        if (m.product >= x.labels.size()) throw std::runtime_error("produto inexistente");
        const TDF_Label product = x.labels[m.product];
        NCollection_Sequence<TDF_Label> first, second;
        if (m.kind != 3) shape_labels(x, product, m.faces, first);
        if (!m.faces2.empty()) shape_labels(x, product, m.faces2, second);
        if (m.kind == 3) {
            // só gráfico: rótulo comum sem faces. Ligado a uma face, o gravador o trata como dimensão e
            // cai; solto, escreve o `shape_aspect` do produto e a apresentação, e o leitor o devolve igual.
            if (m.prs.size() < 6) throw std::runtime_error("PMI gráfico sem traços");
            TDF_Label dl = x.dimtol->AddDimension();
            occ::handle<XCAFDoc_Dimension> attr;
            if (!dl.FindAttribute(XCAFDoc_Dimension::GetID(), attr)) attr = XCAFDoc_Dimension::Set(dl);
            occ::handle<XCAFDimTolObjects_DimensionObject> o = new XCAFDimTolObjects_DimensionObject();
            o->SetType(XCAFDimTolObjects_DimensionType_CommonLabel);
            set_presentation(o, m);
            attr->SetObject(o);
        } else if (m.kind == 0) {
            TDF_Label dl = x.dimtol->AddDimension();
            occ::handle<XCAFDoc_Dimension> attr;
            if (!dl.FindAttribute(XCAFDoc_Dimension::GetID(), attr)) attr = XCAFDoc_Dimension::Set(dl);
            occ::handle<XCAFDimTolObjects_DimensionObject> o = new XCAFDimTolObjects_DimensionObject();
            o->SetType(XCAFDimTolObjects_DimensionType(m.ty));
            o->SetValue(m.value);
            if (m.has_tol) {
                // o gravador escreve o módulo do inferior, qualquer que seja o sinal passado
                o->SetUpperTolValue(m.upper);
                o->SetLowerTolValue(m.lower);
            }
            if (!m.name.empty()) o->SetSemanticName(ascii(m.name));
            for (uint32_t k : m.mods) o->AddModifier(XCAFDimTolObjects_DimensionModif(k));
            set_presentation(o, m);
            attr->SetObject(o);
            if (second.IsEmpty()) x.dimtol->SetDimension(first, NCollection_Sequence<TDF_Label>(), dl);
            else x.dimtol->SetDimension(first, second, dl);
        } else if (m.kind == 1) {
            TDF_Label tl = x.dimtol->AddGeomTolerance();
            occ::handle<XCAFDoc_GeomTolerance> attr;
            if (!tl.FindAttribute(XCAFDoc_GeomTolerance::GetID(), attr)) attr = XCAFDoc_GeomTolerance::Set(tl);
            occ::handle<XCAFDimTolObjects_GeomToleranceObject> o = new XCAFDimTolObjects_GeomToleranceObject();
            o->SetType(XCAFDimTolObjects_GeomToleranceType(m.ty));
            o->SetValue(m.value);
            o->SetTypeOfValue(XCAFDimTolObjects_GeomToleranceTypeValue(m.zone));
            o->SetMaterialRequirementModifier(XCAFDimTolObjects_GeomToleranceMatReqModif(m.mat));
            if (m.zone_mod != 0) {
                o->SetZoneModifier(XCAFDimTolObjects_GeomToleranceZoneModif(m.zone_mod));
                if (m.zone_val > 0) o->SetValueOfZoneModifier(m.zone_val);
            }
            if (!m.name.empty()) o->SetSemanticName(ascii(m.name));
            for (uint32_t k : m.mods) o->AddModifier(XCAFDimTolObjects_GeomToleranceModif(k));
            set_presentation(o, m);
            attr->SetObject(o);
            x.dimtol->SetGeomTolerance(first, tl);
            for (uint32_t id : m.datums) {
                if (id >= x.pmi_datums.size()) throw std::runtime_error("PMI: datum inexistente");
                x.dimtol->SetDatumToGeomTol(x.pmi_datums[id], tl);
            }
        } else {
            if (m.id != x.pmi_datums.size()) throw std::runtime_error("PMI: os datums entram em ordem");
            TDF_Label dl = x.dimtol->AddDatum(ascii(m.name), new TCollection_HAsciiString(""), new TCollection_HAsciiString(""));
            occ::handle<XCAFDoc_Datum> attr;
            if (!dl.FindAttribute(XCAFDoc_Datum::GetID(), attr)) attr = XCAFDoc_Datum::Set(dl);
            occ::handle<XCAFDimTolObjects_DatumObject> o = new XCAFDimTolObjects_DatumObject();
            o->SetName(ascii(m.name));
            o->SetPosition(int(m.position));
            if (m.target) {
                o->IsDatumTarget(true);
                o->SetDatumTargetType(XCAFDimTolObjects_DatumTargetType(m.target_type));
                o->SetDatumTargetNumber(int(m.target_number));
                if (m.target_length > 0) o->SetDatumTargetLength(m.target_length);
                if (m.target_width > 0) o->SetDatumTargetWidth(m.target_width);
            }
            set_presentation(o, m);
            attr->SetObject(o);
            x.dimtol->SetDatum(first, dl);
            x.pmi_datums.push_back(dl);
        }
        x.pmi_prs.push_back(std::string(m.prs_name));
    });
}

namespace {

// Produto e faces de uma lista de rótulos de forma (produto inteiro ou subformas).
void read_faces(const XdeDoc::Impl& x, const NCollection_Sequence<TDF_Label>& ls, uint32_t& product, rust::Vec<uint32_t>& faces) {
    for (const TDF_Label& l : ls) {
        TCollection_AsciiString own;
        TDF_Tool::Entry(l, own);
        const bool whole = x.index.count(own.ToCString()) > 0;
        const TDF_Label owner = whole ? l : l.Father();
        TCollection_AsciiString entry;
        TDF_Tool::Entry(owner, entry);
        auto found = x.index.find(entry.ToCString());
        if (found == x.index.end()) continue;
        product = found->second;
        if (whole) continue;
        TopTools_IndexedMapOfShape fmap;
        TopExp::MapShapes(XCAFDoc_ShapeTool::GetShape(owner), TopAbs_FACE, fmap);
        const TopoDS_Shape sub = XCAFDoc_ShapeTool::GetShape(l);
        for (TopExp_Explorer e(sub, TopAbs_FACE); e.More(); e.Next()) {
            for (int i = 1; i <= fmap.Extent(); ++i) {
                if (fmap(i).IsSame(e.Current()) || fmap(i).IsPartner(e.Current())) {
                    faces.push_back(uint32_t(i - 1));
                    break;
                }
            }
        }
    }
}

// A dimensão chama de `HasTextPoint` o que a tolerância e o datum chamam de `HasPointText`.
bool has_text_point(const XCAFDimTolObjects_DimensionObject& o) { return o.HasTextPoint(); }
template <class Obj>
bool has_text_point(const Obj& o) { return o.HasPointText(); }

// Apresentação lida: cada aresta vira segmentos (as polilinhas do arquivo voltam como arestas retas;
// curva, se houver, em 8 pedaços).
template <class Obj>
void get_presentation(const occ::handle<Obj>& o, XdePmi& p) {
    const TopoDS_Shape prs = o->GetPresentation();
    if (!prs.IsNull()) {
        for (TopExp_Explorer e(prs, TopAbs_EDGE); e.More(); e.Next()) {
            BRepAdaptor_Curve c(TopoDS::Edge(e.Current()));
            const int n = c.GetType() == GeomAbs_Line ? 1 : 8;
            for (int k = 0; k < n; ++k) {
                const double t0 = c.FirstParameter() + (c.LastParameter() - c.FirstParameter()) * k / n;
                const double t1 = c.FirstParameter() + (c.LastParameter() - c.FirstParameter()) * (k + 1) / n;
                const gp_Pnt a = c.Value(t0), b = c.Value(t1);
                for (double v : {a.X(), a.Y(), a.Z(), b.X(), b.Y(), b.Z()}) p.prs.push_back(v);
            }
        }
    }
    if (!o->GetPresentationName().IsNull()) p.prs_name = rust::String(o->GetPresentationName()->ToCString());
    if (o->HasPlane()) {
        const gp_Ax2 a = o->GetPlane();
        for (double v : {a.Location().X(), a.Location().Y(), a.Location().Z(), a.Direction().X(), a.Direction().Y(), a.Direction().Z(), a.XDirection().X(), a.XDirection().Y(), a.XDirection().Z()}) p.plane.push_back(v);
    }
    if (has_text_point(*o)) {
        const gp_Pnt t = o->GetPointTextAttach();
        for (double v : {t.X(), t.Y(), t.Z()}) p.text_at.push_back(v);
    }
}

// Os rótulos de anotação na ordem em que `xde_pmi_read` os devolve: datums, dimensões, tolerâncias,
// pulando os sem objeto. As vistas citam as anotações por esta ordem.
std::vector<TDF_Label> ordered_labels(const XdeDoc::Impl& x) {
    std::vector<TDF_Label> out;
    NCollection_Sequence<TDF_Label> dims, tols, datums;
    x.dimtol->GetDatumLabels(datums);
    x.dimtol->GetDimensionLabels(dims);
    x.dimtol->GetGeomToleranceLabels(tols);
    for (const TDF_Label& l : datums) out.push_back(l);
    for (const TDF_Label& l : dims) {
        occ::handle<XCAFDoc_Dimension> a;
        if (l.FindAttribute(XCAFDoc_Dimension::GetID(), a) && !a->GetObject().IsNull()) out.push_back(l);
    }
    for (const TDF_Label& l : tols) {
        occ::handle<XCAFDoc_GeomTolerance> a;
        if (l.FindAttribute(XCAFDoc_GeomTolerance::GetID(), a) && !a->GetObject().IsNull()) out.push_back(l);
    }
    return out;
}

XdePmi blank(uint8_t kind) {
    XdePmi p;
    p.kind = kind;
    p.product = 0;
    p.ty = 0;
    p.value = p.lower = p.upper = 0.;
    p.has_tol = false;
    p.zone = p.mat = p.zone_mod = 0;
    p.zone_val = 0.;
    p.id = p.position = 0;
    p.target = false;
    p.target_type = p.target_number = 0;
    p.target_length = p.target_width = 0.;
    return p;
}

}  // namespace

rust::Vec<XdePmi> xde_pmi_read(const XdeDoc& d) {
    return guard("PMI", [&] {
        const XdeDoc::Impl& x = *d.p;
        rust::Vec<XdePmi> out;
        NCollection_Sequence<TDF_Label> dims, tols, datums;
        x.dimtol->GetDatumLabels(datums);
        x.dimtol->GetDimensionLabels(dims);
        x.dimtol->GetGeomToleranceLabels(tols);
        std::vector<TDF_Label> dl;
        for (const TDF_Label& l : datums) {
            XdePmi p = blank(2);
            p.id = uint32_t(dl.size());
            dl.push_back(l);
            occ::handle<XCAFDoc_Datum> a;
            if (l.FindAttribute(XCAFDoc_Datum::GetID(), a)) {
                if (!a->GetName().IsNull()) p.name = rust::String(a->GetName()->ToCString());
                occ::handle<XCAFDimTolObjects_DatumObject> o = a->GetObject();
                if (!o.IsNull()) {
                    if (!o->GetName().IsNull() && p.name.empty()) p.name = rust::String(o->GetName()->ToCString());
                    p.position = uint32_t(o->GetPosition());
                    if (o->IsDatumTarget()) {
                        p.target = true;
                        p.target_type = uint32_t(o->GetDatumTargetType());
                        p.target_number = uint32_t(o->GetDatumTargetNumber());
                        p.target_length = o->GetDatumTargetLength();
                        p.target_width = o->GetDatumTargetWidth();
                    }
                    get_presentation(o, p);
                }
            }
            NCollection_Sequence<TDF_Label> a1, a2;
            if (XCAFDoc_DimTolTool::GetRefShapeLabel(l, a1, a2)) read_faces(x, a1, p.product, p.faces);
            out.push_back(std::move(p));
        }
        for (const TDF_Label& l : dims) {
            XdePmi p = blank(0);
            occ::handle<XCAFDoc_Dimension> a;
            if (!l.FindAttribute(XCAFDoc_Dimension::GetID(), a)) continue;
            occ::handle<XCAFDimTolObjects_DimensionObject> o = a->GetObject();
            if (o.IsNull()) continue;
            p.ty = uint32_t(o->GetType());
            if (o->GetType() == XCAFDimTolObjects_DimensionType_CommonLabel) p.kind = 3;
            get_presentation(o, p);
            p.value = o->GetValue();
            if (o->IsDimWithPlusMinusTolerance()) {
                p.has_tol = true;
                p.upper = o->GetUpperTolValue();
                p.lower = o->GetLowerTolValue();
            }
            if (!o->GetSemanticName().IsNull()) p.name = rust::String(o->GetSemanticName()->ToCString());
            for (const auto& k : o->GetModifiers()) p.mods.push_back(uint32_t(k));
            NCollection_Sequence<TDF_Label> a1, a2;
            if (XCAFDoc_DimTolTool::GetRefShapeLabel(l, a1, a2)) {
                read_faces(x, a1, p.product, p.faces);
                read_faces(x, a2, p.product, p.faces2);
            }
            out.push_back(std::move(p));
        }
        for (const TDF_Label& l : tols) {
            XdePmi p = blank(1);
            occ::handle<XCAFDoc_GeomTolerance> a;
            if (!l.FindAttribute(XCAFDoc_GeomTolerance::GetID(), a)) continue;
            occ::handle<XCAFDimTolObjects_GeomToleranceObject> o = a->GetObject();
            if (o.IsNull()) continue;
            p.ty = uint32_t(o->GetType());
            p.value = o->GetValue();
            p.zone = uint32_t(o->GetTypeOfValue());
            p.mat = uint32_t(o->GetMaterialRequirementModifier());
            p.zone_mod = uint32_t(o->GetZoneModifier());
            p.zone_val = o->GetValueOfZoneModifier();
            get_presentation(o, p);
            if (!o->GetSemanticName().IsNull()) p.name = rust::String(o->GetSemanticName()->ToCString());
            for (const auto& k : o->GetModifiers()) p.mods.push_back(uint32_t(k));
            NCollection_Sequence<TDF_Label> a1, a2;
            if (XCAFDoc_DimTolTool::GetRefShapeLabel(l, a1, a2)) read_faces(x, a1, p.product, p.faces);
            NCollection_Sequence<TDF_Label> refs;
            if (XCAFDoc_DimTolTool::GetDatumOfTolerLabels(l, refs)) {
                for (const TDF_Label& r : refs)
                    for (size_t i = 0; i < dl.size(); ++i)
                        if (dl[i] == r) p.datums.push_back(uint32_t(i));
            }
            out.push_back(std::move(p));
        }
        return out;
    });
}

void xde_view_add(XdeDoc& d, const XdeView& v) {
    guard("vista", [&] {
        XdeDoc::Impl& x = *d.p;
        XdeDoc::Impl::View w;
        w.name = std::string(v.name);
        w.eye = gp_Pnt(v.eye[0], v.eye[1], v.eye[2]);
        const gp_Vec dir(v.dir[0], v.dir[1], v.dir[2]);
        gp_Vec up(v.up[0], v.up[1], v.up[2]);
        if (dir.Magnitude() < 1e-9) throw std::runtime_error("vista sem direção");
        // o arquivo guarda um sistema de eixos: o "para cima" tem de ser perpendicular à visada
        up = up - dir.Normalized() * up.Dot(dir.Normalized());
        if (up.Magnitude() < 1e-9) throw std::runtime_error("vista com o para cima na direção de visada");
        w.dir = gp_Dir(dir);
        w.up = gp_Dir(up);
        w.ortho = v.ortho;
        w.zoom = v.zoom;
        w.w = v.width;
        w.h = v.height;
        for (uint32_t i : v.shown) {
            if (i >= x.pmi_prs.size()) throw std::runtime_error("vista cita anotação inexistente");
            w.shown.push_back(i);
        }
        x.views.push_back(std::move(w));
    });
}

void xde_prs_scale(XdeDoc::Impl& x, double factor) {
    gp_Trsf t;
    t.SetScale(gp_Pnt(0, 0, 0), factor);
    auto scale = [&](auto o) {
        if (o.IsNull()) return;
        if (!o->GetPresentation().IsNull()) o->SetPresentation(BRepBuilderAPI_Transform(o->GetPresentation(), t, true).Shape(), o->GetPresentationName());
        if (has_text_point(*o)) o->SetPointTextAttach(o->GetPointTextAttach().Scaled(gp_Pnt(0, 0, 0), factor));
    };
    for (const TDF_Label& l : ordered_labels(x)) {
        occ::handle<XCAFDoc_Dimension> d;
        occ::handle<XCAFDoc_GeomTolerance> g;
        occ::handle<XCAFDoc_Datum> a;
        if (l.FindAttribute(XCAFDoc_Dimension::GetID(), d)) {
            auto o = d->GetObject();
            scale(o);
            // valor e desvios de comprimento (ângulo não tem unidade de comprimento)
            const auto ty = o.IsNull() ? XCAFDimTolObjects_DimensionType_CommonLabel : o->GetType();
            if (!o.IsNull() && ty != XCAFDimTolObjects_DimensionType_Location_Angular && ty != XCAFDimTolObjects_DimensionType_Size_Angular) {
                occ::handle<NCollection_HArray1<double>> v = o->GetValues();
                if (!v.IsNull()) {
                    occ::handle<NCollection_HArray1<double>> w = new NCollection_HArray1<double>(v->Lower(), v->Upper());
                    for (int i = v->Lower(); i <= v->Upper(); ++i) w->SetValue(i, v->Value(i) * factor);
                    o->SetValues(w);
                }
            }
            d->SetObject(o);
        } else if (l.FindAttribute(XCAFDoc_GeomTolerance::GetID(), g)) {
            auto o = g->GetObject();
            scale(o);
            if (!o.IsNull()) {
                o->SetValue(o->GetValue() * factor);
                if (o->GetValueOfZoneModifier() > 0) o->SetValueOfZoneModifier(o->GetValueOfZoneModifier() * factor);
            }
            g->SetObject(o);
        } else if (l.FindAttribute(XCAFDoc_Datum::GetID(), a)) {
            auto o = a->GetObject();
            scale(o);
            if (!o.IsNull() && o->IsDatumTarget()) {
                if (o->GetDatumTargetLength() > 0) o->SetDatumTargetLength(o->GetDatumTargetLength() * factor);
                if (o->GetDatumTargetWidth() > 0) o->SetDatumTargetWidth(o->GetDatumTargetWidth() * factor);
            }
            a->SetObject(o);
        }
    }
}

namespace {

occ::handle<StepGeom_CartesianPoint> step_point(const gp_Pnt& p, double k) {
    occ::handle<NCollection_HArray1<double>> c = new NCollection_HArray1<double>(1, 3);
    c->SetValue(1, p.X() * k);
    c->SetValue(2, p.Y() * k);
    c->SetValue(3, p.Z() * k);
    occ::handle<StepGeom_CartesianPoint> q = new StepGeom_CartesianPoint();
    q->Init(new TCollection_HAsciiString(""), c);
    return q;
}

occ::handle<StepGeom_Direction> step_dir(const gp_Dir& d) {
    occ::handle<NCollection_HArray1<double>> c = new NCollection_HArray1<double>(1, 3);
    c->SetValue(1, d.X());
    c->SetValue(2, d.Y());
    c->SetValue(3, d.Z());
    occ::handle<StepGeom_Direction> q = new StepGeom_Direction();
    q->Init(new TCollection_HAsciiString(""), c);
    return q;
}

}  // namespace

// Vistas salvas: o gravador do OCCT 8.0.1 não escreve `camera_model_d3`; o leitor lê. Cada vista vira um
// `draughting_model` com a câmera e os `draughting_callout` das anotações que aparecem nela, achados pelo
// nome da apresentação (único por anotação, garantido do lado do Rust). O contexto é o da apresentação
// do PMI (ou o da forma), em que as unidades já são as do arquivo.
void xde_views_write(XdeDoc::Impl& x, STEPCAFControl_Writer& w, double unit_mm) {
    occ::handle<StepData_StepModel> model = w.ChangeWriter().Model();
    std::map<std::string, occ::handle<StepVisual_DraughtingCallout>> callouts;
    std::vector<occ::handle<StepVisual_DraughtingModel>> dms;
    occ::handle<StepRepr_RepresentationContext> ctx, ctx_forma;
    for (int i = 1; i <= model->NbEntities(); ++i) {
        const occ::handle<Standard_Transient>& e = model->Value(i);
        if (auto c = occ::down_cast<StepVisual_DraughtingCallout>(e)) {
            if (!c->Name().IsNull()) callouts.emplace(c->Name()->ToCString(), c);
        } else if (auto dm = occ::down_cast<StepVisual_DraughtingModel>(e)) {
            dms.push_back(dm);
            if (ctx.IsNull()) ctx = dm->ContextOfItems();
        } else if (auto sr = occ::down_cast<StepShape_ShapeRepresentation>(e)) {
            if (ctx_forma.IsNull()) ctx_forma = sr->ContextOfItems();
        }
    }
    // O gravador tira o contexto da apresentação do PMI do primeiro elemento semântico ligado à forma. Só
    // com rótulos comuns (modo gráfico), o modelo de apresentação sai sem contexto e a gravação falha:
    // o contexto da forma é o mesmo, nas mesmas unidades.
    for (auto& dm : dms)
        if (dm->ContextOfItems().IsNull()) dm->SetContextOfItems(ctx_forma);
    if (x.views.empty()) return;
    if (ctx.IsNull()) ctx = ctx_forma;
    if (ctx.IsNull()) throw std::runtime_error("vista: o arquivo não tem contexto de representação");
    const double k = 1. / unit_mm;
    for (const XdeDoc::Impl::View& v : x.views) {
        // o leitor faz "para cima" = direção ^ x: logo x = cima ^ direção
        const gp_Dir xdir = v.up.Crossed(v.dir);
        occ::handle<StepGeom_Axis2Placement3d> ax = new StepGeom_Axis2Placement3d();
        ax->Init(new TCollection_HAsciiString(""), step_point(v.eye, k), true, step_dir(v.dir), true, step_dir(xdir));
        StepGeom_Axis2Placement sel;
        sel.SetValue(ax);
        occ::handle<StepVisual_PlanarBox> win = new StepVisual_PlanarBox();
        win->Init(new TCollection_HAsciiString(""), v.w * k, v.h * k, sel);
        occ::handle<StepVisual_ViewVolume> vol = new StepVisual_ViewVolume();
        vol->Init(v.ortho ? StepVisual_copParallel : StepVisual_copCentral, step_point(v.eye, k), v.zoom * k, 0., false, 0., false, false, win);
        occ::handle<StepVisual_CameraModelD3> cam = new StepVisual_CameraModelD3();
        cam->Init(new TCollection_HAsciiString(v.name.c_str()), ax, vol);
        std::vector<occ::handle<StepRepr_RepresentationItem>> items{cam};
        for (uint32_t i : v.shown) {
            auto f = callouts.find(x.pmi_prs[i]);
            if (f != callouts.end()) items.push_back(f->second);
        }
        occ::handle<NCollection_HArray1<occ::handle<StepRepr_RepresentationItem>>> arr =
            new NCollection_HArray1<occ::handle<StepRepr_RepresentationItem>>(1, int(items.size()));
        for (size_t i = 0; i < items.size(); ++i) arr->SetValue(int(i) + 1, items[i]);
        occ::handle<StepVisual_DraughtingModel> dm = new StepVisual_DraughtingModel();
        dm->Init(new TCollection_HAsciiString(v.name.c_str()), arr, ctx);
        model->AddWithRefs(dm);
    }
}

rust::Vec<XdeView> xde_views_read(const XdeDoc& d) {
    return guard("vista", [&] {
        const XdeDoc::Impl& x = *d.p;
        rust::Vec<XdeView> out;
        occ::handle<XCAFDoc_ViewTool> vt = XCAFDoc_DocumentTool::ViewTool(x.doc->Main());
        if (vt.IsNull()) return out;
        const std::vector<TDF_Label> ordem = ordered_labels(x);
        NCollection_Sequence<TDF_Label> views;
        vt->GetViewLabels(views);
        for (const TDF_Label& l : views) {
            occ::handle<XCAFDoc_View> a;
            if (!l.FindAttribute(XCAFDoc_View::GetID(), a)) continue;
            occ::handle<XCAFView_Object> o = a->GetObject();
            if (o.IsNull()) continue;
            XdeView v;
            v.name = o->Name().IsNull() ? rust::String() : rust::String(o->Name()->ToCString());
            const gp_Pnt e = o->ProjectionPoint();
            const gp_Dir dir = o->ViewDirection(), up = o->UpDirection();
            v.eye = {e.X(), e.Y(), e.Z()};
            v.dir = {dir.X(), dir.Y(), dir.Z()};
            v.up = {up.X(), up.Y(), up.Z()};
            v.ortho = o->Type() != XCAFView_ProjectionType_Central;
            // o leitor converte o ponto de projeção para mm, mas devolve distância e janela cruas
            v.zoom = o->ZoomFactor() * x.unit_mm;
            v.width = o->WindowHorizontalSize() * x.unit_mm;
            v.height = o->WindowVerticalSize() * x.unit_mm;
            NCollection_Sequence<TDF_Label> gdts;
            if (vt->GetRefGDTLabel(l, gdts)) {
                for (const TDF_Label& g : gdts)
                    for (size_t i = 0; i < ordem.size(); ++i)
                        if (ordem[i] == g) v.shown.push_back(uint32_t(i));
            }
            out.push_back(std::move(v));
        }
        return out;
    });
}

}  // namespace fenkra
