//! KawanEditor branding (logo, About line, identity labels) and the host's accent colour.
//! PdfCraft's own UI calls these at a few hook points; everything here reads the host policy
//! (`pdfcraft_engine::kawan`) and falls back to KawanEditor's defaults and PdfCraft's own colours.

use egui::{Color32, Sense, vec2};
use pdfcraft_engine::kawan as policy;

use crate::theme::Tokens;

/// The default logo: KawanEditor's mark, 256 × 256.
const DEFAULT_LOGO: &[u8] = include_bytes!("../assets/kawan-logo.png");
/// Larger host logos are refused (a small PNG can inflate to a huge bitmap).
const MAX_SIDE: u32 = 2048;

/// Decode a PNG into an egui image; `None` for anything that is not a PNG of a sane size.
pub fn decode_png(bytes: &[u8]) -> Option<egui::ColorImage> {
    if !policy::is_png(bytes) {
        return None;
    }
    let limits = png::Limits { bytes: 64 * 1024 * 1024 };
    let mut decoder = png::Decoder::new_with_limits(std::io::Cursor::new(bytes), limits);
    decoder.set_transformations(png::Transformations::ALPHA | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = reader.info().size();
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
        return None;
    }
    // At most MAX_SIDE² × 4 bytes after the size check (and the decoder's own limit).
    let mut buf = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buf).ok()?;
    let data = buf.get(..frame.buffer_size())?;
    // `ALPHA` turns every PNG into one of these two.
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::GrayscaleAlpha => data.chunks_exact(2).flat_map(|p| if let &[g, a] = p { [g, g, g, a] } else { [0; 4] }).collect(),
        _ => return None,
    };
    let size = [usize::try_from(w).ok()?, usize::try_from(h).ok()?];
    (rgba.len() == size[0].checked_mul(size[1])?.checked_mul(4)?).then(|| egui::ColorImage::from_rgba_unmultiplied(size, &rgba))
}

/// The logo texture: the host's PNG when it sent one that decodes, otherwise KawanEditor's. One
/// texture, kept with the policy generation it was built for and replaced when a new policy comes.
pub fn logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("kawan-logo");
    let generation = policy::generation();
    if let Some((built, texture)) = ctx.data(|d| d.get_temp::<(u64, egui::TextureHandle)>(id))
        && built == generation
    {
        return Some(texture);
    }
    let image = policy::logo_png().and_then(|png| decode_png(&png)).or_else(|| decode_png(DEFAULT_LOGO))?;
    let texture = ctx.load_texture("kawan-logo", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, (generation, texture.clone())));
    Some(texture)
}

/// The logo, `size` points square, labelled "KawanEditor". With a host `href` it is a link that
/// opens in a new tab.
pub fn logo(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let href = policy::logo_href();
    let sense = if href.is_some() { Sense::click() } else { Sense::hover() };
    let response = match logo_texture(ui.ctx()) {
        Some(texture) => ui.add(egui::Image::from_texture(&texture).fit_to_exact_size(vec2(size, size)).alt_text(policy::PRODUCT).sense(sense)),
        None => {
            let response = ui.allocate_response(vec2(size, size), sense);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, policy::PRODUCT));
            response
        }
    };
    match href {
        Some(href) => {
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(href));
            }
            response
        }
        None => response,
    }
}

/// The small logo at the end of the title bar, with a little space before it.
pub fn header_logo(ui: &mut egui::Ui) -> egui::Response {
    ui.add_space(4.0);
    logo(ui, 18.0)
}

/// The About dialog's version and credit line.
pub fn about() -> String {
    policy::about("PdfCraft")
}

/// PdfCraft labels that name the product itself; they name KawanEditor instead.
const IDENTITY: [&str; 2] = ["About PdfCraft", "Welcome to PdfCraft"];

/// `english` untranslated, an identity label naming KawanEditor: the host's command catalogue
/// (`listCommands`) is English.
pub fn identity(english: &str) -> String {
    if IDENTITY.contains(&english) { english.replace("PdfCraft", policy::PRODUCT) } else { english.to_owned() }
}

/// `english` in the current language; an identity label names KawanEditor (in every language,
/// since each catalogue keeps the name "PdfCraft" as is).
pub fn label(english: &str) -> String {
    let translated = crate::i18n::t(english);
    if IDENTITY.contains(&english) { translated.replace("PdfCraft", policy::PRODUCT) } else { translated.to_owned() }
}

/// The host accent in place of PdfCraft's colour `upstream`, with `alpha` (unmultiplied).
pub fn accent_or(upstream: [u8; 3], alpha: u8) -> Color32 {
    let [r, g, b] = policy::accent().unwrap_or(upstream);
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// PdfCraft's tokens with the host accent: the accent itself, its text colour on panels and
/// the soft and selected washes, blended from the panel colour. Unchanged without an accent.
pub fn tint(t: Tokens) -> Tokens {
    match policy::accent() {
        Some(a) => tint_with(t, a),
        None => t,
    }
}

fn tint_with(mut t: Tokens, a: [u8; 3]) -> Tokens {
    let s = [t.panel.r(), t.panel.g(), t.panel.b()];
    let (text, soft, selected) = if t.dark() {
        (policy::mix(a, [255; 3], 0.35), policy::mix(s, a, 0.25), policy::mix(s, a, 0.22))
    } else {
        (policy::mix(a, [0; 3], 0.15), policy::mix(s, a, 0.12), policy::mix(s, a, 0.10))
    };
    t.accent = rgb(a);
    t.badge_new = rgb(a);
    t.accent_text = rgb(text);
    t.accent_soft = rgb(soft);
    t.selected = rgb(selected);
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeKind;
    use policy::Policy;

    fn with_accent(accent: Option<[u8; 3]>) {
        policy::set(Policy { accent, ..Policy::default() });
    }

    #[test]
    fn bad_host_png_falls_back_to_default() {
        assert!(decode_png(b"nope").is_none());
        let mut truncated = DEFAULT_LOGO.to_vec();
        truncated.truncate(40);
        assert!(decode_png(&truncated).is_none(), "a PNG signature alone is not enough");
        let default = decode_png(DEFAULT_LOGO).map(|i| i.size);
        assert_eq!(default, Some([256, 256]));
    }

    #[test]
    fn identity_names_kawan_editor_without_translating() {
        crate::i18n::set_current(crate::i18n::Lang::from_pref("de"));
        assert_eq!(identity("About PdfCraft"), "About KawanEditor");
        assert_eq!(identity("Welcome to PdfCraft"), "Welcome to KawanEditor");
        assert_eq!(identity("PdfCraft on GitHub"), "PdfCraft on GitHub", "the upstream source keeps its name");
        assert_eq!(identity("Open"), "Open", "never translated");
        crate::i18n::set_current(crate::i18n::Lang::from_pref("en"));
    }

    /// `w` x `h` pixels of `color` at `depth`, every byte `fill`; `palette` for indexed colour.
    fn encode(w: u32, h: u32, color: png::ColorType, depth: png::BitDepth, fill: u8, palette: Option<Vec<u8>>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(color);
        enc.set_depth(depth);
        if let Some(p) = palette {
            enc.set_palette(p);
        }
        let mut writer = enc.write_header().unwrap();
        let row = (w as usize * color.samples() * depth as usize).div_ceil(8);
        writer.write_image_data(&vec![fill; row * h as usize]).unwrap();
        writer.finish().unwrap();
        out
    }

    fn first_pixel(bytes: &[u8]) -> Option<([usize; 2], egui::Color32)> {
        decode_png(bytes).and_then(|i| Some((i.size, *i.pixels.first()?)))
    }

    #[test]
    fn png_variants_decode_to_rgba() {
        use png::{BitDepth, ColorType};
        let grey = encode(3, 2, ColorType::Grayscale, BitDepth::Eight, 0x80, None);
        assert_eq!(first_pixel(&grey), Some(([3, 2], egui::Color32::from_gray(0x80))), "grey");
        let grey_alpha = encode(2, 2, ColorType::GrayscaleAlpha, BitDepth::Eight, 0xFF, None);
        assert_eq!(first_pixel(&grey_alpha), Some(([2, 2], egui::Color32::WHITE)), "grey + alpha");
        let palette = encode(2, 2, ColorType::Indexed, BitDepth::Eight, 0, Some(vec![0xE1, 0x1D, 0x48]));
        assert_eq!(first_pixel(&palette), Some(([2, 2], egui::Color32::from_rgb(0xE1, 0x1D, 0x48))), "palette");
        let deep = encode(2, 2, ColorType::Rgb, BitDepth::Sixteen, 0xFF, None);
        assert_eq!(first_pixel(&deep), Some(([2, 2], egui::Color32::WHITE)), "16-bit");
    }

    #[test]
    fn oversized_png_is_refused() {
        let wide = encode(MAX_SIDE + 1, 1, png::ColorType::Grayscale, png::BitDepth::One, 0, None);
        assert!(decode_png(&wide).is_none(), "wider than {MAX_SIDE}");
        let tall = encode(1, MAX_SIDE + 1, png::ColorType::Grayscale, png::BitDepth::One, 0, None);
        assert!(decode_png(&tall).is_none(), "taller than {MAX_SIDE}");
        let edge = encode(MAX_SIDE, 1, png::ColorType::Grayscale, png::BitDepth::One, 0, None);
        assert_eq!(decode_png(&edge).map(|i| i.size), Some([MAX_SIDE as usize, 1]));
    }

    #[test]
    fn logo_texture_follows_the_policy() {
        let ctx = egui::Context::default();
        let red = encode(4, 4, png::ColorType::Rgba, png::BitDepth::Eight, 0xFF, None);
        policy::set(policy::Policy { logo_png: Some(red.clone().into()), ..policy::Policy::default() });
        assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([4, 4]));
        let first = logo_texture(&ctx).map(|t| t.id());
        assert_eq!(logo_texture(&ctx).map(|t| t.id()), first, "cached while the policy stays");
        let textures = ctx.tex_manager().read().num_allocated();
        policy::set(policy::Policy { logo_png: Some(b"nope".to_vec().into()), ..policy::Policy::default() });
        assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([256, 256]), "a bad host PNG falls back");
        assert_ne!(logo_texture(&ctx).map(|t| t.id()), first, "a new policy, a new texture");
        for _ in 0..3 {
            policy::set(policy::Policy { logo_png: Some(red.clone().into()), ..policy::Policy::default() });
            assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([4, 4]));
        }
        assert_eq!(ctx.tex_manager().read().num_allocated(), textures, "the old logo is freed, not kept per generation");
        policy::set(policy::Policy::default());
    }

    #[test]
    fn identity_labels_name_kawan_editor() {
        crate::i18n::set_current(crate::i18n::Lang::from_pref("en"));
        assert_eq!(label("About PdfCraft"), "About KawanEditor");
        assert_eq!(label("Welcome to PdfCraft"), "Welcome to KawanEditor");
        assert_eq!(label("PdfCraft on GitHub"), "PdfCraft on GitHub", "the upstream source keeps its name");
        crate::i18n::set_current(crate::i18n::Lang::from_pref("de"));
        assert_eq!(label("About PdfCraft"), "Über KawanEditor");
        crate::i18n::set_current(crate::i18n::Lang::from_pref("en"));
    }

    #[test]
    fn tint_is_identity_without_accent() {
        with_accent(None);
        for kind in [ThemeKind::Light, ThemeKind::Dark] {
            let t = Tokens::for_kind(kind);
            assert_eq!(format!("{:?}", tint(t)), format!("{t:?}"));
        }
        assert_eq!(accent_or([0x1B, 0x63, 0xE0], 255), Color32::from_rgb(0x1B, 0x63, 0xE0));
    }

    #[test]
    fn tint_replaces_the_accent_family() {
        let a = [0xE1, 0x1D, 0x48];
        with_accent(Some(a));
        let light = tint(Tokens::for_kind(ThemeKind::Light));
        assert_eq!(light.accent, rgb(a));
        assert_eq!(light.badge_new, rgb(a));
        assert_eq!(light.accent_text, rgb([191, 25, 61]));
        assert_eq!(light.accent_soft, rgb([251, 228, 233]));
        assert_eq!(light.selected, rgb([252, 232, 237]));
        let dark = tint(Tokens::for_kind(ThemeKind::Dark));
        assert_eq!(dark.accent, rgb(a));
        assert_eq!(dark.badge_new, rgb(a));
        assert_eq!(dark.accent_text, rgb([236, 108, 136]));
        assert_eq!(dark.accent_soft, rgb([85, 36, 47]));
        assert_eq!(dark.selected, rgb([79, 36, 45]));
        // Everything else stays PdfCraft's.
        assert_eq!(dark.panel, Tokens::for_kind(ThemeKind::Dark).panel);
        assert_eq!(accent_or([0x1B, 0x63, 0xE0], 255), rgb(a));
        with_accent(None);
    }
}
