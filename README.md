# Czyste linie DXF dla TruTops

Pliki DXF z Etsy często zawierają **setki wierzchołków na każdej krawędzi** (aproksymacja krzywych jako POLYLINE). AutoCAD wyświetla je jako gładkie linie, ale **TruTops pokazuje każdy wierzchołek** (zielone kwadraciki).

To narzędzie usuwa nadmiarowe punkty algorytmem Ramer–Douglas–Peucker i zapisuje uproszczone **LWPOLYLINE**.

## Szybki start (CLI)

```powershell
pip install -r requirements.txt
python simplify_dxf.py panel_nr10.dxf
```

Wynik w folderze źródła (para plików):
- `<nazwa_pliku>_<timestamp>.dxf` — do TruTops
- `<nazwa_pliku>_<timestamp>.pdf` — podgląd (orientacyjny, do załączników)

Przykład: `panel_nr10_20260527_115224.dxf` + `panel_nr10_20260527_115224.pdf`

Możesz konwertować wiele plików naraz:

```powershell
python simplify_dxf.py panel1.dxf panel2.dxf panel3.dxf -t 0.1
```

### Parametry

| Opcja | Opis |
|--------|------|
| `-o plik.dxf` | Ścieżka wyjściowa (tylko dla 1 pliku wejściowego) |
| `-t 0.1` | Tolerancja w mm (domyślnie 0.1) |
| `--version R2000` | Format DXF (R2000 zalecany) |

Przykład dla większej dokładności na łukach:

`python simplify_dxf.py panel_nr10.dxf -t 0.05`

Dla pliku `panel_nr10.dxf`: **~136 000 → ~1 600 wierzchołków** (tolerancja 0.1 mm).

## GUI (wiele plików)

Uruchom aplikację okienkową:

```powershell
python dxf_gui.py
```

Funkcje GUI:
- wybór wielu plików DXF,
- suwak tolerancji,
- wybór formatu `R2000` / `R12`,
- automatyczny zapis DXF + PDF obok źródła (`<nazwa>_<timestamp>.dxf` / `.pdf`).

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

## Dlaczego TruTops pokazuje więcej punktów?

| Program | Zachowanie |
|---------|------------|
| AutoCAD | Rysuje gęstą polilinię jako gładką linię; wierzchołki widać po zaznaczeniu |
| TruTops | Traktuje **każdy wierzchołek** jako węzeł ścieżki cięcia |

Plik z Etsy ma typ `POLYLINE` z setkami `VERTEX` na prostych i łukach. Po uproszczeniu zostają tylko załomienia (jak na zrzucie `autocad.png`).

## Pliki w repozytorium

- `simplify_dxf.py` — główne narzędzie (bez AutoCAD)
- `dxf_gui.py` — aplikacja okienkowa (wiele plików)
- `build_exe.bat` — build standalone EXE
- `TRUTOPS_CLEAN.lsp` — to samo w AutoCAD
- `panel_nr10.dxf` — przykład wejściowy
- `panel_nr10_zapisany_w_trutopsie.dxf` — jak TruTops zapisuje (osobne LINE)
