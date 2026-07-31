#!/usr/bin/env python3
from __future__ import annotations

import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk

from simplify_dxf import convert_file

DEFAULT_TOLERANCE = 0.1
DEFAULT_VERSION = "R2000"


class App(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title("DXF Cleaner dla TruTops")
        self.geometry("560x420")
        self.minsize(420, 320)

        icon = Path(__file__).with_name("rust-app") / "assets" / "icon.ico"
        if icon.is_file():
            try:
                self.iconbitmap(default=str(icon))
            except tk.TclError:
                pass

        self.selected_files: list[Path] = []
        self._build_ui()

    def _build_ui(self) -> None:
        root = ttk.Frame(self, padding=16)
        root.pack(fill=tk.BOTH, expand=True)

        ttk.Label(root, text="DXF → TruTops", font=("", 14, "bold")).pack(anchor="w")
        ttk.Label(root, text="SPLINE / polilinie → LINE + ARC").pack(anchor="w", pady=(0, 12))

        top = ttk.Frame(root)
        top.pack(fill=tk.X)
        ttk.Button(top, text="Wybierz DXF…", command=self.pick_files).pack(side=tk.LEFT)
        self.run_button = ttk.Button(top, text="Konwertuj", command=self.run_conversion)
        self.run_button.pack(side=tk.LEFT, padx=(10, 0))

        self.files_label = ttk.Label(root, text="Brak plików")
        self.files_label.pack(anchor="w", pady=(10, 4))

        list_frame = ttk.Frame(root)
        list_frame.pack(fill=tk.BOTH, expand=True)
        self.files_list = tk.Listbox(list_frame, height=6)
        self.files_list.pack(fill=tk.BOTH, expand=True, side=tk.LEFT)
        scrollbar = ttk.Scrollbar(list_frame, orient=tk.VERTICAL, command=self.files_list.yview)
        scrollbar.pack(fill=tk.Y, side=tk.RIGHT)
        self.files_list.configure(yscrollcommand=scrollbar.set)

        ttk.Label(root, text="Log").pack(anchor="w", pady=(10, 4))
        self.log = tk.Text(root, height=8, wrap="word", state=tk.DISABLED)
        self.log.pack(fill=tk.BOTH, expand=True)

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
                self.files_list.insert(tk.END, p.name)
        self.files_label.config(text=f"Wybrane: {len(self.selected_files)} plik(ów)")

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
        self.append_log("-" * 50)
        self.append_log(
            f"Start: plików={len(self.selected_files)}, tol={DEFAULT_TOLERANCE}"
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
                    tolerance=DEFAULT_TOLERANCE,
                    version=DEFAULT_VERSION,
                    output_path=None,
                )
                self._threadsafe_log(
                    f"OK: {result['before']} -> {result['after']} | kontury: {result['contours']}"
                )
                if result.get("splines"):
                    self._threadsafe_log(
                        f"SPLINE: {result['splines']} -> lukow: {result['arc_segments']}"
                    )
                self._threadsafe_log(f"Zapisano: {result['output']}")
            except Exception as exc:
                failures += 1
                self._threadsafe_log(f"BLAD: {path} | {exc}")

        self.after(0, self._finish_run, failures)

    def _threadsafe_log(self, text: str) -> None:
        self.after(0, self.append_log, text)

    def _finish_run(self, failures: int) -> None:
        self.run_button.configure(state=tk.NORMAL)
        self.append_log("Gotowe.")
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
