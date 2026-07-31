"""
Konwersja SPLINE (krzywe Béziera / B-spline) na segmenty proste i łukowe (bulge).

Wyjście: lista wierzchołków (x, y, bulge), gdzie bulge dotyczy odcinka
do następnego wierzchołka (format LWPOLYLINE / DXF).
"""

from __future__ import annotations

import math
from typing import Sequence

from ezdxf.math import Bezier4P, Vec3, bulge_3_points, bulge_to_arc

Vertex = tuple[float, float, float]  # x, y, bulge
Point2 = tuple[float, float]

_MAX_DEPTH = 12
_SAMPLE_COUNT = 16
_STRAIGHT_BULGE = 1e-6


def _xy(p: Vec3) -> Point2:
    return (float(p.x), float(p.y))


def _bezier_point(ctrl: Sequence[Vec3], t: float) -> Vec3:
    return Bezier4P(ctrl).point(t)


def _split_bezier(ctrl: Sequence[Vec3], t: float = 0.5) -> tuple[list[Vec3], list[Vec3]]:
    """De Casteljau — podział cubic Bézier w parametrze t."""
    p0, p1, p2, p3 = ctrl
    q0 = p0.lerp(p1, t)
    q1 = p1.lerp(p2, t)
    q2 = p2.lerp(p3, t)
    r0 = q0.lerp(q1, t)
    r1 = q1.lerp(q2, t)
    s = r0.lerp(r1, t)
    return [p0, q0, r0, s], [s, r1, q2, p3]


def _point_to_arc_distance(p: Point2, start: Point2, end: Point2, bulge: float) -> float:
    """Odległość punktu od łuku (lub odcinka gdy bulge≈0)."""
    if abs(bulge) < _STRAIGHT_BULGE:
        ax, ay = start
        bx, by = end
        px, py = p
        dx, dy = bx - ax, by - ay
        len_sq = dx * dx + dy * dy
        if len_sq == 0:
            return math.hypot(px - ax, py - ay)
        t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / len_sq))
        return math.hypot(px - (ax + t * dx), py - (ay + t * dy))

    center, start_angle, end_angle, radius = bulge_to_arc(start, end, bulge)
    cx, cy = float(center.x), float(center.y)
    px, py = p
    dist_center = math.hypot(px - cx, py - cy)
    return abs(dist_center - abs(radius))


def _fit_error(ctrl: Sequence[Vec3], bulge: float) -> float:
    start = _xy(ctrl[0])
    end = _xy(ctrl[3])
    curve = Bezier4P(ctrl)
    max_err = 0.0
    for i in range(1, _SAMPLE_COUNT):
        t = i / _SAMPLE_COUNT
        pt = _xy(curve.point(t))
        err = _point_to_arc_distance(pt, start, end, bulge)
        if err > max_err:
            max_err = err
    return max_err


def _chord_length(ctrl: Sequence[Vec3]) -> float:
    a, b = ctrl[0], ctrl[3]
    return math.hypot(b.x - a.x, b.y - a.y)


def _approximate_bezier(
    ctrl: Sequence[Vec3],
    tolerance: float,
    depth: int = 0,
) -> list[Vertex]:
    """
    Rekurencyjnie dopasuj cubic Bézier do łuku/prostej.
    Zwraca wierzchołki z bulge na każdym segmencie oprócz ostatniego
    punktu końcowego (bulge=0 na końcu — ustawiane przy składaniu).
    """
    start = ctrl[0]
    end = ctrl[3]
    chord = _chord_length(ctrl)
    if chord < 1e-12:
        return [(_xy(start)[0], _xy(start)[1], 0.0)]

    mid = _xy(_bezier_point(ctrl, 0.5))
    try:
        bulge = float(bulge_3_points(_xy(start), _xy(end), mid))
    except (ValueError, ZeroDivisionError):
        bulge = 0.0

    if abs(bulge) < _STRAIGHT_BULGE:
        bulge = 0.0

    # Unikaj półokręgów+ z jednym segmentem gdy błąd duży — |bulge| > ~1 to >180°
    error = _fit_error(ctrl, bulge) if abs(bulge) <= 1.0 else tolerance + 1.0

    if error <= tolerance or depth >= _MAX_DEPTH:
        sx, sy = _xy(start)
        ex, ey = _xy(end)
        return [(sx, sy, bulge), (ex, ey, 0.0)]

    left, right = _split_bezier(ctrl, 0.5)
    left_verts = _approximate_bezier(left, tolerance, depth + 1)
    right_verts = _approximate_bezier(right, tolerance, depth + 1)
    # Połącz bez duplikatu punktu w środku
    return left_verts[:-1] + right_verts


def _as_ctrl4(curve) -> list[Vec3] | None:
    """Normalizuj wynik dekompozycji / aproksymacji do 4 punktów kontrolnych."""
    if isinstance(curve, Bezier4P):
        ctrl = list(curve.control_points)
    elif isinstance(curve, (list, tuple)) and len(curve) == 4:
        ctrl = list(curve)
    else:
        return None
    return [Vec3(p.x, p.y, 0.0) for p in ctrl]


def _bezier_curves_from_spline(spline_entity) -> list[list[Vec3]]:
    bspline = spline_entity.construction_tool()
    raw: list = []
    try:
        raw = list(bspline.bezier_decomposition())
    except Exception:
        raw = []
    if not raw:
        try:
            raw = list(bspline.cubic_bezier_approximation(level=3))
        except Exception:
            return []
    out: list[list[Vec3]] = []
    for item in raw:
        ctrl = _as_ctrl4(item)
        if ctrl is not None:
            out.append(ctrl)
    return out


def _merge_vertices(segments: list[list[Vertex]], eps: float = 1e-9) -> list[Vertex]:
    if not segments:
        return []
    out: list[Vertex] = []
    for seg in segments:
        if not seg:
            continue
        if not out:
            out.extend(seg)
            continue
        # Odrzuć pierwszy punkt segmentu jeśli pokrywa się z końcem out
        first = seg[0]
        last = out[-1]
        if math.hypot(first[0] - last[0], first[1] - last[1]) <= eps:
            # Zachowaj bulge z końca poprzedniego (już ustawiony) — zamień ostatni
            # na wierzchołek z bulge pierwszego segmentu, potem dodaj resztę
            out[-1] = (last[0], last[1], first[2])
            out.extend(seg[1:])
        else:
            out.extend(seg)
    return out


def spline_to_vertices(
    spline_entity,
    tolerance: float,
) -> tuple[list[Vertex], bool]:
    """
    Konwertuj encję SPLINE na listę wierzchołków (x, y, bulge) + flaga closed.

    Ostatni wierzchołek ma bulge=0 (niezastosowany); przy closed=True
    bulge ostatniego wierzchołka dotyczy łuku zamykającego do pierwszego —
    tu SPLINE zwykle open, więc zostawiamy 0.
    """
    closed = bool(getattr(spline_entity, "closed", False))
    curves = _bezier_curves_from_spline(spline_entity)
    if not curves:
        return [], closed

    segments: list[list[Vertex]] = []
    for ctrl2 in curves:
        verts = _approximate_bezier(ctrl2, tolerance)
        if len(verts) >= 2:
            segments.append(verts)

    merged = _merge_vertices(segments)
    if len(merged) < 2:
        return [], closed

    # Usuń prawie-zerowe odcinki
    cleaned: list[Vertex] = [merged[0]]
    for v in merged[1:]:
        prev = cleaned[-1]
        if math.hypot(v[0] - prev[0], v[1] - prev[1]) > 1e-9:
            cleaned.append(v)
        else:
            # Zachowaj większy |bulge| jeśli punkty się zbiegają
            if abs(v[2]) > abs(prev[2]):
                cleaned[-1] = (prev[0], prev[1], v[2])

    if closed and len(cleaned) > 2:
        if math.hypot(cleaned[0][0] - cleaned[-1][0], cleaned[0][1] - cleaned[-1][1]) < 1e-6:
            # Przenieś bulge z przedostatniego segmentu; usuń duplikat końca
            cleaned = cleaned[:-1]

    # Ostatni wierzchołek: bulge dotyczy segmentu do następnego — przy open = 0
    if cleaned:
        x, y, _ = cleaned[-1]
        cleaned[-1] = (x, y, 0.0)

    return cleaned, closed


def count_arc_segments(vertices: list[Vertex]) -> int:
    if len(vertices) < 2:
        return 0
    n = 0
    for i, (_x, _y, bulge) in enumerate(vertices):
        if i == len(vertices) - 1:
            break
        if abs(bulge) >= _STRAIGHT_BULGE:
            n += 1
    return n
