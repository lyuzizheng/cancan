use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use std::{ffi::c_void, io};

// Keep one page at or below 1,920,000 RGBA pixels (7.68 MB raw) before PNG encoding and IPC,
// while allowing ordinary statement pages to render at up to 2x their PDF point dimensions.
const MAX_RENDER_WIDTH: f64 = 1_200.0;
const MAX_RENDER_HEIGHT: f64 = 1_600.0;
const MAX_RENDER_SCALE: f64 = 2.0;

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenderedDocumentPage {
    pub(crate) page_count: u32,
    pub(crate) page_number: u32,
    pub(crate) png_base64: String,
}

pub(crate) fn render_pdf_page(pdf: &[u8], page_number: u32) -> io::Result<RenderedDocumentPage> {
    let rendered = render_pdf_page_pixels(pdf, page_number)?;
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, rendered.width, rendered.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| io::Error::other("PDF page PNG header failed"))?;
        writer
            .write_image_data(&rendered.pixels)
            .map_err(|_| io::Error::other("PDF page PNG encoding failed"))?;
    }
    Ok(RenderedDocumentPage {
        page_count: rendered.page_count,
        page_number,
        png_base64: STANDARD.encode(png),
    })
}

struct RenderedPixels {
    height: u32,
    page_count: u32,
    pixels: Vec<u8>,
    width: u32,
}

#[cfg(target_os = "macos")]
fn render_pdf_page_pixels(pdf: &[u8], page_number: u32) -> io::Result<RenderedPixels> {
    if page_number == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PDF pages are one-indexed",
        ));
    }
    let provider = unsafe {
        CGDataProviderCreateWithData(std::ptr::null_mut(), pdf.as_ptr().cast(), pdf.len(), None)
    };
    if provider.is_null() {
        return Err(io::Error::other("Core Graphics data provider failed"));
    }
    let document = unsafe { CGPDFDocumentCreateWithProvider(provider) };
    if document.is_null() {
        unsafe { CGDataProviderRelease(provider) };
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Core Graphics rejected the in-memory PDF",
        ));
    }

    let page_count = unsafe { CGPDFDocumentGetNumberOfPages(document) };
    let requested_page = usize::try_from(page_number)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid PDF page"))?;
    if requested_page > page_count {
        unsafe {
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PDF page is out of range",
        ));
    }
    let page = unsafe { CGPDFDocumentGetPage(document, requested_page) };
    if page.is_null() {
        unsafe {
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Core Graphics could not load the PDF page",
        ));
    }

    let page_box = unsafe { CGPDFPageGetBoxRect(page, PDF_CROP_BOX) };
    let page_width = page_box.size.width.abs();
    let page_height = page_box.size.height.abs();
    if !page_width.is_finite()
        || !page_height.is_finite()
        || page_width <= 0.0
        || page_height <= 0.0
    {
        unsafe {
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PDF page has invalid dimensions",
        ));
    }
    let scale = MAX_RENDER_SCALE
        .min(MAX_RENDER_WIDTH / page_width)
        .min(MAX_RENDER_HEIGHT / page_height);
    let width = (page_width * scale).round().max(1.0) as u32;
    let height = (page_height * scale).round().max(1.0) as u32;
    let bytes_per_row = usize::try_from(width)
        .ok()
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| io::Error::other("PDF render dimensions overflow"))?;
    let pixel_len = usize::try_from(height)
        .ok()
        .and_then(|value| value.checked_mul(bytes_per_row))
        .ok_or_else(|| io::Error::other("PDF render dimensions overflow"))?;
    let color_space = unsafe { CGColorSpaceCreateDeviceRGB() };
    if color_space.is_null() {
        unsafe {
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::other("Core Graphics color space failed"));
    }
    let mut pixels = vec![0xff_u8; pixel_len];
    let context = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            width as usize,
            height as usize,
            8,
            bytes_per_row,
            color_space,
            ALPHA_PREMULTIPLIED_LAST,
        )
    };
    if context.is_null() {
        unsafe {
            CGColorSpaceRelease(color_space);
            CGPDFDocumentRelease(document);
            CGDataProviderRelease(provider);
        }
        return Err(io::Error::other("Core Graphics bitmap context failed"));
    }

    let target = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: f64::from(width),
            height: f64::from(height),
        },
    };
    let transform = unsafe { CGPDFPageGetDrawingTransform(page, PDF_CROP_BOX, target, 0, true) };
    unsafe {
        CGContextConcatCTM(context, transform);
        CGContextDrawPDFPage(context, page);
        CGContextRelease(context);
        CGColorSpaceRelease(color_space);
        CGPDFDocumentRelease(document);
        CGDataProviderRelease(provider);
    }
    for row in 0..usize::try_from(height).expect("height fits usize") / 2 {
        let opposite = usize::try_from(height).expect("height fits usize") - row - 1;
        let (before_opposite, opposite_and_after) = pixels.split_at_mut(opposite * bytes_per_row);
        before_opposite[row * bytes_per_row..(row + 1) * bytes_per_row]
            .swap_with_slice(&mut opposite_and_after[..bytes_per_row]);
    }

    Ok(RenderedPixels {
        height,
        page_count: u32::try_from(page_count)
            .map_err(|_| io::Error::other("PDF has too many pages"))?,
        pixels,
        width,
    })
}

#[cfg(not(target_os = "macos"))]
fn render_pdf_page_pixels(_pdf: &[u8], _page_number: u32) -> io::Result<RenderedPixels> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "the Phase 1 viewer requires macOS",
    ))
}

#[cfg(target_os = "macos")]
const ALPHA_PREMULTIPLIED_LAST: u32 = 1;
#[cfg(target_os = "macos")]
const PDF_CROP_BOX: i32 = 1;

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGAffineTransform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDataProviderCreateWithData(
        info: *mut c_void,
        data: *const c_void,
        size: usize,
        release_data: Option<unsafe extern "C" fn(*mut c_void, *const c_void, usize)>,
    ) -> *mut c_void;
    fn CGDataProviderRelease(provider: *mut c_void);
    fn CGPDFDocumentCreateWithProvider(provider: *mut c_void) -> *mut c_void;
    fn CGPDFDocumentRelease(document: *mut c_void);
    fn CGPDFDocumentGetNumberOfPages(document: *mut c_void) -> usize;
    fn CGPDFDocumentGetPage(document: *mut c_void, page: usize) -> *mut c_void;
    fn CGPDFPageGetBoxRect(page: *mut c_void, box_kind: i32) -> CGRect;
    fn CGPDFPageGetDrawingTransform(
        page: *mut c_void,
        box_kind: i32,
        rect: CGRect,
        rotate: i32,
        preserve_aspect_ratio: bool,
    ) -> CGAffineTransform;
    fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    fn CGColorSpaceRelease(color_space: *mut c_void);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        color_space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    fn CGContextConcatCTM(context: *mut c_void, transform: CGAffineTransform);
    fn CGContextDrawPDFPage(context: *mut c_void, page: *mut c_void);
    fn CGContextRelease(context: *mut c_void);
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::io::{Cursor, Write as _};

    #[test]
    fn renders_requested_pdf_pages_as_png_without_returning_source_bytes() {
        let pdf = synthetic_pdf(2);
        let rendered = render_pdf_page(&pdf, 2).expect("render second page");
        let png = STANDARD
            .decode(&rendered.png_base64)
            .expect("decode rendered PNG");

        assert_eq!(rendered.page_count, 2);
        assert_eq!(rendered.page_number, 2);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(!png.windows(5).any(|bytes| bytes == b"%PDF-"));
    }

    #[test]
    fn rejects_invalid_pdf_and_page_requests() {
        let pdf = synthetic_pdf(1);
        assert_eq!(
            render_pdf_page(&pdf, 0)
                .expect_err("reject page zero")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            render_pdf_page(&pdf, 2)
                .expect_err("reject out-of-range page")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            render_pdf_page(b"not a PDF", 1)
                .expect_err("reject invalid PDF")
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn caps_oversized_pdf_pages_to_the_renderer_pixel_budget() {
        let pdf = synthetic_pdf_with_media_box(1, 10_000, 20_000);
        let rendered = render_pdf_page(&pdf, 1).expect("render oversized page");
        let png = STANDARD
            .decode(&rendered.png_base64)
            .expect("decode rendered PNG");
        let decoder = png::Decoder::new(Cursor::new(png));
        let reader = decoder.read_info().expect("read rendered PNG info");
        let info = reader.info();

        assert!(info.width <= MAX_RENDER_WIDTH as u32);
        assert!(info.height <= MAX_RENDER_HEIGHT as u32);
        assert!(u64::from(info.width) * u64::from(info.height) <= 1_920_000);
    }

    fn synthetic_pdf(page_count: usize) -> Vec<u8> {
        synthetic_pdf_with_media_box(page_count, 64, 96)
    }

    fn synthetic_pdf_with_media_box(page_count: usize, width: u32, height: u32) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {page_count} >>",
                (0..page_count)
                    .map(|index| format!("{} 0 R", 3 + index * 2))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        for index in 0..page_count {
            let page_object = 3 + index * 2;
            let content_object = page_object + 1;
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << >> /Contents {content_object} 0 R >>"
            ));
            objects.push(format!(
                "<< /Length 23 >>\nstream\n0 0 0 rg 0 0 {width} {height} re f\nendstream"
            ));
        }
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object)
                .expect("write PDF object");
        }
        let xref = pdf.len();
        write!(
            &mut pdf,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len() + 1
        )
        .expect("write xref");
        for offset in offsets {
            writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write xref entry");
        }
        write!(
            &mut pdf,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .expect("write trailer");
        pdf
    }
}
