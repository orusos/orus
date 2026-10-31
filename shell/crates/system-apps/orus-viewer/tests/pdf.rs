use orus_viewer::pdf::{Renderer, cache_dir};

/// Pages drawn by hayro; the same tests run on pdfium when `ORUS_PDFIUM` names the library.
fn render_page(
    file: &std::path::Path,
    page: usize,
    box_px: u32,
    cache: &std::path::Path,
) -> Result<orus_viewer::pdf::Rendered, String> {
    Renderer::shared().render_page(file, page, box_px, cache)
}

/// A PDF of `sizes` (width, height in points): each page is a black rectangle on white.
fn pdf_with_pages(sizes: &[(u32, u32)]) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    let page_ids: Vec<String> = (0..sizes.len())
        .map(|i| format!("{} 0 R", 3 + i * 2))
        .collect();
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            page_ids.join(" "),
            sizes.len()
        ),
    ];
    for (i, (w, h)) in sizes.iter().enumerate() {
        let content_id = 4 + i * 2;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents {content_id} 0 R >>"
        ));
        let stream = format!("0 0 0 rg 10 10 {} {} re f", w / 2, h / 2);
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", i + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in &offsets {
        out.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    out
}

fn png_size(path: &std::path::Path) -> (u32, u32) {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let be = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn a_page_is_rendered_to_fit_the_box_and_the_document_length_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.pdf");
    std::fs::write(&file, pdf_with_pages(&[(200, 100), (100, 400)])).unwrap();
    let cache = dir.path().join("cache");

    let wide = render_page(&file, 0, 400, &cache).unwrap();
    assert_eq!(wide.page_count, 2);
    assert_eq!(
        png_size(&wide.png),
        (400, 200),
        "the long side fits the box"
    );

    let tall = render_page(&file, 1, 400, &cache).unwrap();
    assert_eq!(png_size(&tall.png), (100, 400));
    assert_ne!(wide.png, tall.png);
}

#[test]
fn a_rendered_page_is_reused_until_the_file_changes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.pdf");
    std::fs::write(&file, pdf_with_pages(&[(200, 100)])).unwrap();
    let cache = dir.path().join("cache");

    let first = render_page(&file, 0, 200, &cache).unwrap().png;
    let stamp = std::fs::metadata(&first).unwrap().modified().unwrap();
    let again = render_page(&file, 0, 200, &cache).unwrap().png;
    assert_eq!(first, again);
    assert_eq!(
        std::fs::metadata(&again).unwrap().modified().unwrap(),
        stamp
    );

    std::fs::write(&file, pdf_with_pages(&[(300, 100), (300, 100)])).unwrap();
    let edited = render_page(&file, 0, 200, &cache).unwrap();
    assert_ne!(edited.png, first, "an edited file is a new cache entry");
    assert_eq!(edited.page_count, 2);
}

#[test]
fn a_missing_page_a_missing_file_and_a_non_pdf_say_what_is_wrong() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let file = dir.path().join("doc.pdf");
    std::fs::write(&file, pdf_with_pages(&[(100, 100)])).unwrap();
    assert!(
        render_page(&file, 5, 200, &cache)
            .err()
            .unwrap()
            .contains("1 pages")
    );
    assert!(
        render_page(&dir.path().join("none.pdf"), 0, 200, &cache)
            .err()
            .unwrap()
            .contains("cannot read")
    );
    let text = dir.path().join("note.pdf");
    std::fs::write(&text, "just text").unwrap();
    assert!(
        render_page(&text, 0, 200, &cache)
            .err()
            .unwrap()
            .contains("cannot open this PDF")
    );
    assert!(cache_dir().ends_with(".orus/cache/viewer"));
}

#[test]
fn the_engine_follows_the_library_that_was_found() {
    let engine = Renderer::shared().engine();
    match std::env::var_os("ORUS_PDFIUM") {
        Some(_) => assert_eq!(engine, "pdfium"),
        None if std::path::Path::new("/usr/lib/orus/libpdfium.so").is_file() => {}
        None => assert_eq!(engine, "hayro"),
    }
}

#[test]
fn a_password_protected_pdf_is_refused_and_pdfium_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let locked = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/locked.pdf");
    let renderer = Renderer::shared();
    let error = renderer
        .render_page(&locked, 0, 200, dir.path())
        .err()
        .expect("a protected file does not render");
    if renderer.engine() == "pdfium" {
        assert!(error.contains("needs a password"), "{error}");
    }
}
