use eframe::egui::{
    self, Align, Button, CentralPanel, Color32, Context, CornerRadius, Frame, Label,
    Layout, Margin, RichText, ScrollArea, SidePanel, Stroke, TextEdit, TopBottomPanel, Ui, Vec2,
    ViewportBuilder,
};

use crate::{core::SighFarCore, models::{EncodedMessage, HistoryEntry, OperationKind}};

pub fn launch_gui() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(Vec2::new(1240.0, 760.0))
            .with_min_inner_size(Vec2::new(980.0, 640.0))
            .with_title("SighFar // SmileFar"),
        ..Default::default()
    };

    eframe::run_native(
        "SighFar",
        options,
        Box::new(|cc| {
            configure_theme(&cc.egui_ctx);
            Ok(Box::<SmileFarGui>::default())
        }),
    )
    .map_err(|err| anyhow::anyhow!("failed to launch GUI: {err}"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Encode,
    Decode,
    History,
    Settings,
    Roadmap,
}

struct EncodeState {
    message: String,
    chain: String,
    secure: bool,
    passphrase: String,
    result: Option<EncodedMessage>,
    status: String,
}

impl Default for EncodeState {
    fn default() -> Self {
        Self {
            message: String::new(),
            chain: "morse,caesar:4,reverse".to_string(),
            secure: true,
            passphrase: String::new(),
            result: None,
            status: "Ready to encode a message.".to_string(),
        }
    }
}

#[derive(Default)]
struct DecodeState {
    input: String,
    chain: String,
    secure: bool,
    passphrase: String,
    companion_code: String,
    result: Option<EncodedMessage>,
    status: String,
}

pub struct SmileFarGui {
    core: SighFarCore,
    active_tab: Tab,
    encode: EncodeState,
    decode: DecodeState,
    history: Vec<HistoryEntry>,
    history_status: String,
}

impl Default for SmileFarGui {
    fn default() -> Self {
        let core = SighFarCore::default();
        let history = core.load_history().unwrap_or_default();
        Self {
            core,
            active_tab: Tab::Encode,
            encode: EncodeState::default(),
            decode: DecodeState {
                chain: "morse,caesar:4,reverse".to_string(),
                status: "Ready to decode a message.".to_string(),
                ..Default::default()
            },
            history,
            history_status: "Encrypted history loaded from local storage.".to_string(),
        }
    }
}

impl eframe::App for SmileFarGui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("title_bar")
            .exact_height(52.0)
            .frame(panel_frame(Color32::from_rgb(118, 22, 19), 8.0))
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(RichText::new("SmileFar OS 2.0").size(24.0).strong().color(Color32::WHITE));
                    ui.add_space(12.0);
                    ui.label(RichText::new("offline cipher workbench").size(16.0).color(Color32::from_rgb(255, 192, 170)));
                });
            });

        SidePanel::left("nav")
            .exact_width(260.0)
            .resizable(false)
            .frame(panel_frame(Color32::from_rgb(29, 29, 29), 16.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("Smile").size(42.0).strong().color(Color32::WHITE));
                    ui.label(RichText::new("Far").size(28.0).strong().color(Color32::from_rgb(255, 181, 61)));
                    ui.label(RichText::new("Rust pivot branch").size(14.0).color(Color32::from_rgb(205, 150, 145)));
                    ui.add_space(20.0);
                });

                for (tab, label) in [
                    (Tab::Encode, "Encode"),
                    (Tab::Decode, "Decode"),
                    (Tab::History, "Encrypted History"),
                    (Tab::Settings, "Settings"),
                    (Tab::Roadmap, "Roadmap"),
                ] {
                    let selected = self.active_tab == tab;
                    let button = Button::new(RichText::new(label).size(18.0))
                        .min_size(Vec2::new(220.0, 44.0))
                        .fill(if selected { Color32::from_rgb(135, 38, 31) } else { Color32::from_rgb(55, 55, 55) })
                        .stroke(Stroke::new(2.0, Color32::from_rgb(157, 92, 74)))
                        .corner_radius(8.0);
                    if ui.add(button).clicked() {
                        self.active_tab = tab;
                    }
                    ui.add_space(8.0);
                }

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Ultrakill-inspired shell, offline-first workflow.").size(13.0).color(Color32::from_rgb(160, 160, 160)));
                });
            });

        CentralPanel::default()
            .frame(panel_frame(Color32::from_rgb(18, 18, 18), 20.0))
            .show(ctx, |ui| {
                match self.active_tab {
                    Tab::Encode => self.render_encode(ui),
                    Tab::Decode => self.render_decode(ui),
                    Tab::History => self.render_history(ui),
                    Tab::Settings => self.render_settings(ui),
                    Tab::Roadmap => self.render_roadmap(ui),
                }
            });
    }
}

impl SmileFarGui {
    fn render_encode(&mut self, ui: &mut Ui) {
        panel_title(ui, "Tip of the Day");
        framed_body(ui, |ui| {
            ui.label(RichText::new("Layer your ciphers, then split the passphrase and companion code across different channels.").size(18.0));
            ui.add_space(16.0);
            ui.label("Message");
            ui.add(TextEdit::multiline(&mut self.encode.message).desired_rows(6).hint_text("Enter plaintext to transform"));
            ui.add_space(12.0);
            ui.label("Technique chain");
            ui.add(TextEdit::singleline(&mut self.encode.chain).hint_text("morse,caesar:4,reverse"));
            ui.checkbox(&mut self.encode.secure, "Wrap output in secure paired-key envelope");
            if self.encode.secure {
                ui.label("Primary passphrase");
                ui.add(TextEdit::singleline(&mut self.encode.passphrase).password(true).hint_text("Shared secret"));
            }
            ui.add_space(12.0);
            if ui.add(accent_button("Encode")).clicked() {
                match self.core.encode(
                    &self.encode.message,
                    &self.encode.chain,
                    self.encode.secure,
                    Some(&self.encode.passphrase),
                ) {
                    Ok(result) => {
                        self.encode.status = "Message encoded successfully.".to_string();
                        self.encode.result = Some(result);
                        self.refresh_history();
                    }
                    Err(err) => {
                        self.encode.status = err.to_string();
                    }
                }
            }
            ui.add_space(10.0);
            status_line(ui, &self.encode.status);

            if let Some(result) = &self.encode.result {
                ui.separator();
                ui.label(RichText::new("Transformed text").strong());
                output_box(ui, &result.transformed_text);
                if let Some(payload) = &result.secure_payload {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Secure payload").strong());
                    output_box(ui, payload);
                }
                if let Some(keys) = &result.key_pair {
                    ui.add_space(8.0);
                    ui.columns(2, |columns| {
                        columns[0].label(RichText::new("Primary passphrase").strong());
                        output_box(&mut columns[0], &keys.passphrase);
                        columns[1].label(RichText::new("Companion code").strong());
                        output_box(&mut columns[1], &keys.companion_code);
                    });
                }
            }
        });
    }

    fn render_decode(&mut self, ui: &mut Ui) {
        panel_title(ui, "Recovery Console");
        framed_body(ui, |ui| {
            ui.label(RichText::new("Recover plaintext using the original chain and, if needed, both key parts.").size(18.0));
            ui.add_space(16.0);
            ui.checkbox(&mut self.decode.secure, "Input is a secure payload");
            ui.label(if self.decode.secure { "Secure payload" } else { "Cipher text" });
            ui.add(TextEdit::multiline(&mut self.decode.input).desired_rows(6));
            ui.add_space(12.0);
            ui.label("Technique chain");
            ui.add(TextEdit::singleline(&mut self.decode.chain).hint_text("morse,caesar:4,reverse"));
            if self.decode.secure {
                ui.columns(2, |columns| {
                    columns[0].label("Primary passphrase");
                    columns[0].add(TextEdit::singleline(&mut self.decode.passphrase).password(true));
                    columns[1].label("Companion code");
                    columns[1].add(TextEdit::singleline(&mut self.decode.companion_code));
                });
            }
            ui.add_space(12.0);
            if ui.add(accent_button("Decode")).clicked() {
                match self.core.decode(
                    &self.decode.input,
                    &self.decode.chain,
                    self.decode.secure,
                    Some(&self.decode.passphrase),
                    Some(&self.decode.companion_code),
                ) {
                    Ok(result) => {
                        self.decode.status = "Message decoded successfully.".to_string();
                        self.decode.result = Some(result);
                        self.refresh_history();
                    }
                    Err(err) => {
                        self.decode.status = err.to_string();
                    }
                }
            }
            ui.add_space(10.0);
            status_line(ui, &self.decode.status);
            if let Some(result) = &self.decode.result {
                ui.separator();
                ui.label(RichText::new("Decoded text").strong());
                output_box(ui, &result.transformed_text);
            }
        });
    }

    fn render_history(&mut self, ui: &mut Ui) {
        panel_title(ui, "Encrypted History");
        framed_body(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Encrypted event log").size(18.0).strong());
                if ui.add(accent_button("Refresh")).clicked() {
                    self.refresh_history();
                }
            });
            status_line(ui, &self.history_status);
            ui.add_space(8.0);
            ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                for entry in &self.history {
                    let title = match entry.operation {
                        OperationKind::Encode => "Encode",
                        OperationKind::Decode => "Decode",
                    };
                    Frame::new()
                        .fill(Color32::from_rgb(27, 27, 27))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(113, 56, 47)))
                        .corner_radius(10.0)
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(title).strong().color(Color32::from_rgb(255, 181, 61)));
                                ui.add_space(8.0);
                                ui.label(entry.timestamp.to_rfc3339());
                            });
                            ui.label(format!("in: {}", entry.input_preview));
                            ui.label(format!("out: {}", entry.output_preview));
                            ui.label(format!(
                                "chain: {}",
                                entry.techniques.iter().map(|item| item.title()).collect::<Vec<_>>().join(" -> ")
                            ));
                            ui.label(format!("secure: {}", if entry.used_secure_envelope { "yes" } else { "no" }));
                        });
                    ui.add_space(10.0);
                }
                if self.history.is_empty() {
                    ui.add(Label::new("No encrypted history yet. Use Encode or Decode first."));
                }
            });
        });
    }

    fn render_settings(&mut self, ui: &mut Ui) {
        panel_title(ui, "System Settings");
        framed_body(ui, |ui| {
            ui.label(RichText::new("Current prototype switches").size(18.0).strong());
            ui.add_space(12.0);
            ui.label("GitHub OAuth: planned");
            ui.label("Update channel: planned");
            ui.label("File hiding / carrier mode: planned");
            ui.separator();
            ui.label(RichText::new("Local encrypted history").strong());
            output_box(ui, &self.core.diagnostics());
            ui.add_space(8.0);
            ui.label("Production note: move secrets into OS-backed key stores per platform once GUI workflows stabilize.");
        });
    }

    fn render_roadmap(&mut self, ui: &mut Ui) {
        panel_title(ui, "Roadmap Grid");
        framed_body(ui, |ui| {
            roadmap_card(ui, "Phase 1", &[
                "offline Rust core",
                "stacked cipher workflows",
                "encrypted local history",
            ]);
            roadmap_card(ui, "Phase 2", &[
                "SmileOS-inspired GUI shell",
                "drag and drop carrier workflows",
                "key bundle import/export",
            ]);
            roadmap_card(ui, "Phase 3", &[
                "GitHub OAuth in settings",
                "signed release packaging",
                "platform update channels",
            ]);
        });
    }

    fn refresh_history(&mut self) {
        match self.core.load_history() {
            Ok(history) => {
                self.history = history;
                self.history_status = "Encrypted history refreshed.".to_string();
            }
            Err(err) => {
                self.history_status = err.to_string();
            }
        }
    }
}

fn configure_theme(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgb(13, 13, 13);
    visuals.panel_fill = Color32::from_rgb(16, 16, 16);
    visuals.extreme_bg_color = Color32::from_rgb(12, 12, 12);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 22, 22);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(48, 48, 48);
    visuals.widgets.active.bg_fill = Color32::from_rgb(118, 22, 19);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(154, 43, 35);
    visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
    visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.override_text_color = Some(Color32::from_rgb(240, 236, 228));
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(18.0, 10.0);
    style.visuals.window_corner_radius = CornerRadius::same(12);
    ctx.set_style(style);
}

fn panel_frame(fill: Color32, margin: f32) -> Frame {
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(2.0, Color32::from_rgb(121, 55, 44)))
        .corner_radius(12.0)
        .inner_margin(Margin::same(margin as i8))
}

fn panel_title(ui: &mut Ui, title: &str) {
    Frame::new()
        .fill(Color32::from_rgb(121, 29, 23))
        .stroke(Stroke::new(2.0, Color32::from_rgb(170, 78, 58)))
        .corner_radius(10.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).size(28.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("[ ]  [ ]  [ ]").size(18.0).color(Color32::from_rgb(255, 183, 146)));
                });
            });
        });
    ui.add_space(12.0);
}

fn framed_body(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    panel_frame(Color32::from_rgb(20, 20, 20), 18.0).show(ui, add_contents);
}

fn accent_button(label: &str) -> Button<'static> {
    Button::new(RichText::new(label).size(18.0).strong())
        .fill(Color32::from_rgb(146, 38, 29))
        .stroke(Stroke::new(2.0, Color32::from_rgb(225, 132, 80)))
        .corner_radius(8.0)
}

fn output_box(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(Color32::from_rgb(10, 10, 10))
        .stroke(Stroke::new(1.0, Color32::from_rgb(109, 54, 44)))
        .corner_radius(8.0)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace());
        });
}

fn status_line(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(Color32::from_rgb(255, 183, 146)));
}

fn roadmap_card(ui: &mut Ui, title: &str, items: &[&str]) {
    Frame::new()
        .fill(Color32::from_rgb(24, 24, 24))
        .stroke(Stroke::new(1.0, Color32::from_rgb(121, 55, 44)))
        .corner_radius(10.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(20.0).strong().color(Color32::from_rgb(255, 181, 61)));
            ui.add_space(8.0);
            for item in items {
                ui.label(format!("• {item}"));
            }
        });
    ui.add_space(10.0);
}
