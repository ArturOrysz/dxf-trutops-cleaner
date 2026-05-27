#!/usr/bin/env python3
"""
Upraszcza geometrię DXF (gęste POLYLINE z Etsy) do czystych LWPOLYLINE
z wierzchołkami tylko w załomieniach — bez nadmiarowych punktów w TruTops.

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
from ezdxf.addons import r12writer


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


def add_lwpolyline(
    msp,
    points: list[tuple[float, float]],
    *,
    layer: str,
    closed: bool,
) -> None:
    if len(points) < 2:
        return
    pl = msp.add_lwpolyline(
        [(x, y) for x, y in points],
        dxfattribs={"layer": layer},
        close=closed,
    )
    _ = pl


def collect_simplified(
    doc: ezdxf.document.Drawing, tolerance: float
) -> tuple[list[tuple[list[tuple[float, float]], bool, str]], int, int]:
    msp = doc.modelspace()
    sources = list(msp.query("POLYLINE LWPOLYLINE LINE"))
    originals = 0
    simplified = 0
    result: list[tuple[list[tuple[float, float]], bool, str]] = []

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
            result.append((simp, closed, layer))

    return result, originals, simplified


def write_r2000(
    polylines: list[tuple[list[tuple[float, float]], bool, str]], path: Path
) -> ezdxf.document.Drawing:
    doc = ezdxf.new("R2000")
    msp = doc.modelspace()
    for pts, closed, layer in polylines:
        add_lwpolyline(msp, pts, layer=layer, closed=closed)
    doc.saveas(path)
    return doc


def write_r12(
    polylines: list[tuple[list[tuple[float, float]], bool, str]], path: Path
) -> ezdxf.document.Drawing:
    doc = ezdxf.new("R12")
    msp = doc.modelspace()
    for pts, closed, layer in polylines:
        out_pts = list(pts)
        if closed and out_pts and out_pts[0] != out_pts[-1]:
            out_pts.append(out_pts[0])
        r12writer.draw_polyline(msp, out_pts, attribs={"layer": layer})
    doc.saveas(path)
    return doc


def make_timestamped_output_path(input_path: Path) -> Path:
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    return input_path.with_name(f"{input_path.stem}_{stamp}.dxf")


def write_preview_pdf(
    polylines: list[tuple[list[tuple[float, float]], bool, str]],
    path: Path,
    *,
    title: str | None = None,
) -> None:
    """Podgląd PDF (orientacyjny) — do załączników, bez wymagań precyzji."""
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots(figsize=(11.69, 8.27))
    xs_all: list[float] = []
    ys_all: list[float] = []

    for pts, closed, _layer in polylines:
        if len(pts) < 2:
            continue
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        if closed and (xs[0], ys[0]) != (xs[-1], ys[-1]):
            xs.append(xs[0])
            ys.append(ys[0])
        ax.plot(xs, ys, color="#1a1a1a", linewidth=0.9, solid_capstyle="round")
        xs_all.extend(xs)
        ys_all.extend(ys)

    if not xs_all:
        plt.close(fig)
        raise ValueError("Brak geometrii do podglądu PDF.")

    width = max(xs_all) - min(xs_all)
    height = max(ys_all) - min(ys_all)
    margin = max(width, height) * 0.05 or 1.0
    ax.set_xlim(min(xs_all) - margin, max(xs_all) + margin)
    ax.set_ylim(min(ys_all) - margin, max(ys_all) + margin)
    ax.set_aspect("equal", adjustable="box")
    ax.axis("off")
    if title:
        ax.set_title(title, fontsize=10, pad=8)

    fig.savefig(path, format="pdf", bbox_inches="tight", pad_inches=0.15)
    plt.close(fig)


def convert_file(input_path: Path, tolerance: float, version: str, output_path: Path | None = None) -> dict:
    out = output_path or make_timestamped_output_path(input_path)
    src = ezdxf.readfile(input_path)
    polylines, before, after = collect_simplified(src, tolerance)
    if not polylines:
        raise ValueError("Brak geometrii POLYLINE/LWPOLYLINE/LINE.")

    if version == "R12":
        doc = write_r12(polylines, out)
    else:
        doc = write_r2000(polylines, out)

    pdf_out = out.with_suffix(".pdf")
    write_preview_pdf(polylines, pdf_out, title=input_path.name)

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
        "pdf_output": pdf_out,
        "before": before,
        "after": after,
        "contours": len(polylines),
        "extents": extents_text,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Upraszcza DXF pod TruTops — usuwa nadmiarowe wierzchołki na prostych."
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
        if result["extents"]:
            print(f"Zakres: {result['extents']}")
        print(f"Zapisano DXF: {result['output']}")
        print(f"Zapisano PDF: {result['pdf_output']}")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
