//! Geometry helpers: RDP, bulge/arc math, Bezier and sampled-curve arc fitting.

use std::f64::consts::PI;

pub type Point2 = (f64, f64);
/// (x, y, bulge) — bulge applies to the segment toward the next vertex.
pub type Vertex = (f64, f64, f64);

const MAX_DEPTH: usize = 12;
const SAMPLE_COUNT: usize = 16;
const STRAIGHT_BULGE: f64 = 1e-6;

#[inline]
pub fn dist(a: Point2, b: Point2) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

pub fn dedupe_consecutive(points: &[Point2], eps: f64) -> Vec<Point2> {
    let mut out = Vec::new();
    for &p in points {
        if out
            .last()
            .map(|q| dist(*q, p) > eps)
            .unwrap_or(true)
        {
            out.push(p);
        }
    }
    out
}

fn dist_point_to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (px, py) = p;
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return dist(p, a);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0);
    dist(p, (ax + t * dx, ay + t * dy))
}

pub fn rdp_simplify(points: &[Point2], tolerance: f64) -> Vec<Point2> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let start = points[0];
    let end = points[points.len() - 1];
    let mut max_dist = 0.0;
    let mut index = 0;
    for (i, &p) in points.iter().enumerate().skip(1).take(points.len().saturating_sub(2)) {
        let d = dist_point_to_segment(p, start, end);
        if d > max_dist {
            max_dist = d;
            index = i;
        }
    }
    if max_dist <= tolerance {
        return vec![start, end];
    }
    let left = rdp_simplify(&points[..=index], tolerance);
    let right = rdp_simplify(&points[index..], tolerance);
    let mut out = left;
    out.pop();
    out.extend(right);
    out
}

/// Bulge from three points (start, end, point on arc). Returns 0 on degeneracy.
pub fn bulge_3_points(start: Point2, end: Point2, point: Point2) -> f64 {
    let (x1, y1) = start;
    let (x2, y2) = end;
    let (x3, y3) = point;
    let a = dist(start, end);
    let b = dist(end, point);
    let c = dist(point, start);
    if a < 1e-15 || b < 1e-15 || c < 1e-15 {
        return 0.0;
    }
    // Angle at `point` via cosine law is wrong for bulge; use inscribed angle formula:
    // bulge = tan(theta/4) related — standard CAD formula via area / chord:
    let area2 = x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2);
    if area2.abs() < 1e-18 {
        return 0.0;
    }
    // Radius from circumcircle
    let d = 2.0
        * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2));
    if d.abs() < 1e-18 {
        return 0.0;
    }
    let ux = ((x1 * x1 + y1 * y1) * (y2 - y3)
        + (x2 * x2 + y2 * y2) * (y3 - y1)
        + (x3 * x3 + y3 * y3) * (y1 - y2))
        / d;
    let uy = ((x1 * x1 + y1 * y1) * (x3 - x2)
        + (x2 * x2 + y2 * y2) * (x1 - x3)
        + (x3 * x3 + y3 * y3) * (x2 - x1))
        / d;
    let r = dist((ux, uy), start);
    if r < 1e-15 {
        return 0.0;
    }
    // Angle from start to end around center
    let a0 = (y1 - uy).atan2(x1 - ux);
    let a1 = (y2 - uy).atan2(x2 - ux);
    let mut sweep = a1 - a0;
    // Choose sweep so that `point` lies on the arc
    let ap = (y3 - uy).atan2(x3 - ux);
    let mut s_ccw = sweep;
    while s_ccw <= 0.0 {
        s_ccw += 2.0 * PI;
    }
    while s_ccw > 2.0 * PI {
        s_ccw -= 2.0 * PI;
    }
    let mut p_from_start = ap - a0;
    while p_from_start < 0.0 {
        p_from_start += 2.0 * PI;
    }
    while p_from_start >= 2.0 * PI {
        p_from_start -= 2.0 * PI;
    }
    let use_ccw = p_from_start <= s_ccw + 1e-9;
    if !use_ccw {
        sweep = if sweep > 0.0 {
            sweep - 2.0 * PI
        } else {
            sweep
        };
        if sweep >= 0.0 {
            sweep -= 2.0 * PI;
        }
    } else if sweep <= 0.0 {
        sweep += 2.0 * PI;
    }
    (sweep / 4.0).tan()
}

/// Convert bulge segment to arc center / angles (radians) / radius.
pub fn bulge_to_arc(start: Point2, end: Point2, bulge: f64) -> (Point2, f64, f64, f64) {
    let (x0, y0) = start;
    let (x1, y1) = end;
    let chord = dist(start, end);
    if chord < 1e-15 || bulge.abs() < STRAIGHT_BULGE {
        return (((x0 + x1) * 0.5, (y0 + y1) * 0.5), 0.0, 0.0, 0.0);
    }
    let sagitta_factor = (1.0 - bulge * bulge) / (4.0 * bulge);
    let mx = (x0 + x1) * 0.5;
    let my = (y0 + y1) * 0.5;
    let dx = x1 - x0;
    let dy = y1 - y0;
    // Perpendicular
    let cx = mx - sagitta_factor * dy;
    let cy = my + sagitta_factor * dx;
    let radius = dist((cx, cy), start);
    let start_angle = (y0 - cy).atan2(x0 - cx);
    let end_angle = (y1 - cy).atan2(x1 - cx);
    ((cx, cy), start_angle, end_angle, radius)
}

fn point_to_arc_distance(p: Point2, start: Point2, end: Point2, bulge: f64) -> f64 {
    if bulge.abs() < STRAIGHT_BULGE {
        return dist_point_to_segment(p, start, end);
    }
    let (center, _, _, radius) = bulge_to_arc(start, end, bulge);
    (dist(p, center) - radius.abs()).abs()
}

fn bezier_point(ctrl: &[Point2; 4], t: f64) -> Point2 {
    let u = 1.0 - t;
    let uu = u * u;
    let uuu = uu * u;
    let tt = t * t;
    let ttt = tt * t;
    let x = uuu * ctrl[0].0
        + 3.0 * uu * t * ctrl[1].0
        + 3.0 * u * tt * ctrl[2].0
        + ttt * ctrl[3].0;
    let y = uuu * ctrl[0].1
        + 3.0 * uu * t * ctrl[1].1
        + 3.0 * u * tt * ctrl[2].1
        + ttt * ctrl[3].1;
    (x, y)
}

fn lerp(a: Point2, b: Point2, t: f64) -> Point2 {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn split_bezier(ctrl: &[Point2; 4], t: f64) -> ([Point2; 4], [Point2; 4]) {
    let q0 = lerp(ctrl[0], ctrl[1], t);
    let q1 = lerp(ctrl[1], ctrl[2], t);
    let q2 = lerp(ctrl[2], ctrl[3], t);
    let r0 = lerp(q0, q1, t);
    let r1 = lerp(q1, q2, t);
    let s = lerp(r0, r1, t);
    ([ctrl[0], q0, r0, s], [s, r1, q2, ctrl[3]])
}

fn fit_error(ctrl: &[Point2; 4], bulge: f64) -> f64 {
    let start = ctrl[0];
    let end = ctrl[3];
    let mut max_err = 0.0;
    for i in 1..SAMPLE_COUNT {
        let t = i as f64 / SAMPLE_COUNT as f64;
        let pt = bezier_point(ctrl, t);
        let err = point_to_arc_distance(pt, start, end, bulge);
        if err > max_err {
            max_err = err;
        }
    }
    max_err
}

pub fn approximate_bezier(ctrl: &[Point2; 4], tolerance: f64) -> Vec<Vertex> {
    approximate_bezier_rec(ctrl, tolerance, 0)
}

fn approximate_bezier_rec(ctrl: &[Point2; 4], tolerance: f64, depth: usize) -> Vec<Vertex> {
    let start = ctrl[0];
    let end = ctrl[3];
    if dist(start, end) < 1e-12 {
        return vec![(start.0, start.1, 0.0)];
    }
    let mid = bezier_point(ctrl, 0.5);
    let mut bulge = bulge_3_points(start, end, mid);
    if bulge.abs() < STRAIGHT_BULGE {
        bulge = 0.0;
    }
    let error = if bulge.abs() <= 1.0 {
        fit_error(ctrl, bulge)
    } else {
        tolerance + 1.0
    };
    if error <= tolerance || depth >= MAX_DEPTH {
        return vec![(start.0, start.1, bulge), (end.0, end.1, 0.0)];
    }
    let (left, right) = split_bezier(ctrl, 0.5);
    let mut left_v = approximate_bezier_rec(&left, tolerance, depth + 1);
    let right_v = approximate_bezier_rec(&right, tolerance, depth + 1);
    left_v.pop();
    left_v.extend(right_v);
    left_v
}

/// Fit arcs/lines to a sampled polyline (used for general B-splines).
pub fn approximate_polyline_arcs(points: &[Point2], tolerance: f64) -> Vec<Vertex> {
    let pts = dedupe_consecutive(points, 1e-9);
    if pts.len() < 2 {
        return Vec::new();
    }
    approximate_poly_rec(&pts, tolerance, 0)
}

fn approximate_poly_rec(pts: &[Point2], tolerance: f64, depth: usize) -> Vec<Vertex> {
    if pts.len() < 2 {
        return Vec::new();
    }
    if pts.len() == 2 {
        return vec![(pts[0].0, pts[0].1, 0.0), (pts[1].0, pts[1].1, 0.0)];
    }
    let start = pts[0];
    let end = pts[pts.len() - 1];
    let mid = pts[pts.len() / 2];
    let mut bulge = bulge_3_points(start, end, mid);
    if bulge.abs() < STRAIGHT_BULGE {
        bulge = 0.0;
    }
    let mut max_err = 0.0;
    if bulge.abs() <= 1.0 {
        for &p in &pts[1..pts.len() - 1] {
            let err = point_to_arc_distance(p, start, end, bulge);
            if err > max_err {
                max_err = err;
            }
        }
    } else {
        max_err = tolerance + 1.0;
    }
    if max_err <= tolerance || depth >= MAX_DEPTH || pts.len() <= 3 {
        return vec![(start.0, start.1, bulge), (end.0, end.1, 0.0)];
    }
    let mid_i = pts.len() / 2;
    let mut left = approximate_poly_rec(&pts[..=mid_i], tolerance, depth + 1);
    let right = approximate_poly_rec(&pts[mid_i..], tolerance, depth + 1);
    left.pop();
    left.extend(right);
    left
}

pub fn merge_vertices(segments: &[Vec<Vertex>], eps: f64) -> Vec<Vertex> {
    let mut out: Vec<Vertex> = Vec::new();
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        if out.is_empty() {
            out.extend(seg.iter().copied());
            continue;
        }
        let first = seg[0];
        let last = *out.last().unwrap();
        if dist((first.0, first.1), (last.0, last.1)) <= eps {
            let n = out.len();
            out[n - 1] = (last.0, last.1, first.2);
            out.extend(seg[1..].iter().copied());
        } else {
            out.extend(seg.iter().copied());
        }
    }
    out
}

pub fn clean_vertices(merged: &[Vertex], closed: bool) -> Vec<Vertex> {
    if merged.is_empty() {
        return Vec::new();
    }
    let mut cleaned: Vec<Vertex> = vec![merged[0]];
    for &v in &merged[1..] {
        let prev = *cleaned.last().unwrap();
        if dist((v.0, v.1), (prev.0, prev.1)) > 1e-9 {
            cleaned.push(v);
        } else if v.2.abs() > prev.2.abs() {
            let n = cleaned.len();
            cleaned[n - 1] = (prev.0, prev.1, v.2);
        }
    }
    if closed && cleaned.len() > 2 {
        let a = cleaned[0];
        let b = *cleaned.last().unwrap();
        if dist((a.0, a.1), (b.0, b.1)) < 1e-6 {
            cleaned.pop();
        }
    }
    if let Some(last) = cleaned.last_mut() {
        last.2 = 0.0;
    }
    cleaned
}

pub fn count_arc_segments(vertices: &[Vertex]) -> usize {
    if vertices.len() < 2 {
        return 0;
    }
    vertices[..vertices.len() - 1]
        .iter()
        .filter(|v| v.2.abs() >= STRAIGHT_BULGE)
        .count()
}

pub fn as_bulge_vertices(points: &[Point2]) -> Vec<Vertex> {
    points.iter().map(|&(x, y)| (x, y, 0.0)).collect()
}

// --- B-spline sampling (Cox–de Boor) ---

fn find_span(n: usize, degree: usize, u: f64, knots: &[f64]) -> usize {
    if u >= knots[n + 1] {
        return n;
    }
    if u <= knots[degree] {
        return degree;
    }
    let mut low = degree;
    let mut high = n + 1;
    let mut mid = (low + high) / 2;
    while u < knots[mid] || u >= knots[mid + 1] {
        if u < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }
    mid
}

fn basis_funs(span: usize, u: f64, degree: usize, knots: &[f64]) -> Vec<f64> {
    let mut n = vec![0.0; degree + 1];
    let mut left = vec![0.0; degree + 1];
    let mut right = vec![0.0; degree + 1];
    n[0] = 1.0;
    for j in 1..=degree {
        left[j] = u - knots[span + 1 - j];
        right[j] = knots[span + j] - u;
        let mut saved = 0.0;
        for r in 0..j {
            let temp = n[r] / (right[r + 1] + left[j - r]);
            n[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        n[j] = saved;
    }
    n
}

pub fn bspline_point(
    controls: &[Point2],
    weights: Option<&[f64]>,
    knots: &[f64],
    degree: usize,
    u: f64,
) -> Point2 {
    let n = controls.len() - 1;
    let span = find_span(n, degree, u, knots);
    let basis = basis_funs(span, u, degree, knots);
    if let Some(w) = weights {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut wsum = 0.0;
        for i in 0..=degree {
            let idx = span - degree + i;
            let bw = basis[i] * w.get(idx).copied().unwrap_or(1.0);
            x += bw * controls[idx].0;
            y += bw * controls[idx].1;
            wsum += bw;
        }
        if wsum.abs() < 1e-15 {
            return controls[span.min(n)];
        }
        (x / wsum, y / wsum)
    } else {
        let mut x = 0.0;
        let mut y = 0.0;
        for i in 0..=degree {
            let idx = span - degree + i;
            x += basis[i] * controls[idx].0;
            y += basis[i] * controls[idx].1;
        }
        (x, y)
    }
}

pub fn sample_bspline(
    controls: &[Point2],
    weights: Option<&[f64]>,
    knots: &[f64],
    degree: usize,
    samples: usize,
) -> Vec<Point2> {
    if controls.len() < 2 || knots.len() < controls.len() + degree + 1 {
        return controls.to_vec();
    }
    let u0 = knots[degree];
    let u1 = knots[controls.len()];
    if (u1 - u0).abs() < 1e-15 {
        return controls.to_vec();
    }
    let n = samples.max(8);
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        // Stay slightly inside open interval at the end for numerical stability
        let u = if i == n {
            u1 - 1e-12 * (u1 - u0).abs().max(1.0)
        } else {
            u0 + t * (u1 - u0)
        };
        pts.push(bspline_point(controls, weights, knots, degree, u));
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_circle_bezier_one_arc() {
        let k = 0.5522847498;
        let ctrl = [(1.0, 0.0), (1.0, k), (k, 1.0), (0.0, 1.0)];
        let verts = approximate_bezier(&ctrl, 0.01);
        assert!(count_arc_segments(&verts) >= 1);
        assert!((verts[0].2 - 0.41421356235).abs() < 0.05);
    }
}
