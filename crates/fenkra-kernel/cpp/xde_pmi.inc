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

#include <NCollection_HArray1.hxx>
#include <TCollection_HAsciiString.hxx>
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

}  // namespace

void xde_pmi_add(XdeDoc& d, const XdePmi& m) {
    guard("PMI", [&] {
        std::lock_guard<std::mutex> gate(g_step_gate);
        XdeDoc::Impl& x = *d.p;
        if (m.product >= x.labels.size()) throw std::runtime_error("produto inexistente");
        const TDF_Label product = x.labels[m.product];
        NCollection_Sequence<TDF_Label> first, second;
        shape_labels(x, product, m.faces, first);
        if (!m.faces2.empty()) shape_labels(x, product, m.faces2, second);
        if (m.kind == 0) {
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
            attr->SetObject(o);
            x.dimtol->SetDatum(first, dl);
            x.pmi_datums.push_back(dl);
        }
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

}  // namespace fenkra
