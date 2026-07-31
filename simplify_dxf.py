#!/usr/bin/env python3
"""
Upraszcza geometrię DXF (gęste POLYLINE z Etsy) do czystych LWPOLYLINE
z wierzchołkami tylko w załomieniach — bez nadmiarowych punktów w TruTops.

SPLINE (krzywe Béziera) konwertuje na LWPOLYLINE z segmentami prostymi i łukowymi (bulge).

Wymaga: pip install ezdxf
"""

from __future__ import annotations

import argparse
import math
import sys
from datetime import datetime
from pathlib import Path

import ezdxf
from ezdxf import bbox
from ezdxf.math import bulge_to_arc

from arc_fit import count_arc_segments, spline_to_vertices

# (x, y, bulge) — bulge dotyczy odcinka do następnego wierzchołka
Vertex = tuple[float, float, float]
Contour = tuple[list[Vertex], bool, str]


def dedupe_consecutive(points: list[tuple[float, float]], eps: float = 1e-9) -> list[tuple[float, float]]:
    if not points:
        return []
    out = [points[0]]
    for p in points[1:]:
        if math.hypot(p[0] - out[-1][0], p[1] - out[-1][1]) > eps:
            out.append(p)
    return out


def dist_point_to_segment(
    p: tuple[float, float],
    a: tuple[float, float],
    b: tuple[float, float],
) -> float:
    ax, ay = a
    bx, by = b
    px, py = p
    dx, dy = bx - ax, by - ay
    len_sq = dx * dx + dy * dy
    if len_sq == 0:
        return math.hypot(px - ax, py - ay)
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / len_sq))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def rdp_simplify(points: list[tuple[float, float]], tolerance: float) -> list[tuple[float, float]]:
    """Ramer–Douglas–Peucker — usuwa punkty współliniowe / leżące na prostej."""
    if len(points) <= 2:
        return points[:]

    start, end = points[0], points[-1]
    max_dist = 0.0
    index = 0
    for i in range(1, len(points) - 1):
        d = dist_point_to_segment(points[i], start, end)
        if d > max_dist:
            max_dist = d
            index = i

    if max_dist <= tolerance:
        return [start, end]

    left = rdp_simplify(points[: index + 1], tolerance)
    right = rdp_simplify(points[index:], tolerance)
    return left[:-1] + right


def polyline_points_2d(entity) -> tuple[list[tuple[float, float]], bool]:
    dxftype = entity.dxftype()
    if dxftype == "LWPOLYLINE":
        closed = entity.closed
        pts = [(float(x), float(y)) for x, y, *_ in entity.get_points("xy")]
        return pts, closed
    if dxftype == "POLYLINE":
        closed = entity.is_closed
        pts = [(v.dxf.location.x, v.dxf.location.y) for v in entity.vertices]
        return pts, closed
    if dxftype == "LINE":
        s = entity.dxf.start
        e = entity.dxf.end
        return [(s.x, s.y), (e.x, e.y)], False
    raise TypeError(dxftype)


def as_bulge_vertices(points: list[tuple[float, float]]) -> list[Vertex]:
    return [(x, y, 0.0) for x, y in points]


def collect_simplified(
    doc: ezdxf.document.Drawing, tolerance: float
) -> tuple[list[Contour], int, int, int, int]:
    """
    Zwraca: kontury, wierzchołki przed, wierzchołki po, liczba SPLINE, segmenty łukowe.
    """
    msp = doc.modelspace()
    sources = list(msp.query("POLYLINE LWPOLYLINE LINE"))
    splines = list(msp.query("SPLINE"))
    originals = 0
    simplified = 0
    arc_segments = 0
    result: list[Contour] = []

    for entity in sources:
        try:
            pts, closed = polyline_points_2d(entity)
        except TypeError:
            continue
        layer = entity.dxf.layer
        pts = dedupe_consecutive(pts)
        originals += len(pts)
        simp = rdp_simplify(pts, tolerance)
        if closed and len(simp) > 2:
            if math.hypot(simp[0][0] - simp[-1][0], simp[0][1] - simp[-1][1]) < 1e-6:
                simp = simp[:-1]
        simplified += len(simp)
        if len(simp) >= 2:
            verts = as_bulge_vertices(simp)
            result.append((verts, closed, layer))

    for entity in splines:
        layer = entity.dxf.layer
        try:
            verts, closed = spline_to_vertices(entity, tolerance)
        except Exception:
            continue
        # Przybliżona liczba „przed”: próbki spłaszczenia przy tej samej tolerancji
        try:
            from ezdxf import path as ezpath

            flat = list(ezpath.make_path(entity).flattening(distance=max(tolerance, 0.01)))
            originals += max(len(flat), len(verts))
        except Exception:
            originals += len(verts)
        simplified += len(verts)
        arc_segments += count_arc_segments(verts)
        if len(verts) >= 2:
            result.append((verts, closed, layer))

    return result, originals, simplified, len(splines), arc_segments


def _add_line_or_arc(msp, start: Vertex, end: Vertex, *, layer: str) -> None:
    """TruTops rozpoznaje tylko LINE i ARC — nie LWPOLYLINE."""
    x0, y0, bulge = start
    x1, y1, _ = end
    attribs = {"layer": layer}
    if abs(bulge) < 1e-6:
        msp.add_line((x0, y0), (x1, y1), dxfattribs=attribs)
        return
    center, start_angle, end_angle, radius = bulge_to_arc((x0, y0), (x1, y1), bulge)
    msp.add_arc(
        center=(float(center.x), float(center.y)),
        radius=abs(float(radius)),
        start_angle=math.degrees(float(start_angle)),
        end_angle=math.degrees(float(end_angle)),
        dxfattribs=attribs,
    )


def _write_contours_as_lines_arcs(msp, polylines: list[Contour]) -> None:
    for pts, closed, layer in polylines:
        if len(pts) < 2:
            continue
        n = len(pts)
        for i in range(n - 1):
            _add_line_or_arc(msp, pts[i], pts[i + 1], layer=layer)
        if closed and n >= 2:
            _add_line_or_arc(msp, pts[-1], pts[0], layer=layer)


def write_r2000(polylines: list[Contour], path: Path) -> ezdxf.document.Drawing:
    """R2000 dla TruTops: wyłącznie LINE + ARC."""
    doc = ezdxf.new("R2000")
    _write_contours_as_lines_arcs(doc.modelspace(), polylines)
    doc.saveas(path)
    return doc


def write_r12(polylines: list[Contour], path: Path) -> ezdxf.document.Drawing:
    """R12 dla TruTops: wyłącznie LINE + ARC."""
    doc = ezdxf.new("R12")
    _write_contours_as_lines_arcs(doc.modelspace(), polylines)
    doc.saveas(path)
    return doc


def make_timestamped_output_path(input_path: Path) -> Path:
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    return input_path.with_name(f"{input_path.stem}_{stamp}.dxf")


def convert_file(input_path: Path, tolerance: float, version: str, output_path: Path | None = None) -> dict:
    out = output_path or make_timestamped_output_path(input_path)
    src = ezdxf.readfile(input_path)
    polylines, before, after, spline_count, arc_segments = collect_simplified(src, tolerance)
    if not polylines:
        raise ValueError("Brak geometrii POLYLINE/LWPOLYLINE/LINE/SPLINE.")

    if version == "R12":
        doc = write_r12(polylines, out)
    else:
        doc = write_r2000(polylines, out)

    extents_text = None
    try:
        box = bbox.extents(doc.modelspace())
        if box.has_data:
            extents_text = f"{box.extmin} — {box.extmax}"
    except Exception:
        pass

    return {
        "input": input_path,
        "output": out,
        "before": before,
        "after": after,
        "contours": len(polylines),
        "splines": spline_count,
        "arc_segments": arc_segments,
        "extents": extents_text,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Upraszcza DXF pod TruTops — RDP na poliliniach, SPLINE→łuki+proste."
    )
    parser.add_argument("inputs", nargs="+", type=Path, help="Jeden lub wiele plików DXF wejściowych")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="Plik DXF wyjściowy (dla 1 pliku; domyślnie: <nazwa>_<timestamp>.dxf)",
    )
    parser.add_argument(
        "-t",
        "--tolerance",
        type=float,
        default=0.1,
        help="Tolerancja odchylenia w jednostkach rysunku (mm), domyślnie 0.1",
    )
    parser.add_argument(
        "--version",
        default="R2000",
        choices=["R12", "R2000"],
        help="Wersja DXF wyjściowa (R2000 zalecane dla TruTops)",
    )
    args = parser.parse_args()

    if args.output and len(args.inputs) > 1:
        print("Błąd: --output można użyć tylko dla jednego pliku wejściowego.", file=sys.stderr)
        return 1

    failures = 0
    for index, input_path in enumerate(args.inputs):
        if not input_path.is_file():
            print(f"Błąd: nie znaleziono pliku {input_path}", file=sys.stderr)
            failures += 1
            continue

        output_path = args.output if index == 0 and len(args.inputs) == 1 else None
        print(f"Wczytywanie: {input_path}")
        try:
            result = convert_file(
                input_path=input_path,
                tolerance=args.tolerance,
                version=args.version,
                output_path=output_path,
            )
        except Exception as exc:
            print(f"Błąd konwersji {input_path}: {exc}", file=sys.stderr)
            failures += 1
            continue

        print(
            f"Wierzchołki: {result['before']} -> {result['after']} "
            f"(tolerancja {args.tolerance})"
        )
        print(f"Kontury: {result['contours']}")
        if result["splines"]:
            print(
                f"SPLINE: {result['splines']} -> segmenty lukowe: {result['arc_segments']}"
            )
        if result["extents"]:
            print(f"Zakres: {result['extents']}")
        print(f"Zapisano DXF: {result['output']}")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
