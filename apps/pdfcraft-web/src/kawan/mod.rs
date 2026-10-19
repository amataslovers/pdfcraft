//! KawanEditor host bridge (`?host=kawan`): the page embedding PdfCraft drives it through the
//! `kawan:*` postMessage protocol. `protocol.rs` and `web.rs` are verbatim copies of the
//! kawan-editor canonical bridge; `host.rs` adapts PdfCraft to it and `shell.rs` hosts the app.

pub mod host;
pub mod protocol;
pub mod shell;
pub mod web;

/// The canonical `policy.rs`, carried by the engine crate so its hooks can read it.
pub use pdfcraft_engine::kawan as policy;

use pdfcraft_ui_egui::PdfCraftApp;

/// Route the app's saves through the bridge, wake the app when the page sends a message, and tell
/// the page the bridge is listening.
pub fn attach(app: &mut PdfCraftApp, bridge: &web::Bridge, ctx: &egui::Context) {
    let outbox = bridge.outbox();
    app.host_save = Some(Box::new(move |name: &str, bytes: &[u8], _save_as: bool| {
        outbox.borrow_mut().push(protocol::OutFile { name: name.to_string(), bytes: bytes.to_vec(), export: false });
        Ok(())
    }));
    // Everything else PdfCraft would download (exports, split/combine results) is a copy.
    let outbox = bridge.outbox();
    pdfcraft_ui_egui::set_download_hook(Box::new(move |name: &str, bytes: &[u8]| {
        outbox.borrow_mut().push(protocol::OutFile { name: name.to_string(), bytes: bytes.to_vec(), export: true });
    }));
    let ctx = ctx.clone();
    bridge.set_repaint(Box::new(move || ctx.request_repaint()));
    bridge.post_ready("pdf", env!("CARGO_PKG_VERSION"));
}
