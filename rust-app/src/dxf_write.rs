//! Write AutoCAD/TruTops-compatible DXF R2000 (AC1015) with only LINE and ARC.
//!
//! Uses an ezdxf-generated empty R2000 skeleton (TABLES/BLOCKS/OBJECTS) and
//! injects ENTITIES. Near-flat arcs are written as LINE so TruTops canvas
//! extents stay on the real geometry (huge arc centers blow up the view).

use crate::geom::{bulge_to_arc, Vertex};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const TEMPLATE: &str = include_str!("../assets/r2000_empty.dxf");
const MODEL_SPACE_OWNER: &str = "17"; // *Model_Space BLOCK_RECORD handle in template

pub struct Contour {
    pub verts: Vec<Vertex>,
    pub closed: bool,
    pub layer: String,
}

struct Writer<'a, W: Write> {
    w: &'a mut W,
    next_handle: u64,
}

impl<'a, W: Write> Writer<'a, W> {
    fn pair(&mut self, code: i32, value: &str) -> std::io::Result<()> {
        write!(self.w, "{code}\r\n{value}\r\n")
    }

    fn pair_f(&mut self, code: i32, value: f64) -> std::io::Result<()> {
        self.pair(code, &format!("{value:.12}"))
    }

    fn alloc_handle(&mut self) -> String {
        let h = self.next_handle;
        self.next_handle += 1;
        format!("{h:X}")
    }

    fn write_line(&mut self, layer: &str, a: (f64, f64), b: (f64, f64)) -> std::io::Result<()> {
        let handle = self.alloc_handle();
        self.pair(0, "LINE")?;
        self.pair(5, &handle)?;
        self.pair(330, MODEL_SPACE_OWNER)?;
        self.pair(100, "AcDbEntity")?;
        self.pair(8, layer)?;
        self.pair(100, "AcDbLine")?;
        self.pair_f(10, a.0)?;
        self.pair_f(20, a.1)?;
        self.pair_f(30, 0.0)?;
        self.pair_f(11, b.0)?;
        self.pair_f(21, b.1)?;
        self.pair_f(31, 0.0)?;
        // Explicit Z-up extrusion — Trumpf importers can misplace geometry without it
        self.pair_f(210, 0.0)?;
        self.pair_f(220, 0.0)?;
        self.pair_f(230, 1.0)?;
        Ok(())
    }

    fn write_arc(
        &mut self,
        layer: &str,
        center: (f64, f64),
        radius: f64,
        start_deg: f64,
        end_deg: f64,
    ) -> std::io::Result<()> {
        let handle = self.alloc_handle();
        self.pair(0, "ARC")?;
        self.pair(5, &handle)?;
        self.pair(330, MODEL_SPACE_OWNER)?;
        self.pair(100, "AcDbEntity")?;
        self.pair(8, layer)?;
        self.pair(100, "AcDbCircle")?;
        self.pair_f(10, center.0)?;
        self.pair_f(20, center.1)?;
        self.pair_f(30, 0.0)?;
        self.pair_f(40, radius.abs())?;
        self.pair_f(210, 0.0)?;
        self.pair_f(220, 0.0)?;
        self.pair_f(230, 1.0)?;
        self.pair(100, "AcDbArc")?;
        self.pair_f(50, start_deg)?;
        self.pair_f(51, end_deg)?;
        Ok(())
    }

    fn write_segment(&mut self, layer: &str, start: Vertex, end: Vertex) -> std::io::Result<()> {
        let (x0, y0, bulge) = start;
        let (x1, y1, _) = end;
        if bulge.abs() < 1e-6 {
            return self.write_line(layer, (x0, y0), (x1, y1));
        }

        // Central angle from bulge: theta = 4 * atan(bulge)
        let central = 4.0 * bulge.atan().abs();
        let (center, a0, a1, radius) = bulge_to_arc((x0, y0), (x1, y1), bulge);
        let chord = ((x1 - x0).hypot(y1 - y0)).max(1e-12);

        // TruTops uses arc centers for view bounds — near-flat huge arcs make parts "vanish".
        if radius.abs() < 1e-9
            || central < 2.0_f64.to_radians()
            || radius.abs() > 1500.0
            || radius.abs() > chord * 80.0
        {
            return self.write_line(layer, (x0, y0), (x1, y1));
        }

        let (start_deg, end_deg) = if bulge >= 0.0 {
            (normalize_deg(a0.to_degrees()), normalize_deg(a1.to_degrees()))
        } else {
            (normalize_deg(a1.to_degrees()), normalize_deg(a0.to_degrees()))
        };
        self.write_arc(layer, center, radius, start_deg, end_deg)
    }
}

fn normalize_deg(mut a: f64) -> f64 {
    while a < 0.0 {
        a += 360.0;
    }
    while a >= 360.0 {
        a -= 360.0;
    }
    a
}

fn extents(contours: &[Contour]) -> ((f64, f64), (f64, f64)) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut any = false;
    for c in contours {
        for &(x, y, _) in &c.verts {
            any = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !any {
        return ((0.0, 0.0), (1.0, 1.0));
    }
    ((min_x, min_y), (max_x, max_y))
}

fn split_template_owned(template: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = template
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .collect();
    let mut ent_idx = None;
    for i in 0..lines.len().saturating_sub(3) {
        if lines[i].trim() == "0"
            && lines[i + 1].trim() == "SECTION"
            && lines[i + 2].trim() == "2"
            && lines[i + 3].trim() == "ENTITIES"
        {
            ent_idx = Some(i);
            break;
        }
    }
    let i = ent_idx.ok_or_else(|| "Brak sekcji ENTITIES w szablonie DXF".to_string())?;
    let mut end_idx = None;
    for j in (i + 4)..lines.len().saturating_sub(1) {
        if lines[j].trim() == "0" && lines[j + 1].trim() == "ENDSEC" {
            end_idx = Some(j);
            break;
        }
    }
    let j = end_idx.ok_or_else(|| "Brak ENDSEC po ENTITIES".to_string())?;
    let prefix = lines[..=i + 3].join("\r\n") + "\r\n";
    let suffix = lines[j..].join("\r\n") + "\r\n";
    Ok((prefix, suffix))
}

/// Write DXF R2000 that AutoCAD and TruTops open (LINE + ARC only).
pub fn write_r2000(contours: &[Contour], path: &Path) -> Result<(), String> {
    let (_extmin, _extmax) = extents(contours);
    let (prefix, suffix) = split_template_owned(TEMPLATE)?;

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut file_w = BufWriter::new(file);
    file_w
        .write_all(prefix.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut writer = Writer {
        w: &mut file_w,
        next_handle: 0x400,
    };

    for c in contours {
        if c.verts.len() < 2 {
            continue;
        }
        let layer = if c.layer.is_empty() {
            "0"
        } else {
            c.layer.as_str()
        };
        let n = c.verts.len();
        for i in 0..n - 1 {
            writer
                .write_segment(layer, c.verts[i], c.verts[i + 1])
                .map_err(|e| e.to_string())?;
        }
        if c.closed && n >= 2 {
            writer
                .write_segment(layer, c.verts[n - 1], c.verts[0])
                .map_err(|e| e.to_string())?;
        }
    }

    file_w
        .write_all(suffix.as_bytes())
        .map_err(|e| e.to_string())?;
    file_w.flush().map_err(|e| e.to_string())?;
    Ok(())
}
