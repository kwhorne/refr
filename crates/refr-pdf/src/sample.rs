//! "Circular futures": an original, searchable six-page report used as the start document.
//! Its numbers are fictional demonstration data.

use std::sync::Arc;

use pdfium_render::prelude::*;
use refr_core::{PdfWorkspace, RectD};

use crate::draw::{Fonts, Painter};
use crate::frame::Matrix;
use crate::{PdfError, State};

const INK: u32 = 0xFF28332F;
const MUTED: u32 = 0xFF64706A;
const GREEN: u32 = 0xFF537466;
const DEEP: u32 = 0xFF243B31;
const RULE: u32 = 0xFFDCE2DD;
const PANEL: u32 = 0xFFE8EEE5;

struct Page<'p, 'd, 'a> {
    painter: &'p mut Painter<'d, 'a>,
}

impl Page<'_, '_, '_> {
    fn text(&mut self, value: &str, x: f64, y: f64, size: f64, color: u32, width: f64) -> Result<(), PdfiumError> {
        self.painter.text(value, x, y, size, crate::draw::color(color), width, false)
    }

    fn bold(&mut self, value: &str, x: f64, y: f64, size: f64, color: u32) -> Result<(), PdfiumError> {
        self.painter.text(value, x, y, size, crate::draw::color(color), 500.0, true)
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32) -> Result<(), PdfiumError> {
        self.painter.rect(RectD::new(x, y, w, h), None, Some(crate::draw::color(color)))
    }

    fn rule(&mut self, y: f64) -> Result<(), PdfiumError> {
        self.rect(48.0, y, 499.0, 1.0, RULE)
    }

    fn ring(&mut self, cx: f64, cy: f64, radius: f64, width: f64, color: u32) -> Result<(), PdfiumError> {
        let r = RectD::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
        self.painter.ellipse(r, crate::draw::color(color), width)
    }
}

pub(crate) fn create(state: &mut State) -> Result<PdfWorkspace, PdfError> {
    let mut document = state.pdfium.create_new_pdf()?;
    let fonts = Fonts::new(&mut document);
    let size = PdfPagePaperSize::Custom(PdfPoints::new(595.0), PdfPoints::new(842.0));
    // Blank pages have a plain 595×842 box, so logical space is user space flipped vertically.
    let matrix = Matrix { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: 842.0 };
    for index in 0..6 {
        let mut pdf_page = document.pages_mut().create_page_at_end(size)?;
        let mut painter = Painter { document: &document, page: &mut pdf_page, matrix, fonts: &fonts };
        let mut p = Page { painter: &mut painter };
        p.bold("FIELDWORK", 48.0, 34.0, 12.0, INK)?;
        p.text("RESEARCH & IDEAS / 2026", 430.0, 36.0, 8.0, MUTED, 200.0)?;
        p.rule(62.0)?;
        match index {
            0 => {
                p.text("THE CIRCULAR FUTURES REPORT", 48.0, 103.0, 10.0, GREEN, 490.0)?;
                p.bold("Good ideas.\nLasting impact.", 45.0, 137.0, 49.0, DEEP)?;
                p.text("A practical guide to designing a better tomorrow.", 49.0, 279.0, 13.0, 0xFF5B6860, 490.0)?;
                p.rect(48.0, 331.0, 499.0, 332.0, PANEL)?;
                p.ring(302.0, 497.0, 128.0, 60.0, 0xFF466B52)?;
                p.ring(302.0, 497.0, 58.0, 44.0, 0xFFC5D7B8)?;
                p.text("01 — RETHINK WHAT COMES NEXT", 48.0, 703.0, 9.0, 0xFF63766A, 490.0)?;
                p.text("Insights, principles and practical steps for a more\nthoughtful use of our shared resources.", 48.0, 728.0, 12.0, INK, 490.0)?;
            }
            1 => {
                p.text("01 / THE BIG PICTURE", 48.0, 98.0, 10.0, GREEN, 490.0)?;
                p.bold("Less waste.\nMore possibility.", 48.0, 133.0, 38.0, INK)?;
                p.text("A circular approach starts with a simple question: what could we keep in use for longer? This report turns that question into a shared plan for action.", 48.0, 251.0, 13.0, INK, 470.0)?;
                let metrics = [("32%", "LESS MATERIAL"), ("2.4×", "LONGER LIFE"), ("86%", "RECOVERABLE")];
                for (i, (value, label)) in metrics.iter().enumerate() {
                    let x = 48.0 + i as f64 * 170.0;
                    p.rect(x, 350.0, 158.0, 112.0, 0xFFEEF2EB)?;
                    p.bold(value, x + 13.0, 369.0, 32.0, 0xFF365E46)?;
                    p.text(label, x + 13.0, 427.0, 8.0, MUTED, 140.0)?;
                }
                p.bold("Three principles to put into practice", 48.0, 510.0, 21.0, INK)?;
                let rows = [
                    ("Design for longevity", "Make repair and maintenance part of the original idea."),
                    ("Keep materials moving", "Create pathways for reuse before adding new resources."),
                    ("Measure what matters", "Track useful outcomes, not just the volume produced."),
                ];
                for (i, (title, body)) in rows.iter().enumerate() {
                    let y = 563.0 + i as f64 * 63.0;
                    p.rule(y)?;
                    p.text(&format!("0{}", i + 1), 48.0, y + 16.0, 12.0, GREEN, 40.0)?;
                    p.bold(title, 86.0, y + 11.0, 13.0, INK)?;
                    p.text(body, 86.0, y + 35.0, 10.0, MUTED, 440.0)?;
                }
            }
            2 => {
                p.text("02 / FROM INTENT TO ACTION", 48.0, 98.0, 10.0, GREEN, 490.0)?;
                p.bold("A roadmap for\nmeaningful change.", 48.0, 136.0, 36.0, INK)?;
                let stages = [
                    ("Discover", "Map material flows and listen to the people closest to the work."),
                    ("Design", "Prototype one clear improvement. Test it in a real setting."),
                    ("Deliver", "Create a repeatable system with clear ownership and measures."),
                    ("Learn", "Share results, capture feedback and make the next iteration better."),
                ];
                for (i, (title, body)) in stages.iter().enumerate() {
                    let y = 287.0 + i as f64 * 105.0;
                    p.rect(48.0, y, 42.0, 42.0, PANEL)?;
                    p.bold(&format!("0{}", i + 1), 57.0, y + 10.0, 18.0, 0xFF466B52)?;
                    p.bold(title, 111.0, y - 3.0, 20.0, INK)?;
                    p.text(body, 111.0, y + 33.0, 12.0, MUTED, 411.0)?;
                }
            }
            3 => {
                p.text("03 / MATERIAL INTELLIGENCE", 48.0, 98.0, 10.0, GREEN, 490.0)?;
                p.bold("Choose with care.", 48.0, 141.0, 36.0, INK)?;
                p.bold("Every material has a story", 48.0, 226.0, 18.0, INK)?;
                p.text("Understand where it comes from, how it performs and what happens at the end of its first life. A good specification balances immediate needs with long-term responsibility.", 48.0, 273.0, 12.0, MUTED, 223.0)?;
                p.bold("Build a useful checklist", 320.0, 226.0, 18.0, INK)?;
                p.text("Prioritize durable materials.\n\nChoose reversible connections.\n\nDocument what is inside.\n\nCreate a repair pathway.\n\nPlan for a second life.", 320.0, 273.0, 12.0, MUTED, 218.0)?;
                p.rect(48.0, 514.0, 499.0, 191.0, PANEL)?;
                p.text("“The most valuable resource\nis the one we keep in use.”", 72.0, 550.0, 26.0, 0xFF466B52, 450.0)?;
                p.text("FIELDWORK DESIGN PRINCIPLE / 04", 72.0, 659.0, 9.0, MUTED, 450.0)?;
            }
            4 => {
                p.text("04 / LEARNING THROUGH EVIDENCE", 48.0, 98.0, 10.0, GREEN, 490.0)?;
                p.bold("Progress, made visible.", 48.0, 142.0, 32.0, INK)?;
                p.text("Illustrative recovery rates across four material streams.", 48.0, 217.0, 12.0, MUTED, 490.0)?;
                let bars = [("Paper", 86.0), ("Metals", 78.0), ("Textiles", 64.0), ("Polymers", 53.0)];
                for (i, (name, value)) in bars.iter().enumerate() {
                    let y = 300.0 + i as f64 * 81.0;
                    p.text(name, 48.0, y, 12.0, INK, 90.0)?;
                    p.rect(140.0, y - 2.0, 346.0, 30.0, 0xFFEDF1E9)?;
                    p.rect(140.0, y - 2.0, value * 3.46, 30.0, if i == 0 { 0xFF466B52 } else { 0xFFA7BF99 })?;
                    p.text(&format!("{value}%"), 506.0, y + 3.0, 12.0, INK, 60.0)?;
                }
                p.bold("About this example", 48.0, 679.0, 15.0, INK)?;
                p.text("These values are fictional demonstration data, not a research claim. Use the annotation tools to review, highlight and comment on this document.", 48.0, 713.0, 11.0, MUTED, 490.0)?;
            }
            _ => {
                p.text("05 / TURN INSIGHT INTO COMMITMENT", 48.0, 98.0, 10.0, GREEN, 490.0)?;
                p.bold("Make it a shared effort.", 48.0, 142.0, 32.0, INK)?;
                p.text("Use Fill & Sign to add text, check marks and a drawn signature. Marks in this sample are visual annotations, not certificate-based digital signatures.", 48.0, 220.0, 13.0, MUTED, 490.0)?;
                p.text("YOUR NAME", 48.0, 329.0, 9.0, INK, 200.0)?;
                p.rule(391.0)?;
                p.text("TEAM / ORGANIZATION", 48.0, 432.0, 9.0, INK, 200.0)?;
                p.rule(494.0)?;
                p.text("SIGNATURE", 48.0, 536.0, 9.0, INK, 200.0)?;
                p.rule(632.0)?;
                p.text("DATE", 370.0, 536.0, 9.0, INK, 100.0)?;
                p.bold("Start small. Learn together. Keep going.", 48.0, 716.0, 18.0, 0xFF466B52)?;
            }
        }
        p.rule(792.0)?;
        p.text("CIRCULAR FUTURES / FIELDWORK STUDIO", 48.0, 810.0, 8.0, MUTED, 300.0)?;
        p.text(&format!("{:02}", index + 1), 528.0, 807.0, 10.0, INK, 40.0)?;
    }
    let bytes: Arc<[u8]> = Arc::from(document.save_to_bytes()?);
    drop(document);
    let workspace = state.open(bytes, "Circular futures.pdf")?;
    let titles = ["Cover", "The big picture", "Roadmap", "Materials", "Evidence", "Commitment"];
    let mut workspace = workspace;
    workspace.author = "Fieldwork Studio".into();
    for (page, title) in workspace.pages.iter_mut().zip(titles) {
        page.bookmark = title.into();
    }
    Ok(workspace)
}
