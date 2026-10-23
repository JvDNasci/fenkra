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
std::unique_ptr<Shape> fillet_sel_t(const Shape& s, double r, rust::Slice<const double> mids, double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> chamfer_sel_t(const Shape& s, double d, rust::Slice<const double> mids, double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> thread_tool(double ox, double oy, double oz, double dx, double dy, double dz,
                                   double major_d, double pitch, double length, double angle_deg);
std::unique_ptr<Shape> extrude_until(const Shape& face, double dx, double dy, double dz, double len,
                                     double ox, double oy, double oz, double nx, double ny, double nz);
std::unique_ptr<Shape> chamfer_all_t(const Shape& s, double d, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);

std::unique_ptr<Shape> transform_shape(const Shape& s, uint8_t kind, rust::Slice<const double> p);
std::unique_ptr<Shape> transform_t(const Shape& s, uint8_t kind, rust::Slice<const double> p, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> sweep_tool(const Shape& face, rust::Slice<const double> spine, uint8_t mode);
std::unique_ptr<Shape> loft_tool(const Shape& sections, bool ruled, bool closed);
std::unique_ptr<Shape> primitive(uint8_t kind, const Frame& fr, rust::Slice<const double> p);
std::unique_ptr<Shape> shell_t(const Shape& s, rust::Slice<const double> pts, double thickness, bool outward, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> draft_t(const Shape& s, rust::Slice<const double> pts, double dx, double dy, double dz, double angle_deg,
                               double ox, double oy, double oz, double nx, double ny, double nz, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> faces_offset_tool(const Shape& s, rust::Slice<const double> pts, double value);
std::unique_ptr<Shape> extrude_taper(const Shape& face, double dx, double dy, double dz, double len, double angle_deg);
std::unique_ptr<Shape> thin_profile(const Shape& face, double t);
std::unique_ptr<Shape> thick_segments(rust::Slice<const double> segs, double nx, double ny, double nz, double t);
std::unique_ptr<Shape> edges_mode_t(const Shape& s, uint8_t mode, double v1, double v2, rust::Slice<const double> mids, double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
rust::Vec<double> cylinder_at(const Shape& s, double px, double py, double pz);
std::unique_ptr<Shape> thread_tool_ext(double ox, double oy, double oz, double dx, double dy, double dz, double major_d, double pitch, double length, double angle_deg);

std::unique_ptr<Shape> face_copy(const Shape& s, double px, double py, double pz);
std::unique_ptr<Shape> offset_surface(const Shape& s, double d);
std::unique_ptr<Shape> fill_surface(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends);
std::unique_ptr<Shape> extrude_curve(rust::Slice<const double> pts, double dx, double dy, double dz, double len);
std::unique_ptr<Shape> revolve_curve(rust::Slice<const double> pts, double ox, double oy, double oz, double dx, double dy, double dz, double angle_deg);
rust::Vec<double> surface_near(const Shape& s, double px, double py, double pz);
rust::Vec<double> face_props(const Shape& s, double px, double py, double pz);
std::unique_ptr<Shape> extrude_to_next(const Shape& face, double dx, double dy, double dz, double len, const Shape& body);
std::unique_ptr<Shape> clip_until(const Shape& tool, const Shape& cutter, double px, double py, double pz, bool extend);
std::unique_ptr<Shape> plane_face(double ox, double oy, double oz, double nx, double ny, double nz, double half);
std::unique_ptr<Shape> sphere_surface(double cx, double cy, double cz, double r);
std::unique_ptr<Shape> cylinder_surface(double ox, double oy, double oz, double dx, double dy, double dz, double r, double h0, double h1);
std::unique_ptr<Shape> sweep_surface(rust::Slice<const double> profile, rust::Slice<const double> spine, uint8_t mode);
std::unique_ptr<Shape> loft_surface(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends, bool ruled, bool closed, rust::Slice<const double> guide);
std::unique_ptr<Shape> sew_shapes(const Shape& s, double tol);
std::unique_ptr<Shape> heal_shape(const Shape& s, double tol);
std::unique_ptr<Shape> split_side(const Shape& s, const Shape& tool, bool keep_pos, bool extend);
std::unique_ptr<Shape> nth_face(const Shape& s, uint32_t i);
std::unique_ptr<Shape> untrim_face(const Shape& s);
std::unique_ptr<Shape> extrapolate_face(const Shape& s, double px, double py, double pz, double len, int32_t cont);
rust::Vec<double> boundary_pts(const Shape& s, rust::Vec<uint32_t>& ends);
rust::Vec<double> section_pts(const Shape& a, const Shape& b, rust::Vec<uint32_t>& ends);
std::unique_ptr<Shape> thicken(const Shape& s, double t1, double t2);
std::unique_ptr<Shape> close_surface(const Shape& s, double tol);
std::unique_ptr<Shape> volume_cells(const Shape& s, rust::Slice<const double> exclude);
std::unique_ptr<Shape> mesh_shape(rust::Slice<const double> tris, bool solid);
bool is_solid(const Shape& s);
std::unique_ptr<Shape> affinity(const Shape& s, double ox, double oy, double oz, double xx, double xy, double xz, double nx, double ny, double nz,
                                double fx, double fy, double fz);
rust::String to_brep(const Shape& s);
std::unique_ptr<Shape> from_brep(rust::Str text);
std::unique_ptr<Shape> hole_tool_ext(double ox, double oy, double oz, double dx, double dy, double dz, double diameter, double depth,
                                     uint8_t kind, double d2, double p2, double a3, double tip, double taper);
rust::Vec<double> ray_hit(const Shape& s, double px, double py, double pz, double dx, double dy, double dz);
bool same_face(const Shape& s, double ax, double ay, double az, double bx, double by, double bz);
std::unique_ptr<Shape> full_round_t(const Shape& s, double mx, double my, double mz, double width, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> face_face_fillet_t(const Shape& s, double ax, double ay, double az, double bx, double by, double bz, double r, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);

MeshData tessellate(const Shape& s, double deflection);
rust::Vec<double> bounding_box(const Shape& s);
rust::Vec<double> mass_props(const Shape& s);
uint32_t count_faces(const Shape& s);
void export_step(const Shape& s, rust::Str path);
void export_stl(const Shape& s, rust::Str path);
std::unique_ptr<Shape> import_step(rust::Str path);
rust::String occt_version();

}  // namespace scc
