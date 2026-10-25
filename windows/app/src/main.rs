mod audio;
mod preferences;

use eframe::egui;
use preferences::Preferences;
use std::sync::Arc;
use std::{path::PathBuf, sync::atomic::Ordering, time::Duration};

const BRAND_AMBER: egui::Color32 = egui::Color32::from_rgb(238, 153, 55);
const BRAND_CORAL: egui::Color32 = egui::Color32::from_rgb(222, 91, 72);

fn crunchy_icon() -> anyhow::Result<Arc<egui::IconData>> {
    let image =
        image::load_from_memory(include_bytes!("../../assets/crunchy-ai-icon.png"))?.into_rgba8();
    let (width, height) = image.dimensions();
    Ok(Arc::new(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }))
}

fn apply_brand_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.panel_fill = egui::Color32::from_rgb(27, 28, 30);
    visuals.window_fill = egui::Color32::from_rgb(34, 35, 37);
    visuals.extreme_bg_color = egui::Color32::from_rgb(20, 21, 23);
    visuals.faint_bg_color = egui::Color32::from_rgb(42, 43, 45);
    visuals.override_text_color = Some(egui::Color32::from_rgb(235, 231, 224));
    visuals.weak_text_color = Some(egui::Color32::from_rgb(164, 162, 157));
    visuals.hyperlink_color = BRAND_AMBER;
    visuals.warn_fg_color = BRAND_AMBER;
    visuals.error_fg_color = BRAND_CORAL;
    visuals.selection.bg_fill = egui::Color32::from_rgb(115, 67, 38);
    visuals.selection.stroke.color = egui::Color32::from_rgb(255, 220, 170);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(48, 49, 51);
    visuals.widgets.inactive.bg_stroke.color = egui::Color32::from_rgb(68, 68, 69);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(67, 53, 42);
    visuals.widgets.hovered.bg_stroke.color = BRAND_AMBER;
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(128, 74, 42);
    visuals.widgets.active.bg_stroke.color = egui::Color32::from_rgb(242, 177, 104);
    ctx.set_style(style);
}

#[cfg(test)]
mod brand_tests {
    #[test]
    fn crunchy_brand_icon_loads_as_a_square_rgba_image() {
        let icon = super::crunchy_icon().expect("Crunchy AI icon should be bundled and valid");
        assert!(icon.width >= 256);
        assert_eq!(icon.width, icon.height);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }
}

fn assets() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets")
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--devices") {
        for name in audio::devices()? {
            println!("{name}");
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--check-vbcable") {
        let (ready, message) = audio::driver_status();
        println!("{}: {message}", if ready { "OK" } else { "AUSENTE" });
        if !ready {
            anyhow::bail!("{message}");
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--self-test") {
        return audio::self_test(&assets());
    }
    eframe::run_native(
        "Crunchy AI • Windows",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([620.0, 510.0])
                .with_icon(crunchy_icon()?),
            ..Default::default()
        },
        Box::new(|cc| {
            apply_brand_theme(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

struct App {
    brand_logo: Option<egui::TextureHandle>,
    devices: Vec<String>,
    selected: usize,
    light: bool,
    mode: usize,
    strength: u32,
    monitoring: bool,
    session: Option<audio::Session>,
    error: String,
    driver_ready: bool,
    driver_message: String,
    saved_preferences: Preferences,
    restore_saved_microphone: bool,
}

impl Default for App {
    fn default() -> Self {
        let result = audio::devices();
        let error = result
            .as_ref()
            .err()
            .map(ToString::to_string)
            .unwrap_or_default();
        let (driver_ready, driver_message) = audio::driver_status();
        let saved_preferences = Preferences::load();
        let saved_selection = saved_preferences
            .microphone
            .as_ref()
            .and_then(|saved| result.as_ref().ok()?.iter().position(|name| name == saved));
        let restore_saved_microphone =
            saved_preferences.microphone.is_some() && saved_selection.is_none();
        let selected = saved_selection.unwrap_or(0);
        Self {
            brand_logo: None,
            devices: result.unwrap_or_default(),
            selected,
            light: saved_preferences.light_model,
            mode: saved_preferences.mode,
            strength: saved_preferences.strength,
            monitoring: false,
            session: None,
            error,
            driver_ready,
            driver_message,
            saved_preferences,
            restore_saved_microphone,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));
        let mut selected_by_user = false;
        if let Some(session) = &self.session {
            self.monitoring = session.control.monitor.load(Ordering::Relaxed);
            if let Ok(error) = session.monitor_errors.try_recv() {
                self.error = error;
            }
        }
        if let Some(session) = &self.session {
            if let Ok(error) = session.errors.try_recv() {
                self.error = error;
                self.session = None;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if self.brand_logo.is_none() {
                    if let Ok(image) = image::load_from_memory(include_bytes!("../../assets/crunchy-ai-icon.png")) {
                        let image = image.into_rgba8();
                        let size = [image.width() as usize, image.height() as usize];
                        let pixels = image.into_raw();
                        self.brand_logo = Some(ctx.load_texture(
                            "crunchy-ai-brand-logo",
                            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
                if let Some(logo) = &self.brand_logo {
                    ui.add(egui::Image::new((logo.id(), egui::vec2(38.0, 38.0))));
                }
                ui.heading(egui::RichText::new("Crunchy AI").color(BRAND_AMBER));
                ui.label(egui::RichText::new("SOM CROCANTE").small().color(BRAND_CORAL));
            });
            ui.label("Redução de ruído local • Windows x64 • processamento local");
            ui.separator();
            if self.session.is_none() {
                ui.colored_label(
                    if self.driver_ready {
                        egui::Color32::from_rgb(142, 184, 139)
                    } else {
                        BRAND_AMBER
                    },
                    &self.driver_message,
                );
                if ui.button("Verificar driver").clicked() {
                    (self.driver_ready, self.driver_message) = audio::driver_status();
                    self.error.clear();
                }
            }
            ui.add_space(8.0);
            ui.label("Microfone físico");
            ui.add_enabled_ui(self.session.is_none(), |ui| {
                egui::ComboBox::from_id_salt("mic").width(460.0)
                    .selected_text(self.devices.get(self.selected).map(String::as_str).unwrap_or("Nenhum microfone encontrado"))
                    .show_ui(ui, |ui| {
                        for (i, name) in self.devices.iter().enumerate() {
                            selected_by_user |= ui
                                .selectable_value(&mut self.selected, i, name)
                                .changed();
                        }
                    });
                if ui.button("Atualizar dispositivos").clicked() {
                    let current = if self.restore_saved_microphone {
                        self.saved_preferences.microphone.clone()
                    } else {
                        self.devices.get(self.selected).cloned()
                    };
                    match audio::devices() {
                        Ok(devices) => {
                            self.devices = devices;
                            self.selected = current
                                .as_ref()
                                .and_then(|name| self.devices.iter().position(|item| item == name))
                                .unwrap_or(0);
                        }
                        Err(e) => self.error = e.to_string(),
                    }
                }
                ui.checkbox(&mut self.light, "Modelo leve (menor uso de CPU)");
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, 0, "Supressão");
                ui.selectable_value(&mut self.mode, 1, "Original");
                ui.selectable_value(&mut self.mode, 2, "Mudo");
            });
            ui.add(egui::Slider::new(&mut self.strength, 0..=100).text("Supressão máxima (dB)"));
            ui.add_enabled_ui(self.session.is_some(), |ui| {
                ui.checkbox(&mut self.monitoring, "Monitorar áudio processado na saída padrão (fones)");
                ui.small("Ative para testar o som; use fones para evitar microfonia. Desative para continuar sem retorno local.");
            });
            if let Some(session) = &self.session {
                session.control.mode.store(self.mode, Ordering::Relaxed);
                session.control.strength.store(self.strength, Ordering::Relaxed);
                session.control.monitor.store(self.monitoring, Ordering::Relaxed);
                let peak = f32::from_bits(session.control.peak.load(Ordering::Relaxed));
                ui.add(egui::ProgressBar::new(peak.min(1.0)).text("Nível de entrada"));
                let us = session.control.micros.load(Ordering::Relaxed);
                let dropped = session.control.dropped.load(Ordering::Relaxed);
                let underruns = session.control.underruns.load(Ordering::Relaxed);
                ui.label(format!("Ativo • processamento: {:.1} ms / bloco de 10 ms • amostras descartadas: {dropped} • falhas de preenchimento: {underruns}", us as f64 / 1000.0));
                if ui.button("Parar").clicked() { self.session = None; }
            } else if ui.add_enabled(self.driver_ready && !self.devices.is_empty(), egui::Button::new("Iniciar Crunchy AI")).clicked() {
                self.error.clear();
                match audio::Session::start(self.devices[self.selected].clone(), assets(), self.light, self.mode, self.strength) {
                    Ok(session) => self.session = Some(session),
                    Err(e) => {
                        self.error = format!("{e:#}");
                        (self.driver_ready, self.driver_message) = audio::driver_status();
                    }
                }
            }
            ui.add_space(12.0);
            ui.label("No Discord/OBS, selecione CABLE Output (VB-Audio Virtual Cable) como microfone.");
            ui.label("O HushMic envia o áudio processado ao CABLE Input do VB-CABLE.");
            ui.hyperlink_to("VB-CABLE é donationware • Informações e contribuição para VB-Audio", "https://vb-audio.com/Cable/");
            ui.label("Ao parar ou fechar, o microfone virtual fica em silêncio.");
            if !self.error.is_empty() {
                ui.add_space(8.0);
                ui.colored_label(BRAND_CORAL, &self.error);
            }
        });

        if selected_by_user {
            self.restore_saved_microphone = false;
        }

        let current = Preferences {
            microphone: if self.restore_saved_microphone {
                self.saved_preferences.microphone.clone()
            } else {
                self.devices
                    .get(self.selected)
                    .cloned()
                    .or_else(|| self.saved_preferences.microphone.clone())
            },
            light_model: self.light,
            mode: self.mode,
            strength: self.strength,
        };
        if current != self.saved_preferences {
            if let Err(error) = current.save() {
                self.error = format!("Não foi possível salvar as preferências: {error}");
            }
            self.saved_preferences = current;
        }
    }
}
