use eframe::egui::{self, Color32, RichText};

use crate::shared::{fmt_ms, LogCat};
use crate::App;

const GREEN: Color32 = Color32::from_rgb(64, 200, 96);
const GREY: Color32 = Color32::from_rgb(120, 120, 120);
const AMBER: Color32 = Color32::from_rgb(230, 180, 60);

impl App {
    pub(crate) fn ui(&mut self, ctx: &egui::Context) {
        self.top_bar(ctx);
        self.left_panel(ctx);
        self.log_panel(ctx);
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Load .wasm").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("WebAssembly", &["wasm"])
                        .pick_file()
                    {
                        self.load_wasm(p, ctx);
                    }
                }
                if ui.button("Load .lss").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("LiveSplit splits", &["lss"])
                        .pick_file()
                    {
                        self.load_lss(p);
                    }
                }

                ui.separator();

                let (attached, running, idx, igt) = {
                    let g = self.shared.lock().unwrap();
                    (g.attached, g.run_active, g.current_split_index, g.igt_ms)
                };

                if attached {
                    ui.colored_label(GREEN, "● attached");
                } else {
                    ui.colored_label(GREY, "○ detached");
                }
                ui.separator();
                if running {
                    ui.colored_label(GREEN, "Running");
                } else {
                    ui.colored_label(GREY, "NotRunning");
                }
                ui.separator();
                ui.label(format!("split #{idx}"));
                ui.separator();
                ui.label(format!("IGT {}", fmt_ms(igt.max(0) as u128)));
            });

            ui.horizontal(|ui| {
                let wasm = self
                    .wasm_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "— no wasm loaded —".into());
                let lss = self
                    .lss_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "— no .lss —".into());
                ui.small(RichText::new(format!("wasm: {wasm}    lss: {lss}")).color(GREY));
            });
            ui.add_space(4.0);
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                let (segments, cur, running, counters) = {
                    let g = self.shared.lock().unwrap();
                    (g.segments.clone(), g.current_split_index, g.run_active, g.counters.clone())
                };

                ui.add_space(4.0);
                ui.heading("Segments");
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("segments")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        if segments.is_empty() {
                            ui.small(RichText::new("Load a .lss to see the segment list.").color(GREY));
                        }
                        for (i, seg) in segments.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if i < cur {
                                    let t = seg
                                        .times
                                        .map(|t| format!("{}  (+{})", fmt_ms(t.total_ms), fmt_ms(t.segment_ms)))
                                        .unwrap_or_else(|| "—".into());
                                    ui.colored_label(GREEN, "✓");
                                    ui.label(&seg.name);
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.monospace(t);
                                    });
                                } else if i == cur && running {
                                    ui.colored_label(AMBER, "▶");
                                    ui.label(RichText::new(&seg.name).strong());
                                } else {
                                    ui.colored_label(GREY, "–");
                                    ui.colored_label(GREY, &seg.name);
                                }
                            });
                        }
                    });

                ui.add_space(8.0);
                ui.heading("Counters");
                ui.separator();
                if counters.is_empty() {
                    ui.small(RichText::new("No set_variable emissions yet.").color(GREY));
                }
                for (name, c) in &counters {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.small(RichText::new(format!("#{}", c.split_index)).color(GREY));
                            ui.monospace(&c.value);
                        });
                    });
                }

                ui.add_space(8.0);
                ui.heading("Events");
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("events")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in self.logs.iter().filter(|l| l.cat == LogCat::Timer) {
                            ui.monospace(&line.text);
                        }
                    });
            });
    }

    fn log_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.strong("Log");
                ui.separator();
                for cat in LogCat::ALL {
                    let mut on = self.enabled.contains(&cat);
                    if ui.checkbox(&mut on, cat.label()).changed() {
                        if on {
                            self.enabled.insert(cat);
                        } else {
                            self.enabled.remove(&cat);
                        }
                    }
                }
                ui.separator();
                ui.checkbox(&mut self.autoscroll, "autoscroll");
                if ui.button("Clear").clicked() {
                    self.logs.clear();
                }
                if ui.button("Copy log").clicked() {
                    let text = self
                        .logs
                        .iter()
                        .map(|l| l.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.output_mut(|o| o.copied_text = text);
                }
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("log")
                .auto_shrink([false, false])
                .stick_to_bottom(self.autoscroll)
                .show(ui, |ui| {
                    for line in self.logs.iter().filter(|l| self.enabled.contains(&l.cat)) {
                        ui.monospace(RichText::new(&line.text).color(cat_color(line.cat)));
                    }
                });
        });
    }
}

fn cat_color(cat: LogCat) -> Color32 {
    match cat {
        LogCat::Timer => GREEN,
        LogCat::Var => AMBER,
        LogCat::Wasm => Color32::from_rgb(150, 190, 240),
        LogCat::Runtime => GREY,
        LogCat::Trace => Color32::from_rgb(170, 140, 210),
    }
}
