//! KawanEditor policy (`kawan.rs`): deny and read-only reach the command check and the edit
//! chokepoint, and the default policy leaves PdfCraft as upstream.

use std::sync::Arc;

use pdfcraft_engine::commands::{command, is_enabled};
use pdfcraft_engine::kawan::{self, Policy};
use pdfcraft_engine::{DocId, Edit, EditError, Session};

/// A minimal `n`-page document (same shape as the engine's unit-test fixture).
fn fixture(n: usize) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()];
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} /MediaBox [0 0 200 300] >>", kids.join(" ")).into_bytes());
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    for i in 0..n {
        objs.push(format!("<< /Type /Page /Parent 2 0 R /Contents {} 0 R /Resources << /Font << /F1 3 0 R >> >> >>", 5 + 2 * i).into_bytes());
        let body = format!("BT /F1 24 Tf 20 150 Td (Page {}) Tj ET", i + 1);
        objs.push(format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len()).into_bytes());
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

fn session() -> (Session, DocId) {
    let mut s = Session::new().with_clock(|| 1_700_000_000);
    let id = s.open("fixture.pdf", None, Arc::new(fixture(2)), None).unwrap();
    (s, id)
}

fn enabled(s: &Session, id: DocId, cmd: &str) -> bool {
    is_enabled(command(cmd).unwrap(), s, Some(id))
}

fn rotate() -> Edit {
    Edit::RotatePages { pages: vec![0], degrees: 90 }
}

#[test]
fn deny_disables_a_command() {
    let (s, id) = session();
    kawan::set(Policy { deny: vec!["page.rotate".into()], ..Policy::default() });
    assert!(!enabled(&s, id, "page.rotate"));
    assert!(enabled(&s, id, "page.delete"));
    kawan::set(Policy::default());
}

#[test]
fn read_only_blocks_apply_including_style_annotation() {
    let (mut s, id) = session();
    kawan::set(Policy { read_only: true, ..Policy::default() });
    assert!(matches!(s.apply(id, rotate()), Err(EditError::ReadOnly(_))));
    // The comment quick bar's style edit skips the command registry (spec §4.4).
    let style = Edit::StyleAnnotation { page: 0, index: 0, color: Some([1.0, 0.0, 0.0]), opacity: None, width: None, endings: None };
    assert!(matches!(s.apply(id, style), Err(EditError::ReadOnly(_))));
    kawan::set(Policy::default());
}

#[test]
fn read_only_disables_editing_commands_and_keeps_the_bytes() {
    let (mut s, id) = session();
    let before = s.save_bytes(id).unwrap();
    kawan::set(Policy { read_only: true, ..Policy::default() });
    assert!(!enabled(&s, id, "page.rotate"));
    assert!(!s.get(id).unwrap().editable());
    let _ = s.apply(id, rotate());
    assert_eq!(s.save_bytes(id).unwrap(), before);
    kawan::set(Policy::default());
}

#[test]
fn upstream_policy_changes_nothing() {
    let (mut s, id) = session();
    kawan::set(Policy::default());
    assert!(enabled(&s, id, "page.rotate"));
    s.apply(id, rotate()).unwrap();
}
