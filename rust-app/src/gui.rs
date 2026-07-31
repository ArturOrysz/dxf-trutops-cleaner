//! Native GUI (egui / eframe).

use crate::convert::convert_file;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub struct App {
    files: Vec<PathBuf>,
    tolerance: f64,
    log: String,
    busy: bool,
    rx: Option<Receiver<String>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            tolerance: 0.1,
            log: String::new(),
            busy: false,
            rx: None,
        }
    }
}

impl App {
    fn append_log(&mut self, line: impl AsRef<str>) {
        self.log.push_str(line.as_ref());
        self.log.push('\n');
    }

    fn pick_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("DXF", &["dxf"])
            .pick_files()
        {
            for p in paths {
                if !self.files.iter().any(|x| x == &p) {
                    self.files.push(p);
                }
            }
        }
    }

    fn start_convert(&mut self) {
        if self.busy || self.files.is_empty() {
            return;
        }
        self.busy = true;
        let files = self.files.clone();
        let tolerance = self.tolerance;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.append_log(format!(
            "---- Start: plikow={}, tol={:.2} ----",
            files.len(),
            tolerance
        ));
        thread::spawn(move || {
            for path in files {
                let _ = tx.send(format!("Wczytywanie: {}", path.display()));
                match convert_file(&path, tolerance, None) {
                    Ok(r) => {
                        let _ = tx.send(format!(
                            "OK: {} -> {} | kontury: {}",
                            r.before, r.after, r.contours
                        ));
                        if r.splines > 0 {
                            let _ = tx.send(format!(
                                "SPLINE: {} -> segmenty lukowe: {}",
                                r.splines, r.arc_segments
                            ));
                        }
                        let _ = tx.send(format!("Zapisano DXF: {}", r.output.display()));
                    }
                    Err(e) => {
                        let _ = tx.send(format!("BLAD: {} | {}", path.display(), e));
                    }
                }
            }
            let _ = tx.send("__DONE__".into());
        });
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        let mut messages = Vec::new();
        let mut done = false;
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(msg) if msg == "__DONE__" => {
                        done = true;
                        break;
                    }
                    Ok(msg) => messages.push(msg),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
        }
        for msg in messages {
            self.append_log(msg);
        }
        if done {
            self.busy = false;
            self.rx = None;
            self.append_log("Gotowe.");
        } else if self.busy {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DXF Cleaner dla TruTops (Rust)");
            ui.label("SPLINE / polilinie → LINE + ARC");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Wybierz pliki DXF…"))
                    .clicked()
                {
                    self.pick_files();
                }
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Wyczysc liste"))
                    .clicked()
                {
                    self.files.clear();
                }
            });

            ui.add_space(8.0);
            ui.label(format!("Tolerancja: {:.2} mm", self.tolerance));
            ui.add(egui::Slider::new(&mut self.tolerance, 0.01..=2.0).text("mm"));

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label("Pliki:");
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for f in &self.files {
                            ui.label(f.display().to_string());
                        }
                    });
            });

            ui.add_space(8.0);
            if ui
                .add_enabled(
                    !self.busy && !self.files.is_empty(),
                    egui::Button::new("Uprosc DXF"),
                )
                .clicked()
            {
                self.start_convert();
            }
            if self.busy {
                ui.spinner();
                ui.label("Konwersja…");
            }

            ui.add_space(8.0);
            ui.label("Log:");
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(220.0)
                .show(ui, |ui| {
                    ui.monospace(&self.log);
                });
        });
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 620.0])
            .with_min_inner_size([560.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DXF Cleaner dla TruTops",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
