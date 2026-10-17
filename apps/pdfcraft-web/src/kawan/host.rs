//! PdfCraft as a KawanEditor bridge host.

use pdfcraft_engine::commands::{COMMANDS, Needs};
use pdfcraft_ui_egui::{PdfCraftApp, SaveTarget};
use serde_json::Value;

use super::protocol::{Classify, CommandInfo, Host};
use super::web::Outbox;

/// The app, and the bridge outbox its `host_save` hook writes into.
pub struct PdfHost<'a>(pub &'a mut PdfCraftApp, pub Outbox);

fn pdf_name(name: &str) -> String {
    if name.to_ascii_lowercase().ends_with(".pdf") { name.to_string() } else { format!("{name}.pdf") }
}

/// Commands that change the document; anything new upstream counts as changing (safer for view mode).
fn mutates(needs: Needs) -> bool {
    !matches!(needs, Needs::Nothing | Needs::Document | Needs::Copying | Needs::Undo | Needs::Redo | Needs::TwoPageView)
}

impl Host for PdfHost<'_> {
    fn open(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.0.open_bytes(name, None, bytes)
    }

    fn save(&mut self) -> Result<(String, Vec<u8>), String> {
        self.0.active_ids().ok_or("no document")?;
        // PdfCraft's own Save (flattens fill & sign, marks saved), which hands its bytes to `host_save`.
        if !self.0.save_active(SaveTarget::InPlace) {
            return Err("PdfCraft could not save the document".into());
        }
        let file = self.1.borrow_mut().pop().ok_or("PdfCraft saved nothing")?;
        Ok((pdf_name(&file.name), file.bytes))
    }

    fn export_copy(&mut self) -> Result<(String, Vec<u8>), String> {
        // ponytail: the current bytes without Save's fill & sign flattening; a download copy only.
        let (_, id) = self.0.active_ids().ok_or("no document")?;
        let name = self.0.session.get(id).map(|d| pdf_name(&d.name)).ok_or("no document")?;
        let bytes = self.0.session.save_bytes(id).map_err(|e| e.to_string())?;
        Ok((name, bytes.to_vec()))
    }

    fn exec(&mut self, id: &str, _params: Value) -> Result<Value, String> {
        // PdfCraft commands take no parameters.
        if self.0.execute(id) { Ok(Value::Null) } else { Err(format!("{id} is unknown or not available now")) }
    }

    fn commands(&self) -> Vec<CommandInfo> {
        COMMANDS
            .iter()
            .map(|c| CommandInfo { id: c.id.to_string(), label: c.label.to_string(), menu: c.menu.unwrap_or("").to_string(), mutates: mutates(c.needs) })
            .collect()
    }

    fn dirty(&self) -> bool {
        self.0.first_dirty().is_some()
    }

    fn revision(&self) -> u64 {
        self.0.active_ids().and_then(|(_, id)| self.0.session.get(id)).map_or(0, |d| d.edit_generation())
    }

    fn set_author(&mut self, name: &str) {
        self.0.comment_prefs.author = name.chars().take(200).collect();
    }

    fn classify(&self, _name: &str) -> Classify {
        // Only `host_save` (Save, Save As) reaches here; downloads arrive marked as exports.
        Classify::Save
    }

    fn mark_saved(&mut self) {
        // PdfCraft marked the document saved when `host_save` returned Ok.
    }
}
