//! PdfCraft as a KawanEditor bridge host.

use std::cell::Cell;

use pdfcraft_engine::DocId;
use pdfcraft_engine::commands::{COMMANDS, Needs};
use pdfcraft_ui_egui::{PdfCraftApp, SaveTarget, kawan_brand};
use serde_json::Value;

use super::policy::{self, Policy, Theme};
use super::protocol::{Classify, CommandInfo, Host};
use super::web::Outbox;

/// The app, and the bridge outbox its `host_save` hook writes into.
pub struct PdfHost<'a>(pub &'a mut PdfCraftApp, pub Outbox);

thread_local! {
    /// The document the page opened. A user can open more (drop a file, view a signed version);
    /// the bridge only ever saves this one, under the page's key.
    static OPENED: Cell<Option<DocId>> = const { Cell::new(None) };
}

impl PdfHost<'_> {
    /// The opened document's tab index and id, wherever its tab now is.
    fn opened(&self) -> Result<(usize, DocId), String> {
        let id = OPENED.with(Cell::get).ok_or("no document")?;
        let index = self.0.views.iter().position(|v| v.id == id).ok_or("the document was closed")?;
        Ok((index, id))
    }
}

fn pdf_name(name: &str) -> String {
    if name.to_ascii_lowercase().ends_with(".pdf") { name.to_string() } else { format!("{name}.pdf") }
}

/// PdfCraft's `theme` option for the host's colour scheme (`System` follows the browser, live).
fn theme_option(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "system",
        Theme::Light => "light",
        Theme::Dark => "dark",
    }
}

/// Commands that change the document; anything new upstream counts as changing (safer for view mode).
fn mutates(needs: Needs) -> bool {
    !matches!(needs, Needs::Nothing | Needs::Document | Needs::Copying | Needs::Undo | Needs::Redo | Needs::TwoPageView)
}

/// Commands that change the document although their `Needs` says otherwise, or open another one
/// (a save would then hand that one to the page under this document's key),
/// and the user's own saves: view mode saves nothing.
const ALSO_MUTATING: &[&str] = &[
    "edit.undo", "edit.redo", "file.revert", "a11y.autotag", "a11y.reading_options", "sign.*", "optimize.*", "standards.pdfa", "file.open*",
    "file.close*", "create.*", "file.save*",
];

impl Host for PdfHost<'_> {
    fn open(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.0.open_bytes(name, None, bytes)?;
        OPENED.with(|o| o.set(self.0.active_ids().map(|(_, id)| id)));
        Ok(())
    }

    fn save(&mut self) -> Result<(String, Vec<u8>), String> {
        let (index, _) = self.opened()?;
        // PdfCraft's own Save (flattens fill & sign, marks saved), which hands its bytes to `host_save`.
        // The bridge's own save: the host's command deny list does not apply to it.
        if !policy::as_host(|| self.0.save_view(index, SaveTarget::InPlace)) {
            return Err("PdfCraft could not save the document".into());
        }
        let file = self.1.borrow_mut().pop().ok_or("PdfCraft saved nothing")?;
        Ok((pdf_name(&file.name), file.bytes))
    }

    fn export_copy(&mut self) -> Result<(String, Vec<u8>), String> {
        // ponytail: the current bytes without Save's fill & sign flattening; a download copy only.
        let (_, id) = self.opened()?;
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
            .filter(|c| policy::allows(c.id, mutates(c.needs)).is_ok())
            // An identity label ("About PdfCraft") names KawanEditor, as in the menus.
            .map(|c| CommandInfo {
                id: c.id.to_string(),
                label: kawan_brand::identity(c.label),
                menu: c.menu.unwrap_or("").to_string(),
                mutates: mutates(c.needs),
            })
            .collect()
    }

    fn dirty(&self) -> bool {
        self.0.first_dirty().is_some()
    }

    fn revision(&self) -> u64 {
        self.opened().ok().and_then(|(_, id)| self.0.session.get(id)).map_or(0, |d| d.edit_generation())
    }

    fn set_author(&mut self, name: &str) {
        self.0.comment_prefs.author = name.chars().take(200).collect();
    }

    fn set_policy(&mut self, mut p: Policy) {
        p.mutating = COMMANDS.iter().filter(|c| mutates(c.needs)).map(|c| c.id.to_string()).chain(ALSO_MUTATING.iter().map(|s| s.to_string())).collect();
        let (read_only, theme) = (p.read_only, p.theme);
        policy::set(p);
        if read_only {
            // Only the layout: editable() and Session::apply already refuse edits.
            let _ = self.0.set_option("mode", "read");
        }
        // Always, `System` too: applying the theme also applies the host accent.
        let _ = self.0.set_option("theme", theme_option(theme));
    }

    fn classify(&self, _name: &str) -> Classify {
        // Only `host_save` (Save, Save As) reaches here; downloads arrive marked as exports.
        Classify::Save
    }

    fn mark_saved(&mut self) {
        // PdfCraft marked the document saved when `host_save` returned Ok.
    }
}
