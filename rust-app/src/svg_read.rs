//! Read SVG geometry → cubic Bezier / line segments for arc fitting.
//!
//! SVG is not “only Bezier”: paths also have lines, quadratic curves, arcs;
//! plus shapes (circle, rect, …). `usvg` flattens that to lines + cubics/quads
//! (elliptical arcs become cubics), which we then convert to LINE+ARC.

use crate::dxf_write::Contour;
use crate::geom::{
    approximate_bezier, as_bulge_vertices, clean_vertices, count_arc_segments, dist, merge_vertices,
    Point2, Vertex,
};
use std::fs;
use std::path::Path;
use usvg::{tiny_skia_path::PathSegment, Node, Options, Tree};

#[derive(Debug, Default)]
pub struct SvgStats {
    pub path_count: usize,
    pub before: usize,
    pub after: usize,
    pub arc_segments: usize,
}

fn quad_to_cubic(p0: Point2, p1: Point2, p2: Point2) -> [Point2; 4] {
    [
        p0,
        (
            p0.0 + 2.0 / 3.0 * (p1.0 - p0.0),
            p0.1 + 2.0 / 3.0 * (p1.1 - p0.1),
        ),
        (
            p2.0 + 2.0 / 3.0 * (p1.0 - p2.0),
            p2.1 + 2.0 / 3.0 * (p1.1 - p2.1),
        ),
        p2,
    ]
}

fn flip_y(p: Point2, height: f64) -> Point2 {
    (p.0, height - p.1)
}

fn path_to_contour(
    path: &usvg::tiny_skia_path::Path,
    height: f64,
    tolerance: f64,
    layer: &str,
) -> Option<(Contour, usize, usize)> {
    let mut segments: Vec<Vec<Vertex>> = Vec::new();
    let mut cursor = (0.0_f64, 0.0_f64);
    let mut start = cursor;
    let mut sub_segs: Vec<Vec<Vertex>> = Vec::new();
    let mut before = 0usize;

    let push_line = |from: Point2, to: Point2, out: &mut Vec<Vec<Vertex>>, before: &mut usize| {
        if dist(from, to) < 1e-12 {
            return;
        }
        *before += 2;
        out.push(as_bulge_vertices(&[from, to]));
    };

    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                if !sub_segs.is_empty() {
                    segments.extend(sub_segs.drain(..));
                }
                cursor = flip_y((p.x as f64, p.y as f64), height);
                start = cursor;
            }
            PathSegment::LineTo(p) => {
                let to = flip_y((p.x as f64, p.y as f64), height);
                push_line(cursor, to, &mut sub_segs, &mut before);
                cursor = to;
            }
            PathSegment::QuadTo(p1, p2) => {
                let c1 = flip_y((p1.x as f64, p1.y as f64), height);
                let to = flip_y((p2.x as f64, p2.y as f64), height);
                let ctrl = quad_to_cubic(cursor, c1, to);
                before += 4;
                let verts = approximate_bezier(&ctrl, tolerance);
                if verts.len() >= 2 {
                    sub_segs.push(verts);
                }
                cursor = to;
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                let ctrl = [
                    cursor,
                    flip_y((p1.x as f64, p1.y as f64), height),
                    flip_y((p2.x as f64, p2.y as f64), height),
                    flip_y((p3.x as f64, p3.y as f64), height),
                ];
                before += 4;
                let verts = approximate_bezier(&ctrl, tolerance);
                if verts.len() >= 2 {
                    sub_segs.push(verts);
                }
                cursor = ctrl[3];
            }
            PathSegment::Close => {
                push_line(cursor, start, &mut sub_segs, &mut before);
                cursor = start;
                if !sub_segs.is_empty() {
                    segments.extend(sub_segs.drain(..));
                }
            }
        }
    }
    if !sub_segs.is_empty() {
        segments.extend(sub_segs);
    }

    let merged = merge_vertices(&segments, 1e-9);
    let closed = !merged.is_empty()
        && dist(
            (merged[0].0, merged[0].1),
            (
                merged.last().unwrap().0,
                merged.last().unwrap().1,
            ),
        ) < 1e-4;
    let verts = clean_vertices(&merged, closed);
    if verts.len() < 2 {
        return None;
    }
    let after = verts.len();
    Some((
        Contour {
            verts,
            closed,
            layer: layer.to_string(),
        },
        before,
        after,
    ))
}

fn walk_group(
    group: &usvg::Group,
    height: f64,
    tolerance: f64,
    out: &mut Vec<Contour>,
    stats: &mut SvgStats,
) {
    for node in group.children() {
        match node {
            Node::Group(g) => walk_group(g, height, tolerance, out, stats),
            Node::Path(path) => {
                let data = path.data();
                if let Some((contour, before, after)) =
                    path_to_contour(data, height, tolerance, "0")
                {
                    stats.path_count += 1;
                    stats.before += before;
                    stats.after += after;
                    stats.arc_segments += count_arc_segments(&contour.verts);
                    out.push(contour);
                }
            }
            Node::Image(_) | Node::Text(_) => {}
        }
    }
}

pub fn read_svg(path: &Path, tolerance: f64) -> Result<(Vec<Contour>, SvgStats), String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    let tree = Tree::from_data(&data, &Options::default()).map_err(|e| format!("SVG: {e}"))?;
    let height = tree.size().height() as f64;
    let mut contours = Vec::new();
    let mut stats = SvgStats::default();
    walk_group(tree.root(), height, tolerance, &mut contours, &mut stats);
    if contours.is_empty() {
        return Err("Brak geometrii sciezek w SVG.".into());
    }
    Ok((contours, stats))
}
