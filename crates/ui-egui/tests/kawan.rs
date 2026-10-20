//! KawanEditor branding and host theming: no ArtCraft mark on screen, the KawanEditor logo in
//! the title bar (the host's own when it sends one, a link when it gives an address), and the
//! host accent in egui's visuals.

use std::rc::Rc;

use egui::Color32;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::kawan::{self, Policy};
use pdfcraft_ui_egui::{Dialog, PdfCraftApp};

const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";

fn harness(setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

fn with_document(app: &mut PdfCraftApp) {
    app.open_bytes("one.pdf", None, PDF.to_vec()).unwrap();
}

/// No ArtCraft mark and no community link; the attribution sentence of the About line is allowed.
/// A `Role::Label` node (`ui.label`) keeps its text in `value`, every other node in `label`.
fn no_artcraft(h: &Harness<'static, PdfCraftApp>, place: &str) {
    let mark = h
        .query_all_by(|n| {
            [n.label(), n.value()]
                .into_iter()
                .flatten()
                .any(|l| (l.to_ascii_lowercase().contains("artcraft") && !l.contains("by the ArtCraft team")) || l.contains("Discord"))
        })
        .next();
    assert!(mark.is_none(), "{place}: {mark:?}");
}

/// The topmost node labelled `label` (`get_all`: the home card, About and the title bar can all
/// carry one).
fn topmost<'a>(h: &'a Harness<'static, PdfCraftApp>, label: &'a str) -> egui_kittest::Node<'a> {
    h.get_all_by_label(label).min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).expect("get_all_by_label found one")
}

/// A 4 × 4 opaque red PNG.
fn red_png() -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, 4, 4);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().unwrap();
    writer.write_image_data(&[255, 0, 0, 255].repeat(16)).unwrap();
    writer.finish().unwrap();
    out
}

#[test]
fn no_artcraft_label_in_home_and_about() {
    kawan::set(Policy::default());
    let mut h = harness(|_| {});
    no_artcraft(&h, "home");
    assert!(h.query_all_by_label("Welcome to KawanEditor").next().is_some());
    assert!(h.query_all_by_label("KawanEditor").count() >= 2, "the home card's logo and heading");
    assert!(h.state_mut().execute("help.about"));
    h.run_steps(4);
    assert_eq!(h.state().dialog, Some(Dialog::About));
    no_artcraft(&h, "About");
    let about = kawan::about("PdfCraft");
    assert!(h.query_all_by_label(&about).next().is_some(), "the About line {about:?}");
    assert!(pdfcraft_engine::commands::COMMANDS.iter().all(|c| !c.label.contains("ArtCraft")));
    assert!(pdfcraft_engine::links::LINKS.iter().all(|l| !l.url.contains("artcraft")));
}

#[test]
fn header_logo_is_drawn() {
    kawan::set(Policy::default());
    let h = harness(with_document);
    let logo = topmost(&h, "KawanEditor").rect();
    let open = topmost(&h, "Open").rect();
    assert!(logo.top() < open.bottom() && open.top() < logo.bottom(), "in the title bar, beside Open: {logo:?} {open:?}");
    assert!(h.query_all_by_label("Discord").next().is_none());
}

#[test]
fn host_logo_with_href_opens_it() {
    kawan::set(Policy { logo_png: Some(Rc::from(red_png())), logo_href: Some("https://example.com/".into()), ..Policy::default() });
    let mut h = harness(with_document);
    topmost(&h, "KawanEditor").click();
    let mut opened = None;
    for _ in 0..4 {
        h.step();
        opened = opened.or_else(|| {
            h.output().platform_output.commands.iter().find_map(|c| match c {
                egui::OutputCommand::OpenUrl(open) => Some(open.url.clone()),
                _ => None,
            })
        });
    }
    assert_eq!(opened.as_deref(), Some("https://example.com/"));
    let logo = pdfcraft_ui_egui::kawan_brand::logo_texture(&h.ctx).map(|t| t.size());
    assert_eq!(logo, Some([4, 4]), "the host's PNG, not the default");
    kawan::set(Policy::default());
}

#[test]
fn host_logo_without_href_is_not_a_link() {
    kawan::set(Policy { logo_png: Some(Rc::from(red_png())), ..Policy::default() });
    let mut h = harness(with_document);
    topmost(&h, "KawanEditor").click();
    for _ in 0..4 {
        h.step();
        assert!(!h.output().platform_output.commands.iter().any(|c| matches!(c, egui::OutputCommand::OpenUrl(_))));
    }
    kawan::set(Policy::default());
}

fn selection_stroke(h: &Harness<'static, PdfCraftApp>) -> Color32 {
    h.ctx.global_style().visuals.selection.stroke.color
}

#[test]
fn host_accent_reaches_visuals() {
    kawan::set(Policy::default());
    let h = harness(|_| {});
    assert_eq!(selection_stroke(&h), Color32::from_rgb(0x1B, 0x63, 0xE0), "PdfCraft's own accent without a host accent");
    drop(h);

    let accent = Color32::from_rgb(0xE1, 0x1D, 0x48);
    kawan::set(Policy { accent: Some([0xE1, 0x1D, 0x48]), ..Policy::default() });
    let mut h = harness(|_| {});
    assert_eq!(selection_stroke(&h), accent);
    assert_eq!(h.ctx.global_style().visuals.hyperlink_color, Color32::from_rgb(191, 25, 61), "accent_text, light");
    // What the bridge does on init: the host's theme, which applies the accent again.
    h.state_mut().set_option("theme", "dark").unwrap();
    h.run_steps(4);
    assert_eq!(selection_stroke(&h), accent);
    assert_eq!(h.ctx.global_style().visuals.hyperlink_color, Color32::from_rgb(236, 108, 136), "accent_text, dark");
    // The host's theme back to System keeps the accent.
    h.state_mut().set_option("theme", "system").unwrap();
    h.run_steps(4);
    assert_eq!(selection_stroke(&h), accent, "theme system");
    kawan::set(Policy::default());
}
