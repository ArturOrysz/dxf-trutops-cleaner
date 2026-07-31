# Czyste linie DXF dla TruTops

Pliki DXF z Etsy często zawierają **setki wierzchołków na każdej krawędzi** (aproksymacja krzywych jako POLYLINE). AutoCAD wyświetla je jako gładkie linie, ale **TruTops pokazuje każdy wierzchołek** (zielone kwadraciki).

To narzędzie:
- usuwa nadmiarowe punkty algorytmem Ramer–Douglas–Peucker na gęstych `POLYLINE` / `LWPOLYLINE`,
- konwertuje krzywe Béziera (`SPLINE`) na segmenty **proste i łukowe**,
- zapisuje wynik jako **wyłącznie `LINE` + `ARC`** (wymóg TruTops — program nie czyta polilinii).

## Szybki start (CLI)

```powershell
pip install -r requirements.txt
python simplify_dxf.py panel_nr10.dxf
```

Wynik w folderze źródła:
- `<nazwa_pliku>_<timestamp>.dxf` — do TruTops

Przykład: `panel_nr10_20260527_115224.dxf`

Możesz konwertować wiele plików naraz:

```powershell
python simplify_dxf.py panel1.dxf panel2.dxf panel3.dxf -t 0.1
```

### Parametry

| Opcja | Opis |
|--------|------|
| `-o plik.dxf` | Ścieżka wyjściowa (tylko dla 1 pliku wejściowego) |
| `-t 0.1` | Tolerancja w mm (domyślnie 0.1) — też jakość dopasowania łuków do SPLINE |
| `--version R2000` | Format DXF (R2000 zalecany) |

Przykład dla większej dokładności na łukach:

`python simplify_dxf.py panel_nr10.dxf -t 0.05`

Dla pliku `panel_nr10.dxf`: **~136 000 → ~1 600 wierzchołków** (tolerancja 0.1 mm).

## Krzywe Béziera / SPLINE

Gdy plik źródłowy zawiera encje `SPLINE` (np. eksport z CAD jako Bézier), cleaner automatycznie zamienia je na odcinki proste i łukowe w tolerancji `-t`, a potem zapisuje jako osobne encje **`LINE`** i **`ARC`**.

W AutoCAD możesz sprawdzić wynik poleceniem `LIST` — powinny pojawić się encje `LINE` / `ARC` (nie `LWPOLYLINE` / `SPLINE`).

**Ważne:** TruTops widzi tylko `LINE` i `ARC`. Dlatego wyjście cleanera nie używa polilinii z `bulge`.

## GUI (wiele plików)

Uruchom aplikację okienkową:

```powershell
python dxf_gui.py
```

Funkcje GUI:
- wybór wielu plików DXF,
- suwak tolerancji,
- wybór formatu `R2000` / `R12`,
- automatyczny zapis DXF obok źródła (`<nazwa>_<timestamp>.dxf`).

## Pobierz EXE (bez budowania)

Gotowy program Windows jest w [Releases](https://github.com/ArturOrysz/dxf-trutops-cleaner/releases):

- **dxf_trutops_cleaner.exe** — uruchom bez instalacji Pythona

## Standalone EXE (Windows)

Zbuduj jednoplikowy program lokalnie:

```powershell
build_exe.bat
```

Wynik:
- `dist\dxf_trutops_cleaner.exe`

Plik EXE można uruchomić na Windows bez ręcznego startu Pythona.

## AutoCAD / AutoCAD LT

> **AutoCAD LT 2024 i nowsze** obsługują AutoLISP. Starsze wersje LT — użyj skryptu Python.

1. Otwórz DXF w AutoCAD.
2. W linii poleceń: `(load "ścieżka\\TRUTOPS_CLEAN.lsp")`
3. Komenda: `TRUTOPS-CLEAN` — podaj tolerancję (Enter = 0.1 mm).
4. Komenda: `TRUTOPS-EXPORT` — zapisz DXF.

Skrypt LISP upraszcza tylko polilinie (RDP). Konwersja SPLINE → łuki jest w narzędziu Python / GUI.

## Dlaczego TruTops pokazuje więcej punktów?

| Program | Zachowanie |
|---------|------------|
| AutoCAD | Rysuje gęstą polilinię jako gładką linię; wierzchołki widać po zaznaczeniu |
| TruTops | Traktuje **każdy wierzchołek** jako węzeł ścieżki cięcia |

Plik z Etsy ma typ `POLYLINE` z setkami `VERTEX` na prostych i łukach. Po uproszczeniu zostają tylko załomienia zapisane jako `LINE`, a krzywe jako `ARC` (jak wymaga TruTops).

## Pliki w repozytorium

- `simplify_dxf.py` — główne narzędzie (bez AutoCAD)
- `arc_fit.py` — SPLINE / Bézier → segmenty proste i łukowe
- `dxf_gui.py` — aplikacja okienkowa (wiele plików)
- `build_exe.bat` — build standalone EXE
- `TRUTOPS_CLEAN.lsp` — RDP polilinii w AutoCAD
- `panel_nr10.dxf` — przykład wejściowy
- `panel_nr10_zapisany_w_trutopsie.dxf` — jak TruTops zapisuje (osobne LINE)
