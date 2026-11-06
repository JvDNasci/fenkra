// SPDX-License-Identifier: LGPL-2.1-or-later
// Fachada C++ do Fenkra sobre o OpenCASCADE.
// Regra: nenhuma exceção do OCCT atravessa a ponte; tudo vira std::runtime_error.
#pragma once
#include <memory>
#include <TopoDS_Shape.hxx>
#include "rust/cxx.h"

namespace fenkra {

struct Frame;
struct Seg;
struct MeshData;
struct XdeInst;
struct XdeProduct;
struct XdeInfo;
struct XdeReadOpts;
struct XdeWriteOpts;
struct XdeIgesOpts;
struct XdeDefect;
struct XdePmi;

struct Shape {
    TopoDS_Shape s;
};

// Documento XCAF (cpp/xde.inc). O conteúdo fica escondido: os cabeçalhos do XCAF só entram lá.
struct XdeDoc {
    struct Impl;
    std::shared_ptr<Impl> p;
};

std::unique_ptr<Shape> shape_clone(const Shape& s);
bool shape_is_null(const Shape& s);
std::unique_ptr<Shape> shape_empty();

std::unique_ptr<Shape> make_face(const Frame& frame, rust::Slice<const Seg> segs, rust::Slice<const uint32_t> loop_ends);
std::unique_ptr<Shape> extrude(const Shape& face, double dx, double dy, double dz, double front, double back, rust::Vec<int32_t>& roles);
std::unique_ptr<Shape> revolve(const Shape& face, double ox, double oy, double oz, double dx, double dy, double dz,
                               double angle_deg, double start_deg, rust::Vec<int32_t>& roles);
std::unique_ptr<Shape> boolean(const Shape& a, const Shape& b, uint8_t op);
std::unique_ptr<Shape> hole_tool(double ox, double oy, double oz, double dx, double dy, double dz,
                                 double diameter, double depth, uint8_t kind, double d2, double p2);
std::unique_ptr<Shape> fillet_all(const Shape& s, double radius);
std::unique_ptr<Shape> chamfer_all(const Shape& s, double dist);
std::unique_ptr<Shape> compound(const Shape& a, const Shape& b);
std::unique_ptr<Shape> boolean_t(const Shape& a, const Shape& b, uint8_t op, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out,
                                 rust::Vec<int32_t>& out_b);
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
std::unique_ptr<Shape> loft_tool(const Shape& sections, bool ruled, bool closed, bool alinhar);
std::unique_ptr<Shape> primitive(uint8_t kind, const Frame& fr, rust::Slice<const double> p);
std::unique_ptr<Shape> shell_faces_t(const Shape& s, rust::Slice<const double> pts, double thickness, bool outward, rust::Slice<const double> fpts, rust::Slice<const double> vals, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> shell_t(const Shape& s, rust::Slice<const double> pts, double thickness, bool outward, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> draft_t(const Shape& s, rust::Slice<const double> pts, double dx, double dy, double dz, double angle_deg,
                               double ox, double oy, double oz, double nx, double ny, double nz, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> faces_offset_tool(const Shape& s, rust::Slice<const double> pts, double value);
bool shape_valid(const Shape& s);
std::unique_ptr<Shape> fillet_var_t(const Shape& s, double r0, double r1, rust::Slice<const double> ctrl, bool smooth, rust::Slice<const double> mids, double tol,
                                    rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> defeature_t(const Shape& s, rust::Slice<const double> pts, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> face_offset_t(const Shape& s, rust::Slice<const double> pts, double value, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> extrude_taper(const Shape& face, double dx, double dy, double dz, double len, double angle_deg);
std::unique_ptr<Shape> thin_profile(const Shape& face, double t);
std::unique_ptr<Shape> thick_segments(rust::Slice<const double> segs, double nx, double ny, double nz, double t);
std::unique_ptr<Shape> edges_mode_t(const Shape& s, uint8_t mode, double v1, double v2, rust::Slice<const double> mids, rust::Slice<const uint8_t> flips, double tol, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
rust::Vec<double> cylinder_at(const Shape& s, double px, double py, double pz);
std::unique_ptr<Shape> thread_tool_ext(double ox, double oy, double oz, double dx, double dy, double dz, double major_d, double pitch, double length, double angle_deg);

std::unique_ptr<Shape> face_copy(const Shape& s, double px, double py, double pz);
std::unique_ptr<Shape> offset_surface(const Shape& s, double d);
std::unique_ptr<Shape> offset_surface_j(const Shape& s, double d, uint8_t join);
std::unique_ptr<Shape> fill_surface(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends);
std::unique_ptr<Shape> fill_surface_x(rust::Slice<const double> flat, rust::Slice<const uint32_t> ends, rust::Slice<const int32_t> orders, rust::Slice<const double> weights,
                                      const Shape& support, rust::Slice<const double> pts, rust::Vec<double>& errs);
std::unique_ptr<Shape> curve_shape(rust::Slice<const double> pts);
std::unique_ptr<Shape> bezier_faces(rust::Slice<const double> poles);
rust::Vec<double> sew_report(const Shape& s);
std::unique_ptr<Shape> clip_surface(const Shape& s, const Shape& cutter, double px, double py, double pz, bool extend);
rust::Vec<double> edge_pts_at(const Shape& s, double px, double py, double pz);
rust::Vec<double> edge_plane_pts(const Shape& s, double px, double py, double pz, double ox, double oy, double oz, double nx, double ny, double nz);
rust::Vec<double> free_edge_pts(const Shape& s, rust::Vec<uint32_t>& ends);
std::unique_ptr<Shape> face_propagate(const Shape& s, double px, double py, double pz, int32_t prop, bool complement, rust::Vec<uint32_t>& counts);
std::unique_ptr<Shape> extrude_curve(rust::Slice<const double> pts, double dx, double dy, double dz, double len);
std::unique_ptr<Shape> revolve_curve(rust::Slice<const double> pts, double ox, double oy, double oz, double dx, double dy, double dz, double angle_deg);
rust::Vec<double> surface_near(const Shape& s, double px, double py, double pz);
std::unique_ptr<Shape> grid_surface(rust::Slice<const double> flat, uint32_t nu, uint32_t nv);
rust::Vec<double> connect_check(const Shape& a, const Shape& b, uint32_t samples, double max_gap);
rust::Vec<double> signed_distances(const Shape& s, rust::Slice<const double> flat);
rust::Vec<double> iso_curves(const Shape& s, uint32_t n, rust::Vec<uint32_t>& ends);
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
std::unique_ptr<Shape> split_regions(const Shape& s, const Shape& tool, bool extend);
uint32_t child_count(const Shape& s);
std::unique_ptr<Shape> nth_child(const Shape& s, uint32_t i);
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
rust::Vec<double> hlr_project(const Shape& s, double dx, double dy, double dz, double xx, double xy, double xz, bool hidden,
                              rust::Vec<uint32_t>& ends, rust::Vec<uint8_t>& kinds);
rust::Vec<double> section_loops(const Shape& s, double ox, double oy, double oz, double nx, double ny, double nz, double half, rust::Vec<uint32_t>& ends);
std::unique_ptr<Shape> offset_face(const Shape& face, double d);
std::unique_ptr<Shape> box_tool(double ox, double oy, double oz, double xx, double xy, double xz, double zx, double zy, double zz, double lx, double ly, double lz);
std::unique_ptr<Shape> poly_prism(rust::Slice<const double> pts, double dx, double dy, double dz);
std::unique_ptr<Shape> sm_flange(const Shape& body, double px, double py, double pz, double t, double r, double angle_deg, double length,
                                 double off1, double off2, bool flip, bool flat, double k, rust::Vec<double>& info);
std::unique_ptr<Shape> sm_chain_sweep(const Shape& face, rust::Slice<const double> pts, double nx, double ny, double nz);
std::unique_ptr<Shape> from_brep(rust::Str text);
std::unique_ptr<Shape> hole_tool_ext(double ox, double oy, double oz, double dx, double dy, double dz, double diameter, double depth,
                                     uint8_t kind, double d2, double p2, double a3, double tip, double taper);
rust::Vec<double> ray_hit(const Shape& s, double px, double py, double pz, double dx, double dy, double dz);
bool same_face(const Shape& s, double ax, double ay, double az, double bx, double by, double bz);
std::unique_ptr<Shape> full_round_t(const Shape& s, double mx, double my, double mz, double width, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);
std::unique_ptr<Shape> face_face_fillet_t(const Shape& s, double ax, double ay, double az, double bx, double by, double bz, double r, rust::Slice<const int32_t> tags, rust::Vec<int32_t>& out);

int32_t face_index(const Shape& s, double px, double py, double pz, double limit);
void topology(const Shape& s, rust::Vec<double>& face_info, rust::Vec<double>& edge_mids, rust::Vec<uint32_t>& edge_faces);
MeshData tessellate(const Shape& s, double deflection, double angle);
MeshData tessellate_view(const Shape& s, double deflection, double angle, double curv_tol, double k_ref);
void discard_mesh(const Shape& s);
void set_mesh_threads(int32_t n);
/// Registra (endereço de uma bandeira atômica, ou 0) o cancelamento da thread atual.
void set_cancel_flag(size_t p);
/// Limite de tempo da thread atual, em segundos a partir de agora (negativo limpa).
void set_deadline(double segundos);
int32_t mesh_threads();
void set_mesh_algo(int32_t a);
int32_t mesh_algo();
int32_t mesh_pool_threads();
rust::Vec<double> bounding_box(const Shape& s);
rust::Vec<double> mass_props(const Shape& s);
rust::Vec<double> inertia_props(const Shape& s);
uint32_t count_faces(const Shape& s);
void export_step(const Shape& s, rust::Str path);
void export_stl(const Shape& s, rust::Str path, double deflection, double angle);
std::unique_ptr<Shape> import_step(rust::Str path, rust::Vec<rust::String>& info);
std::unique_ptr<XdeDoc> xde_new();
std::unique_ptr<XdeDoc> xde_read(rust::Str path, const XdeReadOpts& o);
std::unique_ptr<XdeDoc> xde_read_iges(rust::Str path, const XdeReadOpts& o);
XdeInfo xde_info(const XdeDoc& d);
rust::Vec<XdeProduct> xde_products(const XdeDoc& d);
std::unique_ptr<Shape> xde_shape(const XdeDoc& d, uint32_t i);
uint32_t xde_add(XdeDoc& d, const XdeProduct& p, const Shape& s);
void xde_add_component(XdeDoc& d, uint32_t assembly, const XdeInst& c);
void xde_write(XdeDoc& d, rust::Str path, const XdeWriteOpts& o);
void xde_pmi_add(XdeDoc& d, const XdePmi& m);
rust::Vec<XdePmi> xde_pmi_read(const XdeDoc& d);
void xde_write_iges(XdeDoc& d, rust::Str path, const XdeIgesOpts& o);
std::unique_ptr<Shape> xde_moved(const Shape& s, rust::Slice<const double> m);
uint32_t xde_solid_count(const Shape& s);
std::unique_ptr<Shape> xde_solid_at(const Shape& s, uint32_t i);
std::unique_ptr<Shape> xde_heal(const Shape& s, uint8_t level, double tol, rust::Vec<double>& stats);
rust::Vec<XdeDefect> xde_defects(const Shape& s, uint32_t limit);
rust::String occt_version();

}  // namespace fenkra
