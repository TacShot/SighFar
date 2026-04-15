use std::{fs, path::PathBuf};

use eframe::egui::{
    self, Align, Button, CentralPanel, Color32, ComboBox, Context, CornerRadius, FontData,
    FontDefinitions, FontFamily, FontId, Frame, Label, Layout, Margin, RichText, ScrollArea,
    SidePanel, Stroke, TextEdit, TextStyle, TopBottomPanel, Ui, Vec2, ViewportBuilder,
};
use rfd::FileDialog;

use crate::{
    carrier::{embed_file, extract_file},
    config::{AppConfig, ConfigStore},
    core::SighFarCore,
    github_sync::{DeviceAuthState, DevicePollStatus, GitHubSession, GitHubSyncClient},
    models::{EncodedMessage, HistoryEntry, OperationKind, TechniqueDescriptor},
};

pub fn launch_gui() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(Vec2::new(1360.0, 860.0))
            .with_min_inner_size(Vec2::new(1060.0, 720.0))
            .with_title("SighFar // SmileFar"),
        ..Default::default()
    };

    eframe::run_native(
        "SighFar",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
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
    Carrier,
    History,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DraftTechniqueKind {
    Morse,
    Caesar,
    Vigenere,
    RailFence,
    Reverse,
}

impl DraftTechniqueKind {
    fn label(self) -> &'static str {
        match self {
            Self::Morse => "Morse",
            Self::Caesar => "Caesar",
            Self::Vigenere => "Vigenere",
            Self::RailFence => "Rail Fence",
            Self::Reverse => "Reverse",
        }
    }

    fn all() -> [Self; 5] {
        [Self::Morse, Self::Caesar, Self::Vigenere, Self::RailFence, Self::Reverse]
    }
}

#[derive(Clone)]
struct TechniqueBuilder {
    items: Vec<TechniqueDescriptor>,
    draft_kind: DraftTechniqueKind,
    draft_shift: i32,
    draft_keyword: String,
    draft_rails: usize,
}

impl TechniqueBuilder {
    fn with_defaults() -> Self {
        Self {
            items: vec![
                TechniqueDescriptor::Morse,
                TechniqueDescriptor::Caesar { shift: 4 },
                TechniqueDescriptor::Reverse,
            ],
            draft_kind: DraftTechniqueKind::Caesar,
            draft_shift: 4,
            draft_keyword: "smile".to_string(),
            draft_rails: 3,
        }
    }

    fn add_current(&mut self) {
        let item = match self.draft_kind {
            DraftTechniqueKind::Morse => TechniqueDescriptor::Morse,
            DraftTechniqueKind::Caesar => TechniqueDescriptor::Caesar {
                shift: self.draft_shift,
            },
            DraftTechniqueKind::Vigenere => TechniqueDescriptor::Vigenere {
                keyword: self.draft_keyword.trim().to_string(),
            },
            DraftTechniqueKind::RailFence => TechniqueDescriptor::RailFence {
                rails: self.draft_rails.max(2),
            },
            DraftTechniqueKind::Reverse => TechniqueDescriptor::Reverse,
        };
        self.items.push(item);
    }

    fn chain_string(&self) -> String {
        self.items
            .iter()
            .map(|item| match item {
                TechniqueDescriptor::Morse => "morse".to_string(),
                TechniqueDescriptor::Caesar { shift } => format!("caesar:{shift}"),
                TechniqueDescriptor::Vigenere { keyword } => format!("vigenere:{keyword}"),
                TechniqueDescriptor::RailFence { rails } => format!("railfence:{rails}"),
                TechniqueDescriptor::Reverse => "reverse".to_string(),
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

struct EncodeState {
    message: String,
    technique_builder: TechniqueBuilder,
    secure: bool,
    passphrase: String,
    result: Option<EncodedMessage>,
    status: String,
}

impl Default for EncodeState {
    fn default() -> Self {
        Self {
            message: String::new(),
            technique_builder: TechniqueBuilder::with_defaults(),
            secure: true,
            passphrase: String::new(),
            result: None,
            status: "Ready to encode a message.".to_string(),
        }
    }
}

struct DecodeState {
    input: String,
    technique_builder: TechniqueBuilder,
    secure: bool,
    passphrase: String,
    companion_code: String,
    result: Option<EncodedMessage>,
    status: String,
}

impl Default for DecodeState {
    fn default() -> Self {
        Self {
            input: String::new(),
            technique_builder: TechniqueBuilder::with_defaults(),
            secure: false,
            passphrase: String::new(),
            companion_code: String::new(),
            result: None,
            status: "Ready to decode a message.".to_string(),
        }
    }
}

#[derive(Default)]
struct CarrierState {
    carrier_path: String,
    payload_path: String,
    output_path: String,
    extract_container_path: String,
    extract_output_dir: String,
    status: String,
}

struct SettingsState {
    config: AppConfig,
    sync_status: String,
    signed_in: Option<GitHubSession>,
    device_flow: Option<DeviceAuthState>,
}

impl SettingsState {
    fn new(config: AppConfig) -> Self {
        Self {
            config,
            sync_status: "GitHub sync is not connected yet.".to_string(),
            signed_in: None,
            device_flow: None,
        }
    }
}

pub struct SmileFarGui {
    core: SighFarCore,
    config_store: ConfigStore,
    github_sync: GitHubSyncClient,
    active_tab: Tab,
    encode: EncodeState,
    decode: DecodeState,
    carrier: CarrierState,
    settings: SettingsState,
    history: Vec<HistoryEntry>,
    history_status: String,
}

impl Default for SmileFarGui {
    fn default() -> Self {
        let core = SighFarCore::default();
        let config_store = ConfigStore::default();
        let config = config_store.load().unwrap_or_default();
        let history = core.load_history().unwrap_or_default();

        Self {
            core,
            config_store,
            github_sync: GitHubSyncClient::default(),
            active_tab: Tab::Encode,
            encode: EncodeState::default(),
            decode: DecodeState::default(),
            carrier: CarrierState {
                status: "Carrier mode appends a hidden payload trailer to another file. It is not steganography.".to_string(),
                ..Default::default()
            },
            settings: SettingsState::new(config),
            history,
            history_status: "Encrypted history loaded from local storage.".to_string(),
        }
    }
}

impl eframe::App for SmileFarGui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("title_bar")
            .exact_height(54.0)
            .frame(panel_frame(Color32::from_rgb(150, 31, 20), 6))
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(
                        RichText::new("SMILEFAR OS 2.1")
                            .size(24.0)
                            .monospace()
                            .strong()
                            .color(Color32::from_rgb(255, 249, 243)),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("OFFLINE CIPHER WORKBENCH")
                            .size(16.0)
                            .monospace()
                            .color(Color32::from_rgb(255, 223, 195)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("[ ] [ ] [ ]")
                                .size(17.0)
                                .monospace()
                                .color(Color32::from_rgb(255, 201, 168)),
                        );
                    });
                });
            });

        SidePanel::left("nav")
            .exact_width(268.0)
            .resizable(false)
            .frame(panel_frame(Color32::from_rgb(24, 24, 24), 16))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("SMILE")
                            .size(36.0)
                            .monospace()
                            .strong()
                            .color(Color32::from_rgb(255, 252, 247)),
                    );
                    ui.label(
                        RichText::new("FAR")
                            .size(28.0)
                            .monospace()
                            .strong()
                            .color(Color32::from_rgb(255, 191, 79)),
                    );
                    ui.label(
                        RichText::new("RUST DESKTOP BUILD")
                            .size(13.0)
                            .monospace()
                            .color(Color32::from_rgb(225, 214, 206)),
                    );
                    ui.add_space(18.0);
                });

                for (tab, label) in [
                    (Tab::Encode, "Encode"),
                    (Tab::Decode, "Decode"),
                    (Tab::Carrier, "Carrier File"),
                    (Tab::History, "Encrypted History"),
                    (Tab::Settings, "Settings"),
                ] {
                    let selected = self.active_tab == tab;
                    let fill = if selected {
                        Color32::from_rgb(160, 43, 29)
                    } else {
                        Color32::from_rgb(62, 62, 62)
                    };
                    let text = if selected {
                        Color32::from_rgb(255, 248, 244)
                    } else {
                        Color32::from_rgb(245, 239, 234)
                    };
                    let button = Button::new(RichText::new(label).size(17.0).monospace().strong().color(text))
                        .min_size(Vec2::new(226.0, 44.0))
                        .fill(fill)
                        .stroke(Stroke::new(2.0, Color32::from_rgb(209, 126, 98)))
                        .corner_radius(3.0);
                    if ui.add(button).clicked() {
                        self.active_tab = tab;
                    }
                    ui.add_space(8.0);
                }

            });

        CentralPanel::default()
            .frame(panel_frame(Color32::from_rgb(16, 16, 16), 20))
            .show(ctx, |ui| match self.active_tab {
                Tab::Encode => self.render_encode(ui),
                Tab::Decode => self.render_decode(ui),
                Tab::Carrier => self.render_carrier(ui),
                Tab::History => self.render_history(ui),
                Tab::Settings => self.render_settings(ui),
            });
    }
}

impl SmileFarGui {
    fn render_encode(&mut self, ui: &mut Ui) {
        panel_title(ui, "Encode Console");
        framed_body(ui, |ui| {
            ui.label(
                RichText::new("Build your cipher chain from the dropdown, then split the keys if you wrap the result.")
                    .size(20.0)
                    .monospace()
                    .color(primary_text()),
            );
            ui.add_space(20.0);
            ui.label(section_label("MESSAGE"));
            ui.add(TextEdit::multiline(&mut self.encode.message).desired_rows(6).hint_text("Enter plaintext to transform"));
            ui.add_space(18.0);
            render_technique_builder(ui, "encode", &mut self.encode.technique_builder);
            ui.add_space(14.0);
            ui.checkbox(
                &mut self.encode.secure,
                RichText::new("WRAP OUTPUT IN SECURE PAIRED-KEY ENVELOPE")
                    .monospace()
                    .color(primary_text()),
            );
            if self.encode.secure {
                ui.label(RichText::new("Primary passphrase").strong());
                ui.add(TextEdit::singleline(&mut self.encode.passphrase).password(true).hint_text("Shared secret"));
            }
            ui.add_space(16.0);
            if ui.add(accent_button("Encode")).clicked() {
                match self.core.encode(
                    &self.encode.message,
                    &self.encode.technique_builder.chain_string(),
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
                ui.label(section_label("TRANSFORMED TEXT"));
                output_box(ui, &result.transformed_text);
                if let Some(payload) = &result.secure_payload {
                    ui.add_space(8.0);
                    ui.label(section_label("SECURE PAYLOAD"));
                    output_box(ui, payload);
                }
                if let Some(keys) = &result.key_pair {
                    ui.add_space(8.0);
                    ui.columns(2, |columns| {
                        columns[0].label(section_label("PRIMARY PASSPHRASE"));
                        output_box(&mut columns[0], &keys.passphrase);
                        columns[1].label(section_label("COMPANION CODE"));
                        output_box(&mut columns[1], &keys.companion_code);
                    });
                }
            }
        });
    }

    fn render_decode(&mut self, ui: &mut Ui) {
        panel_title(ui, "Decode Console");
        framed_body(ui, |ui| {
            ui.label(
                RichText::new("Rebuild the original chain, then provide both key parts if the input is securely wrapped.")
                    .size(20.0)
                    .monospace()
                    .color(primary_text()),
            );
            ui.add_space(20.0);
            ui.checkbox(
                &mut self.decode.secure,
                RichText::new("INPUT IS A SECURE PAYLOAD")
                    .monospace()
                    .color(primary_text()),
            );
            ui.label(section_label(if self.decode.secure { "SECURE PAYLOAD" } else { "CIPHER TEXT" }));
            ui.add(TextEdit::multiline(&mut self.decode.input).desired_rows(6));
            ui.add_space(18.0);
            render_technique_builder(ui, "decode", &mut self.decode.technique_builder);
            if self.decode.secure {
                ui.add_space(14.0);
                ui.columns(2, |columns| {
                    columns[0].label(section_label("PRIMARY PASSPHRASE"));
                    columns[0].add(TextEdit::singleline(&mut self.decode.passphrase).password(true));
                    columns[1].label(section_label("COMPANION CODE"));
                    columns[1].add(TextEdit::singleline(&mut self.decode.companion_code));
                });
            }
            ui.add_space(16.0);
            if ui.add(accent_button("Decode")).clicked() {
                match self.core.decode(
                    &self.decode.input,
                    &self.decode.technique_builder.chain_string(),
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
                ui.label(section_label("DECODED TEXT"));
                output_box(ui, &result.transformed_text);
            }
        });
    }

    fn render_carrier(&mut self, ui: &mut Ui) {
        panel_title(ui, "Carrier File");
        framed_body(ui, |ui| {
            ui.label(
                RichText::new("Hide one file inside another by appending an extractable SighFar trailer. This is a carrier container, not steganography.")
                    .size(20.0)
                    .monospace()
                    .color(primary_text()),
            );
            ui.add_space(20.0);

            ui.columns(2, |columns| {
                columns[0].label(section_label("CARRIER FILE"));
                path_row(&mut columns[0], &mut self.carrier.carrier_path, FileTarget::File);
                columns[0].add_space(8.0);
                columns[0].label(section_label("PAYLOAD FILE"));
                path_row(&mut columns[0], &mut self.carrier.payload_path, FileTarget::File);
                columns[0].add_space(8.0);
                columns[0].label(section_label("OUTPUT CONTAINER"));
                save_path_row(&mut columns[0], &mut self.carrier.output_path);
                columns[0].add_space(10.0);
                if columns[0].add(accent_button("Hide payload")).clicked() {
                    match embed_file(
                        PathBuf::from(self.carrier.carrier_path.trim()).as_path(),
                        PathBuf::from(self.carrier.payload_path.trim()).as_path(),
                        PathBuf::from(self.carrier.output_path.trim()).as_path(),
                    ) {
                        Ok(result) => {
                            self.carrier.status = format!(
                                "Hidden {} ({} bytes) inside carrier copy {} using a {} byte carrier.",
                                result.payload_name,
                                result.payload_size,
                                result.output_path.display(),
                                result.carrier_size
                            );
                        }
                        Err(err) => {
                            self.carrier.status = err.to_string();
                        }
                    }
                }

                columns[1].label(section_label("CARRIER CONTAINER"));
                path_row(&mut columns[1], &mut self.carrier.extract_container_path, FileTarget::File);
                columns[1].add_space(8.0);
                columns[1].label(section_label("EXTRACTION FOLDER"));
                path_row(&mut columns[1], &mut self.carrier.extract_output_dir, FileTarget::Folder);
                columns[1].add_space(10.0);
                if columns[1].add(accent_button("Extract payload")).clicked() {
                    match extract_file(
                        PathBuf::from(self.carrier.extract_container_path.trim()).as_path(),
                        PathBuf::from(self.carrier.extract_output_dir.trim()).as_path(),
                    ) {
                        Ok(result) => {
                            self.carrier.status = format!(
                                "Extracted {} ({} bytes) to {}.",
                                result.payload_name,
                                result.payload_size,
                                result.extracted_path.display()
                            );
                        }
                        Err(err) => {
                            self.carrier.status = err.to_string();
                        }
                    }
                }
            });
            ui.add_space(12.0);
            status_line(ui, &self.carrier.status);
        });
    }

    fn render_history(&mut self, ui: &mut Ui) {
        panel_title(ui, "Encrypted History");
        framed_body(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ENCRYPTED EVENT LOG").size(18.0).monospace().strong());
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
                        .fill(Color32::from_rgb(29, 29, 29))
                        .stroke(Stroke::new(1.5, Color32::from_rgb(169, 89, 62)))
                        .corner_radius(3.0)
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(title)
                                        .monospace()
                                        .strong()
                                        .color(Color32::from_rgb(255, 194, 92)),
                                );
                                ui.add_space(8.0);
                                ui.label(RichText::new(entry.timestamp.to_rfc3339()).color(secondary_text()));
                            });
                            ui.label(RichText::new(format!("in: {}", entry.input_preview)).color(primary_text()));
                            ui.label(RichText::new(format!("out: {}", entry.output_preview)).color(primary_text()));
                            ui.label(
                                RichText::new(format!(
                                    "chain: {}",
                                    entry.techniques.iter().map(|item| item.title()).collect::<Vec<_>>().join(" -> ")
                                ))
                                .color(primary_text()),
                            );
                            ui.label(
                                RichText::new(format!("secure: {}", if entry.used_secure_envelope { "yes" } else { "no" }))
                                    .color(primary_text()),
                            );
                        });
                    ui.add_space(10.0);
                }
                if self.history.is_empty() {
                    ui.add(Label::new(RichText::new("No encrypted history yet. Use Encode or Decode first.").color(primary_text())));
                }
            });
        });
    }

    fn render_settings(&mut self, ui: &mut Ui) {
        panel_title(ui, "System Settings");
        framed_body(ui, |ui| {
            ui.label(RichText::new("GITHUB SYNC").size(18.0).monospace().strong().color(primary_text()));
            ui.add_space(12.0);
            ui.label(
                RichText::new("USE A GITHUB OAUTH APP CLIENT ID WITH DEVICE FLOW ENABLED. THE APP CAN CREATE A PRIVATE SYNC REPOSITORY AND STORE ONLY THE ENCRYPTED HISTORY BLOB THERE.")
                    .monospace()
                    .color(primary_text()),
            );
            ui.add_space(16.0);
            ui.label(section_label("GITHUB OAUTH CLIENT ID"));
            ui.add(TextEdit::singleline(&mut self.settings.config.github_client_id).hint_text("Iv1.xxxxx"));
            ui.add_space(8.0);
            ui.label(section_label("PRIVATE SYNC REPOSITORY NAME"));
            ui.add(TextEdit::singleline(&mut self.settings.config.github_repo_name).hint_text("sighfar-secure-sync"));

            ui.horizontal(|ui| {
                if ui.add(accent_button("Save settings")).clicked() {
                    match self.config_store.save(&self.settings.config) {
                        Ok(()) => self.settings.sync_status = "Saved local sync settings.".to_string(),
                        Err(err) => self.settings.sync_status = err.to_string(),
                    }
                }
                if ui.add(accent_button("Start sign-in")).clicked() {
                    match self.github_sync.start_device_flow(self.settings.config.github_client_id.trim()) {
                        Ok(device) => {
                            self.settings.device_flow = Some(device.clone());
                            self.settings.sync_status = format!(
                                "Open {} and enter code {}.",
                                device.verification_uri, device.user_code
                            );
                        }
                        Err(err) => self.settings.sync_status = err.to_string(),
                    }
                }
                if ui.add(accent_button("Poll sign-in")).clicked() {
                    self.poll_github_sign_in();
                }
            });

            if let Some(device) = &self.settings.device_flow {
                ui.add_space(10.0);
                ui.columns(2, |columns| {
                    columns[0].label(section_label("VERIFICATION URL"));
                    output_box(&mut columns[0], &device.verification_uri);
                    columns[1].label(section_label("USER CODE"));
                    output_box(&mut columns[1], &device.user_code);
                });
            }

            if let Some(session) = self.settings.signed_in.clone() {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!("Signed in as {}", session.username))
                        .strong()
                        .color(Color32::from_rgb(255, 205, 115)),
                );
                ui.horizontal(|ui| {
                    if ui.add(accent_button("Ensure private repo")).clicked() {
                        match self.github_sync.ensure_private_repo(&session, self.settings.config.github_repo_name.trim()) {
                            Ok(repo) => self.settings.sync_status = format!("Private sync repository ready: {repo}"),
                            Err(err) => self.settings.sync_status = err.to_string(),
                        }
                    }
                    if ui.add(accent_button("Push encrypted history")).clicked() {
                        self.push_history_to_github();
                    }
                    if ui.add(accent_button("Pull encrypted history")).clicked() {
                        self.pull_history_from_github();
                    }
                });
            }

            ui.add_space(10.0);
            status_line(ui, &self.settings.sync_status);
            ui.separator();
            ui.label(section_label("LOCAL ENCRYPTED HISTORY"));
            output_box(ui, &self.core.diagnostics());
            ui.add_space(8.0);
            ui.label(
                RichText::new("The sync repository stores only the encrypted history blob, not the local key file. Another device still needs the key material to decrypt the synced history.")
                    .color(secondary_text()),
            );
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

    fn poll_github_sign_in(&mut self) {
        let Some(device) = self.settings.device_flow.clone() else {
            self.settings.sync_status = "Start sign-in first to get a device code.".to_string();
            return;
        };

        match self
            .github_sync
            .poll_device_flow(self.settings.config.github_client_id.trim(), &device)
        {
            Ok(DevicePollStatus::Authorized(session)) => {
                let username = session.username.clone();
                self.settings.signed_in = Some(session);
                self.settings.sync_status = format!("Signed in as {username}.");
            }
            Ok(DevicePollStatus::Pending(message)) => {
                self.settings.sync_status = message;
            }
            Err(err) => {
                self.settings.sync_status = err.to_string();
            }
        }
    }

    fn push_history_to_github(&mut self) {
        let Some(session) = &self.settings.signed_in else {
            self.settings.sync_status = "Sign in to GitHub first.".to_string();
            return;
        };
        match self.core.export_encrypted_history_blob() {
            Ok(blob) => match self
                .github_sync
                .push_history_blob(session, self.settings.config.github_repo_name.trim(), &blob)
            {
                Ok(()) => {
                    self.settings.sync_status = "Encrypted history pushed to the private sync repository.".to_string();
                }
                Err(err) => {
                    self.settings.sync_status = err.to_string();
                }
            },
            Err(err) => self.settings.sync_status = err.to_string(),
        }
    }

    fn pull_history_from_github(&mut self) {
        let Some(session) = &self.settings.signed_in else {
            self.settings.sync_status = "Sign in to GitHub first.".to_string();
            return;
        };
        match self
            .github_sync
            .pull_history_blob(session, self.settings.config.github_repo_name.trim())
        {
            Ok(blob) => match self.core.import_encrypted_history_blob(&blob) {
                Ok(()) => {
                    self.settings.sync_status = "Encrypted history downloaded from GitHub.".to_string();
                    self.refresh_history();
                }
                Err(err) => self.settings.sync_status = err.to_string(),
            },
            Err(err) => self.settings.sync_status = err.to_string(),
        }
    }
}

fn configure_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    if let Ok(bytes) = fs::read("/System/Library/Fonts/Monaco.ttf") {
        fonts.font_data.insert(
            "monaco".to_string(),
            FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "monaco".to_string());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "monaco".to_string());
    }
    ctx.set_fonts(fonts);
}

fn configure_theme(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgb(7, 6, 10);
    visuals.panel_fill = Color32::from_rgb(15, 14, 18);
    visuals.extreme_bg_color = Color32::from_rgb(5, 5, 5);
    visuals.override_text_color = Some(primary_text());
    visuals.faint_bg_color = Color32::from_rgb(18, 17, 21);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(17, 16, 19);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(180, 103, 81));
    visuals.widgets.noninteractive.fg_stroke.color = primary_text();
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 29, 34);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(14, 14, 18);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.6, Color32::from_rgb(214, 120, 92));
    visuals.widgets.inactive.fg_stroke.color = primary_text();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(96, 41, 30);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(23, 18, 18);
    visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, Color32::from_rgb(255, 174, 104));
    visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(255, 248, 240);
    visuals.widgets.active.bg_fill = Color32::from_rgb(177, 50, 31);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(30, 18, 16);
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, Color32::from_rgb(255, 209, 119));
    visuals.widgets.active.fg_stroke.color = Color32::from_rgb(255, 250, 245);
    visuals.selection.bg_fill = Color32::from_rgb(181, 61, 39);
    visuals.selection.stroke = Stroke::new(1.5, Color32::from_rgb(255, 232, 180));
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(16.0, 16.0);
    style.spacing.button_padding = Vec2::new(20.0, 12.0);
    style.spacing.indent = 14.0;
    style.spacing.interact_size = Vec2::new(52.0, 34.0);
    style.text_styles = [
        (TextStyle::Heading, FontId::new(31.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(20.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(19.0, FontFamily::Monospace)),
        (TextStyle::Monospace, FontId::new(18.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(15.0, FontFamily::Monospace)),
    ]
    .into();
    style.visuals.window_corner_radius = CornerRadius::same(3);
    style.visuals.menu_corner_radius = CornerRadius::same(2);
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(2);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(2);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(2);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(2);
    ctx.set_style(style);
}

fn render_technique_builder(ui: &mut Ui, id: &str, builder: &mut TechniqueBuilder) {
    ui.label(section_label("CIPHER CHAIN"));
    Frame::new()
        .fill(Color32::from_rgb(18, 18, 18))
        .stroke(Stroke::new(1.6, Color32::from_rgb(181, 102, 79)))
        .corner_radius(3.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ComboBox::from_id_salt(format!("{id}_kind"))
                    .selected_text(builder.draft_kind.label())
                    .show_ui(ui, |ui| {
                        for kind in DraftTechniqueKind::all() {
                            ui.selectable_value(&mut builder.draft_kind, kind, kind.label());
                        }
                    });

                match builder.draft_kind {
                    DraftTechniqueKind::Caesar => {
                    ui.label(control_label("SHIFT"));
                    ui.add(egui::DragValue::new(&mut builder.draft_shift).range(-25..=25));
                }
                DraftTechniqueKind::Vigenere => {
                    ui.label(control_label("KEYWORD"));
                    ui.add(TextEdit::singleline(&mut builder.draft_keyword).desired_width(120.0));
                }
                DraftTechniqueKind::RailFence => {
                    ui.label(control_label("RAILS"));
                    ui.add(egui::DragValue::new(&mut builder.draft_rails).range(2..=12));
                }
                    DraftTechniqueKind::Morse | DraftTechniqueKind::Reverse => {}
                }

                if ui.add(accent_button("Add layer")).clicked() {
                    builder.add_current();
                }
                if ui.add(accent_button("Clear")).clicked() {
                    builder.items.clear();
                }
            });

            ui.add_space(10.0);
            if builder.items.is_empty() {
                ui.label(RichText::new("No ciphers selected yet. Add at least one layer.").color(secondary_text()));
            }

            let mut move_up = None;
            let mut move_down = None;
            let mut remove = None;
            for (idx, item) in builder.items.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:02}. {}", idx + 1, item.title()))
                            .monospace()
                            .strong()
                            .color(primary_text()),
                    );
                    if ui.add(secondary_button("UP")).clicked() && idx > 0 {
                        move_up = Some(idx);
                    }
                    if ui.add(secondary_button("DOWN")).clicked() && idx + 1 < builder.items.len() {
                        move_down = Some(idx);
                    }
                    if ui.add(secondary_button("REMOVE")).clicked() {
                        remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = move_up {
                builder.items.swap(idx - 1, idx);
            }
            if let Some(idx) = move_down {
                builder.items.swap(idx, idx + 1);
            }
            if let Some(idx) = remove {
                builder.items.remove(idx);
            }

            ui.add_space(8.0);
            ui.label(section_label("GENERATED CHAIN"));
            output_box(ui, &builder.chain_string());
        });
}

enum FileTarget {
    File,
    Folder,
}

fn path_row(ui: &mut Ui, value: &mut String, target: FileTarget) {
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(value).desired_width(360.0));
        if ui.add(accent_button("BROWSE")).clicked() {
            let selection = match target {
                FileTarget::File => FileDialog::new().pick_file(),
                FileTarget::Folder => FileDialog::new().pick_folder(),
            };
            if let Some(path) = selection {
                *value = path.display().to_string();
            }
        }
    });
}

fn save_path_row(ui: &mut Ui, value: &mut String) {
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(value).desired_width(360.0));
        if ui.add(accent_button("SAVE AS")).clicked() {
            if let Some(path) = FileDialog::new().save_file() {
                *value = path.display().to_string();
            }
        }
    });
}

fn panel_frame(fill: Color32, margin: i8) -> Frame {
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(2.0, Color32::from_rgb(195, 111, 86)))
        .corner_radius(3.0)
        .inner_margin(Margin::same(margin))
}

fn panel_title(ui: &mut Ui, title: &str) {
    Frame::new()
        .fill(Color32::from_rgb(152, 34, 22))
        .stroke(Stroke::new(2.0, Color32::from_rgb(255, 167, 113)))
        .corner_radius(2.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .size(28.0)
                        .monospace()
                        .strong()
                        .color(Color32::from_rgb(255, 248, 243)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("[ ] [ ] [ ]")
                            .size(17.0)
                            .monospace()
                            .color(Color32::from_rgb(255, 208, 179)),
                    );
                });
            });
        });
    ui.add_space(12.0);
}

fn framed_body(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    panel_frame(Color32::from_rgb(20, 18, 22), 22).show(ui, add_contents);
}

fn accent_button(label: &str) -> Button<'static> {
    Button::new(
        RichText::new(label)
            .size(18.0)
            .monospace()
            .strong()
            .color(Color32::from_rgb(255, 246, 238)),
    )
        .fill(Color32::from_rgb(166, 49, 31))
        .stroke(Stroke::new(2.0, Color32::from_rgb(255, 195, 133)))
        .corner_radius(2.0)
        .min_size(Vec2::new(96.0, 34.0))
}

fn secondary_button(label: &str) -> Button<'static> {
    Button::new(
        RichText::new(label)
            .size(14.0)
            .monospace()
            .strong()
            .color(Color32::from_rgb(247, 240, 233)),
    )
    .fill(Color32::from_rgb(66, 66, 72))
    .stroke(Stroke::new(1.6, Color32::from_rgb(188, 118, 96)))
    .corner_radius(2.0)
    .min_size(Vec2::new(76.0, 28.0))
}

fn output_box(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(Color32::from_rgb(10, 9, 13))
        .stroke(Stroke::new(1.4, Color32::from_rgb(187, 110, 87)))
        .corner_radius(2.0)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().color(Color32::from_rgb(252, 247, 242)));
        });
}

fn status_line(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .color(Color32::from_rgb(255, 208, 167)),
    );
}

fn section_label(text: &str) -> RichText {
    RichText::new(text)
        .monospace()
        .strong()
        .color(Color32::from_rgb(241, 235, 229))
}

fn control_label(text: &str) -> RichText {
    RichText::new(text)
        .monospace()
        .color(Color32::from_rgb(227, 218, 209))
}

fn primary_text() -> Color32 {
    Color32::from_rgb(248, 244, 239)
}

fn secondary_text() -> Color32 {
    Color32::from_rgb(226, 214, 205)
}
