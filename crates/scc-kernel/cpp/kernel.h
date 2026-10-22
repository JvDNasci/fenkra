// Fachada C++ do SolidCadCraft sobre o OpenCASCADE.
// Regra: nenhuma exceção do OCCT atravessa a ponte; tudo vira std::runtime_error.
#pragma once
#include <memory>
#include <TopoDS_Shape.hxx>
#include "rust/cxx.h"

namespace scc {

struct Frame;
struct Seg;
struct MeshData;

struct Shape {
    TopoDS_Shape s;
};

std::unique_ptr<Shape> shape_clone(const Shape& s);
bool shape_is_null(const Shape& s);
std::unique_ptr<Shape> shape_empty();

std::unique_ptr<Shape> make_face(const Frame& frame, rust::Slice<const Seg> segs, rust::Slice<const uint32_t> loop_ends);
std::unique_ptr<Shape> extrude(const Shape& face, double dx, double dy, double dz, double front, double back);
std::unique_ptr<Shape> revolve(const Shape& face, double ox, double oy, double oz, double dx, double dy, double dz,
                               double angle_deg, double start_deg);
std::unique_ptr<Shape> boolean(const Shape& a, const Shape& b, uint8_t op);
std::unique_ptr<Shape> hole_tool(double ox, double oy, double oz, double dx, double dy, double dz,
                                 double diameter, double depth, uint8_t kind, double d2, double p2);
std::unique_ptr<Shape> fillet_all(const Shape& s, double radius);
std::unique_ptr<Shape> chamfer_all(const Shape& s, double dist);
std::unique_ptr<Shape> compound(const Shape& a, const Shape& b);
std::unique_ptr<Shape> boolean_t(const Shape& a, const Shape& b, uint8_t op, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> fillet_all_t(const Shape& s, double r, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> chamfer_all_t(const Shape& s, double d, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);

MeshData tessellate(const Shape& s, double deflection);
rust::Vec<double> bounding_box(const Shape& s);
rust::Vec<double> mass_props(const Shape& s);
uint32_t count_faces(const Shape& s);
void export_step(const Shape& s, rust::Str path);
void export_stl(const Shape& s, rust::Str path);
std::unique_ptr<Shape> import_step(rust::Str path);
rust::String occt_version();

}  // namespace scc
