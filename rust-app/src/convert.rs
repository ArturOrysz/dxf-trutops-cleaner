//! Convert DXF drawing → simplified LINE/ARC contours.

use crate::dxf_read::{read_dxf, DxfDrawing};
use crate::dxf_write::{write_r2000, Contour};
use crate::geom::{
    approximate_polyline_arcs, as_bulge_vertices, clean_vertices, count_arc_segments,
    dedupe_consecutive, dist, rdp_simplify, sample_bspline,
};
use chrono::Local;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub output: PathBuf,
    pub before: usize,
    pub after: usize,
    pub contours: usize,
    pub splines: usize,
    pub arc_segments: usize,
}

pub fn timestamped_output(input: &Path) -> PathBuf {
    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    input.with_file_name(format!("{stem}_{stamp}.dxf"))
}

fn collect_simplified(doc: &DxfDrawing, tolerance: f64) -> (Vec<Contour>, usize, usize, usize, usize) {
    let mut result = Vec::new();
    let mut originals = 0usize;
    let mut simplified = 0usize;
    let mut arc_segments = 0usize;

    for line in &doc.lines {
        let pts = vec![line.start, line.end];
        originals += 2;
        simplified += 2;
        result.push(Contour {
            verts: as_bulge_vertices(&pts),
            closed: false,
            layer: line.layer.clone(),
        });
    }

    let poly_sources: Vec<(Vec<(f64, f64)>, bool, String)> = doc
        .lwpolylines
        .iter()
        .map(|p| (p.points.clone(), p.closed, p.layer.clone()))
        .chain(
            doc.polylines
                .iter()
                .map(|p| (p.points.clone(), p.closed, p.layer.clone())),
        )
        .collect();

    for (pts, closed, layer) in poly_sources {
        let pts = dedupe_consecutive(&pts, 1e-9);
        originals += pts.len();
        let mut simp = rdp_simplify(&pts, tolerance);
        if closed && simp.len() > 2 {
            let a = simp[0];
            let b = *simp.last().unwrap();
            if dist(a, b) < 1e-6 {
                simp.pop();
            }
        }
        simplified += simp.len();
        if simp.len() >= 2 {
            result.push(Contour {
                verts: as_bulge_vertices(&simp),
                closed,
                layer,
            });
        }
    }

    for sp in &doc.splines {
        let samples = (sp.controls.len().saturating_mul(12)).max(32);
        let weights = if sp.weights.iter().all(|w| (*w - 1.0).abs() < 1e-9) {
            None
        } else {
            Some(sp.weights.as_slice())
        };
        let sampled = sample_bspline(&sp.controls, weights, &sp.knots, sp.degree, samples);
        originals += sampled.len().max(sp.controls.len());
        let mut verts = approximate_polyline_arcs(&sampled, tolerance);
        verts = clean_vertices(&verts, sp.closed);
        simplified += verts.len();
        arc_segments += count_arc_segments(&verts);
        if verts.len() >= 2 {
            result.push(Contour {
                verts,
                closed: sp.closed,
                layer: sp.layer.clone(),
            });
        }
    }

    (
        result,
        originals,
        simplified,
        doc.splines.len(),
        arc_segments,
    )
}

pub fn convert_file(
    input: &Path,
    tolerance: f64,
    output: Option<PathBuf>,
) -> Result<ConvertResult, String> {
    let doc = read_dxf(input)?;
    let (contours, before, after, splines, arc_segments) = collect_simplified(&doc, tolerance);
    if contours.is_empty() {
        return Err("Brak geometrii LINE/POLYLINE/LWPOLYLINE/SPLINE.".into());
    }
    let out = output.unwrap_or_else(|| timestamped_output(input));
    write_r2000(&contours, &out)?;
    Ok(ConvertResult {
        output: out,
        before,
        after,
        contours: contours.len(),
        splines,
        arc_segments,
    })
}
