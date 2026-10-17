//! Hosts `PdfCraftApp` so the bridge can reach it every frame: forwards every `eframe::App`
//! method PdfCraftApp implements, then lets the bridge handle the page's messages.

use std::rc::Rc;

use pdfcraft_ui_egui::PdfCraftApp;

use super::host::PdfHost;
use super::web::Bridge;

pub struct KawanShell {
    pub app: PdfCraftApp,
    pub bridge: Rc<Bridge>,
}

impl eframe::App for KawanShell {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app.raw_input_hook(ctx, raw);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.app.save(storage);
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
        let outbox = self.bridge.outbox();
        self.bridge.tick(&mut PdfHost(&mut self.app, outbox));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
    }
}
