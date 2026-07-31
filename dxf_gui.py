#!/usr/bin/env python3
from __future__ import annotations

import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk

from simplify_dxf import convert_file


class App(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title("DXF Cleaner dla TruTops")
        self.geometry("860x620")
        self.minsize(760, 520)

        self.selected_files: list[Path] = []
        self.tolerance_var = tk.DoubleVar(value=0.1)
        self.version_var = tk.StringVar(value="R2000")

        self._build_ui()

    def _build_ui(self) -> None:
        root = ttk.Frame(self, padding=16)
        root.pack(fill=tk.BOTH, expand=True)

        top = ttk.Frame(root)
        top.pack(fill=tk.X)

        ttk.Button(top, text="Wybierz pliki DXF...", command=self.pick_files).pack(side=tk.LEFT)
        ttk.Button(top, text="Wyczyść listę", command=self.clear_files).pack(side=tk.LEFT, padx=(10, 0))

        settings = ttk.LabelFrame(root, text="Ustawienia", padding=12)
        settings.pack(fill=tk.X, pady=(14, 10))

        ttk.Label(settings, text="Tolerancja odchylenia (mm)").grid(row=0, column=0, sticky="w")
        self.tolerance_label = ttk.Label(settings, text="0.10")
        self.tolerance_label.grid(row=0, column=1, sticky="w", padx=(10, 0))

        slider = ttk.Scale(
            settings,
            from_=0.01,
            to=2.0,
            orient=tk.HORIZONTAL,
            variable=self.tolerance_var,
            command=self.on_tolerance_change,
        )
        slider.grid(row=1, column=0, columnspan=3, sticky="ew", pady=(8, 4))
        settings.columnconfigure(0, weight=1)

        ttk.Label(settings, text="Format DXF").grid(row=2, column=0, sticky="w", pady=(10, 0))
        ttk.Combobox(
            settings,
            textvariable=self.version_var,
            values=["R2000", "R12"],
            state="readonly",
            width=10,
        ).grid(row=2, column=1, sticky="w", padx=(10, 0), pady=(10, 0))

        ttk.Label(
            settings,
            text="Wynik w folderze źródła: <nazwa>_<timestamp>.dxf",
        ).grid(row=3, column=0, columnspan=3, sticky="w", pady=(10, 0))

        list_frame = ttk.LabelFrame(root, text="Pliki do konwersji", padding=8)
        list_frame.pack(fill=tk.BOTH, expand=True, pady=(6, 10))

        self.files_list = tk.Listbox(list_frame, height=10)
        self.files_list.pack(fill=tk.BOTH, expand=True, side=tk.LEFT)
        scrollbar = ttk.Scrollbar(list_frame, orient=tk.VERTICAL, command=self.files_list.yview)
        scrollbar.pack(fill=tk.Y, side=tk.RIGHT)
        self.files_list.configure(yscrollcommand=scrollbar.set)

        actions = ttk.Frame(root)
        actions.pack(fill=tk.X)
        self.run_button = ttk.Button(actions, text="Uprość DXF", command=self.run_conversion)
        self.run_button.pack(fill=tk.X)

        log_frame = ttk.LabelFrame(root, text="Log", padding=8)
        log_frame.pack(fill=tk.BOTH, expand=True, pady=(10, 0))
        self.log = tk.Text(log_frame, height=10, wrap="word", state=tk.DISABLED)
        self.log.pack(fill=tk.BOTH, expand=True)

    def on_tolerance_change(self, _: str) -> None:
        self.tolerance_label.config(text=f"{self.tolerance_var.get():.2f}")

    def pick_files(self) -> None:
        paths = filedialog.askopenfilenames(
            title="Wybierz pliki DXF",
            filetypes=[("DXF files", "*.dxf"), ("Wszystkie pliki", "*.*")],
        )
        if not paths:
            return
        for item in paths:
            p = Path(item)
            if p not in self.selected_files:
                self.selected_files.append(p)
                self.files_list.insert(tk.END, str(p))

    def clear_files(self) -> None:
        self.selected_files = []
        self.files_list.delete(0, tk.END)

    def append_log(self, text: str) -> None:
        self.log.configure(state=tk.NORMAL)
        self.log.insert(tk.END, text + "\n")
        self.log.see(tk.END)
        self.log.configure(state=tk.DISABLED)

    def run_conversion(self) -> None:
        if not self.selected_files:
            messagebox.showwarning("Brak plików", "Wybierz co najmniej jeden plik DXF.")
            return

        self.run_button.configure(state=tk.DISABLED)
        self.append_log("-" * 70)
        self.append_log(
            f"Start: plików={len(self.selected_files)}, tol={self.tolerance_var.get():.2f}, "
            f"format={self.version_var.get()}"
        )

        worker = threading.Thread(target=self._convert_worker, daemon=True)
        worker.start()

    def _convert_worker(self) -> None:
        failures = 0
        for path in self.selected_files:
            self._threadsafe_log(f"Wczytywanie: {path}")
            try:
                result = convert_file(
                    input_path=path,
                    tolerance=float(self.tolerance_var.get()),
                    version=self.version_var.get(),
                    output_path=None,
                )
                self._threadsafe_log(
                    f"OK: {result['before']} -> {result['after']} | kontury: {result['contours']}"
                )
                if result.get("splines"):
                    self._threadsafe_log(
                        f"SPLINE: {result['splines']} -> segmenty lukowe: {result['arc_segments']}"
                    )
                self._threadsafe_log(f"Zapisano DXF: {result['output']}")
            except Exception as exc:
                failures += 1
                self._threadsafe_log(f"BLAD: {path} | {exc}")

        self.after(0, self._finish_run, failures)

    def _threadsafe_log(self, text: str) -> None:
        self.after(0, self.append_log, text)

    def _finish_run(self, failures: int) -> None:
        self.run_button.configure(state=tk.NORMAL)
        if failures == 0:
            messagebox.showinfo("Gotowe", "Konwersja zakonczona pomyslnie.")
        else:
            messagebox.showwarning(
                "Konwersja zakonczona",
                f"Zakonczono z bledami. Niepowodzenia: {failures}.",
            )


def main() -> None:
    app = App()
    app.mainloop()


if __name__ == "__main__":
    main()
