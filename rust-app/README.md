# DXF TruTops Cleaner — wersja Rust

Natywna aplikacja Windows (**Rust + egui**). Działa niezależnie od wersji Python w katalogu głównym repozytorium.

**Wersja dokumentacji:** 0.2 / release repo **v1.2.x**

---

## Do czego służy

Przygotowuje DXF pod **Trumpf TruTops**:

| Wejście | Wyjście |
|---------|---------|
| Gęste `POLYLINE` / `LWPOLYLINE` | Uproszczenie RDP → odcinki `LINE` |
| `LINE` | Bez zmian (jako `LINE`) |
| `SPLINE` (Bézier / B-spline) | Dopasowanie łuków → `LINE` + `ARC` |
| **`SVG`** (path / koła / linie / …) | Bézier i kształty → `LINE` + `ARC` |

> SVG **nie** jest „tylko Bézier”: w ścieżkach są też linie, kwadraty Béziera, łuki eliptyczne; osobno `circle` / `rect` / `line` / `polyline`. Silnik `usvg` sprowadza to do linii + krzywych, a my do **`LINE` + `ARC`**.

Wyjście zawiera **wyłącznie** encje `LINE` i `ARC` w formacie **DXF R2000 (AC1015)**.

---

## Wymagania TruTops (ważne)

1. **Tylko `LINE` + `ARC`** — TruTops nie korzysta z polilinii ani splajnów jak AutoCAD.
2. **Format R2000** — pełne sekcje `HEADER` / `TABLES` / `BLOCKS` / `ENTITIES` / `OBJECTS` (szkielet jak w typowym eksporcie CAD). Sam „minimalny” DXF bywa odrzucany albo źle interpretowany.
3. **Problem „znikających” części na canvasie**  
   Prawie-płaskie `ARC` mają środek daleko poza rysunkiem (promień tysiące mm). TruTops ustawia widok pod te środki i geometria **znika z ekranu**, mimo że AutoCAD otwiera plik poprawnie.  
   Ta aplikacja **zamienia takie łuki na `LINE`** (kryteria: mały kąt środkowy, zbyt duży promień względem cięciwy).
4. **Extrusion Z-up** (`210/220/230 = 0,0,1`) — ogranicza problemy importerów Trumpf z orientacją łuków.

Po imporcie w TruTops warto zrobić **Zoom Extents** / dopasowanie widoku.

---

## Instalacja / build

### Wymagania deweloperskie

- [Rustup](https://rustup.rs/) — toolchain `stable`
- Windows 10/11 (MSVC)

### Uruchomienie z źródeł

GUI jest minimalne: **Wybierz DXF…** → **Konwertuj** → log.  
Tolerancja w GUI jest stała (`0.1` mm); inne wartości przez CLI (`-t`).

```powershell
cd rust-app
cargo run --release
```

### Build EXE

```powershell
cd rust-app
cargo build --release
```

Binary:

- `rust-app\target\release\dxf_trutops_cleaner_rs.exe`

(albo kopia w Releases / `dist\dxf_trutops_cleaner_rs.exe`)

---

## GUI

1. Uruchom EXE lub `cargo run --release`.
2. **Wybierz pliki DXF…**
3. Ustaw **tolerancję** (mm) — domyślnie `0.1`.
4. **Uprość DXF**.

Wynik trafia obok źródła: `<nazwa>_<yyyyMMdd_HHmmss>.dxf`.

---

## CLI

```powershell
cd rust-app
cargo run --release -- "C:\sciezka\plik.dxf" -t 0.1
```

| Argument | Opis |
|----------|------|
| `plik.dxf` | Jeden lub więcej plików wejściowych |
| `-t` / `--tolerance` | Tolerancja odchylenia w mm (domyślnie `0.1`) |
| `-o` / `--output` | Ścieżka wyjściowa (tylko przy jednym pliku wejściowym) |

Bez argumentów startuje **GUI**.

Przykład:

```powershell
.\dxf_trutops_cleaner_rs.exe ".\panel.dxf" -t 0.05 -o ".\panel_clean.dxf"
```

---

## Parametr tolerancji

| Wartość | Skutek |
|---------|--------|
| mniejsza (np. `0.05`) | Więcej segmentów, dokładniejsze łuki |
| większa (np. `0.2`–`0.5`) | Mniej geometrii, mocniejsze uproszczenie |

Ta sama wartość steruje RDP na poliliniach i dopasowaniem łuków do `SPLINE`.

---

## Struktura kodu

```
rust-app/
  assets/
    r2000_empty.dxf    # szkielet R2000 (TABLES/BLOCKS/OBJECTS)
  src/
    main.rs            # CLI + start GUI
    gui.rs             # egui
    convert.rs         # pipeline konwersji (DXF + SVG)
    dxf_read.rs        # odczyt LINE/POLYLINE/LWPOLYLINE/SPLINE
    svg_read.rs        # odczyt SVG (usvg → Bézier/linie → łuki)
    dxf_write.rs       # zapis R2000 LINE+ARC (+ filtr prawie-płaskich ARC)
    geom.rs            # RDP, bulge, B-spline, dopasowanie łuków
  Cargo.toml
  README.md            # ta dokumentacja
```

---

## Weryfikacja wyniku

**AutoCAD**

1. Otwórz wygenerowany DXF.
2. `LIST` na obiekcie — typ `LINE` lub `ARC` (bez `SPLINE` / `LWPOLYLINE`).

**TruTops**

1. Import DXF.
2. Jeśli widok pusty — Zoom Extents.
3. Sprawdź, czy kontury są na canvasie w skali rysunku (nie „pyłek” w środku ogromnego obszaru).

**Szybki test jednostkowy (łuk Béziera)**

```powershell
cd rust-app
cargo test
```

---

## Rozwiązywanie problemów

| Objaw | Co zrobić |
|-------|-----------|
| AutoCAD nie otwiera pliku | Użyj bieżącego builda Rust (R2000 ze szkieletem). Stary minimalny DXF bez `TABLES` był odrzucany. |
| TruTops: puste / „zniknięte” części | Zoom Extents; upewnij się, że używasz EXE po poprawce filtrującej wielkie ARC. |
| Brak łuków, same linie | Zmniejsz `-t` (np. `0.05`). |
| Brak geometrii w ogóle | Wejście musi mieć `LINE` / `POLYLINE` / `LWPOLYLINE` / `SPLINE` w model space. |

---

## Licencja / powiązanie

Ten katalog jest częścią repozytorium [dxf-trutops-cleaner](https://github.com/ArturOrysz/dxf-trutops-cleaner). Wersja Python pozostaje w katalogu głównym i nie jest wymagana do działania aplikacji Rust.
