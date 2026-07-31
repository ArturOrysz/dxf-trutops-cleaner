//! Minimal ASCII DXF reader for LINE / ARC / LWPOLYLINE / POLYLINE / SPLINE.

use crate::geom::Point2;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DxfLine {
    pub layer: String,
    pub start: Point2,
    pub end: Point2,
}

#[derive(Debug, Clone)]
pub struct DxfLwPolyline {
    pub layer: String,
    pub closed: bool,
    pub points: Vec<Point2>,
}

#[derive(Debug, Clone)]
pub struct DxfPolyline {
    pub layer: String,
    pub closed: bool,
    pub points: Vec<Point2>,
}

#[derive(Debug, Clone)]
pub struct DxfSpline {
    pub layer: String,
    pub closed: bool,
    pub degree: usize,
    pub knots: Vec<f64>,
    pub controls: Vec<Point2>,
    pub weights: Vec<f64>,
}

#[derive(Debug, Default)]
pub struct DxfDrawing {
    pub lines: Vec<DxfLine>,
    pub lwpolylines: Vec<DxfLwPolyline>,
    pub polylines: Vec<DxfPolyline>,
    pub splines: Vec<DxfSpline>,
}

fn pairs_from_text(text: &str) -> Vec<(i32, String)> {
    let mut lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
    // Drop trailing empty
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i].trim().parse::<i32>().unwrap_or(0);
        let value = lines[i + 1].to_string();
        out.push((code, value));
        i += 2;
    }
    out
}

fn parse_f64(s: &str) -> f64 {
    s.trim().parse().unwrap_or(0.0)
}

fn parse_i32(s: &str) -> i32 {
    s.trim().parse().unwrap_or(0)
}

pub fn read_dxf(path: &Path) -> Result<DxfDrawing, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    // DXF often CP1250/latin1 — lossy UTF-8 is fine for numbers/entity names
    let text = String::from_utf8_lossy(&bytes);
    let pairs = pairs_from_text(&text);
    let mut doc = DxfDrawing::default();

    let mut i = 0;
    let mut in_entities = false;
    while i < pairs.len() {
        let (code, ref val) = pairs[i];
        if code == 0 && val.trim() == "SECTION" {
            // look ahead for 2 / ENTITIES
            if let Some((2, name)) = pairs.get(i + 1) {
                in_entities = name.trim() == "ENTITIES";
            }
            i += 1;
            continue;
        }
        if code == 0 && val.trim() == "ENDSEC" {
            in_entities = false;
            i += 1;
            continue;
        }
        if !in_entities {
            i += 1;
            continue;
        }
        if code != 0 {
            i += 1;
            continue;
        }
        match val.trim() {
            "LINE" => {
                let (ent, next) = read_line(&pairs, i + 1);
                doc.lines.push(ent);
                i = next;
            }
            "LWPOLYLINE" => {
                let (ent, next) = read_lwpolyline(&pairs, i + 1);
                doc.lwpolylines.push(ent);
                i = next;
            }
            "POLYLINE" => {
                let (ent, next) = read_polyline(&pairs, i + 1);
                doc.polylines.push(ent);
                i = next;
            }
            "SPLINE" => {
                let (ent, next) = read_spline(&pairs, i + 1);
                doc.splines.push(ent);
                i = next;
            }
            _ => i += 1,
        }
    }
    Ok(doc)
}

fn read_until_next_entity(pairs: &[(i32, String)], start: usize) -> usize {
    let mut i = start;
    while i < pairs.len() {
        if pairs[i].0 == 0 {
            return i;
        }
        i += 1;
    }
    pairs.len()
}

fn read_line(pairs: &[(i32, String)], start: usize) -> (DxfLine, usize) {
    let end = read_until_next_entity(pairs, start);
    let mut layer = "0".to_string();
    let mut x1 = 0.0;
    let mut y1 = 0.0;
    let mut x2 = 0.0;
    let mut y2 = 0.0;
    for j in start..end {
        match pairs[j].0 {
            8 => layer = pairs[j].1.trim().to_string(),
            10 => x1 = parse_f64(&pairs[j].1),
            20 => y1 = parse_f64(&pairs[j].1),
            11 => x2 = parse_f64(&pairs[j].1),
            21 => y2 = parse_f64(&pairs[j].1),
            _ => {}
        }
    }
    (
        DxfLine {
            layer,
            start: (x1, y1),
            end: (x2, y2),
        },
        end,
    )
}

fn read_lwpolyline(pairs: &[(i32, String)], start: usize) -> (DxfLwPolyline, usize) {
    let end = read_until_next_entity(pairs, start);
    let mut layer = "0".to_string();
    let mut flags = 0i32;
    let mut points = Vec::new();
    let mut pending_x: Option<f64> = None;
    for j in start..end {
        match pairs[j].0 {
            8 => layer = pairs[j].1.trim().to_string(),
            70 => flags = parse_i32(&pairs[j].1),
            10 => pending_x = Some(parse_f64(&pairs[j].1)),
            20 => {
                if let Some(x) = pending_x.take() {
                    points.push((x, parse_f64(&pairs[j].1)));
                }
            }
            _ => {}
        }
    }
    (
        DxfLwPolyline {
            layer,
            closed: (flags & 1) != 0,
            points,
        },
        end,
    )
}

fn read_polyline(pairs: &[(i32, String)], start: usize) -> (DxfPolyline, usize) {
    // POLYLINE header then VERTEX entities until SEQEND
    let mut layer = "0".to_string();
    let mut flags = 0i32;
    let mut i = start;
    while i < pairs.len() && pairs[i].0 != 0 {
        match pairs[i].0 {
            8 => layer = pairs[i].1.trim().to_string(),
            70 => flags = parse_i32(&pairs[i].1),
            _ => {}
        }
        i += 1;
    }
    let mut points = Vec::new();
    while i < pairs.len() {
        if pairs[i].0 == 0 && pairs[i].1.trim() == "SEQEND" {
            i = read_until_next_entity(pairs, i + 1);
            break;
        }
        if pairs[i].0 == 0 && pairs[i].1.trim() == "VERTEX" {
            i += 1;
            let mut x = 0.0;
            let mut y = 0.0;
            while i < pairs.len() && pairs[i].0 != 0 {
                match pairs[i].0 {
                    10 => x = parse_f64(&pairs[i].1),
                    20 => y = parse_f64(&pairs[i].1),
                    _ => {}
                }
                i += 1;
            }
            points.push((x, y));
            continue;
        }
        if pairs[i].0 == 0 {
            // unexpected entity
            break;
        }
        i += 1;
    }
    (
        DxfPolyline {
            layer,
            closed: (flags & 1) != 0,
            points,
        },
        i,
    )
}

fn read_spline(pairs: &[(i32, String)], start: usize) -> (DxfSpline, usize) {
    let end = read_until_next_entity(pairs, start);
    let mut layer = "0".to_string();
    let mut flags = 0i32;
    let mut degree = 3usize;
    let mut knots = Vec::new();
    let mut controls = Vec::new();
    let mut weights = Vec::new();
    let mut pending_x: Option<f64> = None;
    for j in start..end {
        match pairs[j].0 {
            8 => layer = pairs[j].1.trim().to_string(),
            70 => flags = parse_i32(&pairs[j].1),
            71 => degree = parse_i32(&pairs[j].1).max(1) as usize,
            40 => knots.push(parse_f64(&pairs[j].1)),
            41 => weights.push(parse_f64(&pairs[j].1)),
            10 => pending_x = Some(parse_f64(&pairs[j].1)),
            20 => {
                if let Some(x) = pending_x.take() {
                    controls.push((x, parse_f64(&pairs[j].1)));
                }
            }
            _ => {}
        }
    }
    // If no weights, treat as non-rational
    if weights.len() != controls.len() {
        weights = vec![1.0; controls.len()];
    }
    (
        DxfSpline {
            layer,
            closed: (flags & 1) != 0,
            degree,
            knots,
            controls,
            weights,
        },
        end,
    )
}
