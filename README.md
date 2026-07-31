# Czyste linie DXF dla TruTops

Pliki DXF z Etsy / CAD często zawierają **setki wierzchołków** (gęste `POLYLINE`) albo krzywe **`SPLINE`** (Bézier). AutoCAD rysuje je gładko, ale **TruTops pokazuje każdy wierzchołek** i **nie importuje polilinii / splajnów** — potrzebuje wyłącznie **`LINE` + `ARC`**.

W repozytorium są **dwie niezależne aplikacje** o tej samej funkcji:

| Wersja | Katalog | Opis |
|--------|---------|------|
| **Python** | root (`simplify_dxf.py`, `dxf_gui.py`) | CLI + GUI (Tkinter), EXE przez PyInstaller |
| **Rust** (zalecana) | [`rust-app/`](rust-app/README.md) | Natywne GUI (egui), mniejszy EXE, bez Pythona |

---

## Problem, który rozwiązujemy

1. **Za dużo punktów** — TruTops traktuje każdy wierzchołek polilinii jako węzeł ścieżki cięcia.
2. **TruTops widzi tylko `LINE` i `ARC`** — `LWPOLYLINE`, `POLYLINE` i `SPLINE` znikają lub są bezużyteczne.
3. **Canvas „pusty” w TruTops** — nawet gdy AutoCAD otwiera plik poprawnie, prawie-płaskie `ARC` o **ogromnym promieniu** (środek tysiące mm od rysunku) rozciągają widok; części wyglądają jakby zniknęły. Cleaner zamienia takie łuki na `LINE` i zapisuje **R2000 (AC1015)** z pełnymi sekcjami DXF.

---

## Pobierz EXE (Windows)

Gotowe buildy: [GitHub Releases](https://github.com/ArturOrysz/dxf-trutops-cleaner/releases)

- **`dxf_trutops_cleaner_rs.exe`** — wersja Rust (zalecana)
- **`dxf_trutops_cleaner.exe`** — wersja Python

---

## Aplikacja Rust (zalecana)

Pełna dokumentacja: **[`rust-app/README.md`](rust-app/README.md)**

```powershell
cd rust-app
cargo run --release
# albo CLI:
cargo run --release -- ".\plik.dxf" -t 0.1
```

Wynik: `<nazwa>_<timestamp>.dxf` obok pliku źródłowego (tylko `LINE` + `ARC`, R2000).

---

## Aplikacja Python

### CLI

```powershell
pip install -r requirements.txt
python simplify_dxf.py panel_nr10.dxf
```

| Opcja | Opis |
|--------|------|
| `-o plik.dxf` | Ścieżka wyjściowa (tylko 1 plik) |
| `-t 0.1` | Tolerancja w mm (domyślnie 0.1) |
| `--version R2000` | Format DXF (`R2000` zalecany dla TruTops) |

### GUI

```powershell
python dxf_gui.py
```

### Build EXE (Python)

```powershell
build_exe.bat
```

Wynik: `dist\dxf_trutops_cleaner.exe`

---

## AutoCAD / AutoCAD LT (tylko polilinie)

> AutoCAD LT 2024+ obsługuje AutoLISP. Starsze LT — użyj Python lub Rust.

1. `(load "ścieżka\\TRUTOPS_CLEAN.lsp")`
2. `TRUTOPS-CLEAN` — tolerancja (Enter = 0.1 mm)
3. `TRUTOPS-EXPORT` — zapis DXF

LISP robi tylko RDP na poliliniach. **SPLINE → ARC** jest w Python / Rust.

---

## Pliki w repozytorium

| Plik / katalog | Rola |
|----------------|------|
| `rust-app/` | Aplikacja Rust (GUI + CLI) |
| `simplify_dxf.py` | Konwersja Python (CLI) |
| `arc_fit.py` | SPLINE / Bézier → łuki (Python) |
| `dxf_gui.py` | GUI Python |
| `build_exe.bat` | Build EXE Python |
| `TRUTOPS_CLEAN.lsp` | RDP w AutoCAD |
| `requirements.txt` | Zależności Python |
