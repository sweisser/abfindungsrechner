//! Writes a `Book` as an xlsx file, with the evaluated results stored next to the formulas (shown by
//! viewers that do not recalculate; Excel and LibreOffice recalculate on opening).

use rust_xlsxwriter::{Color, Format, FormatBorder, Formula, Workbook, XlsxError};

use super::book::{Book, Content, Fmt, Style};
use super::eval::{Value, Values};

fn num_format(fmt: Fmt) -> &'static str {
    match fmt {
        Fmt::General => "General",
        Fmt::Eur => "#,##0.00 \"€\";-#,##0.00 \"€\"",
        Fmt::Pct => "0.00%",
        Fmt::Points => "0.0000",
        Fmt::Factor => "0.000000",
        Fmt::Int => "0",
    }
}

fn format(style: Style, fmt: Fmt) -> Format {
    let f = Format::new().set_num_format(num_format(fmt));
    match style {
        Style::Plain | Style::Formula => f,
        Style::Title => f.set_bold().set_font_size(14),
        Style::Header => f.set_bold().set_border_bottom(FormatBorder::Thin),
        Style::Input => f.set_font_color(Color::RGB(0x1F4FBF)).set_background_color(Color::RGB(0xE8F0FE)),
        Style::Constant => f.set_background_color(Color::RGB(0xEEEEEE)),
        Style::Fixed => f.set_font_color(Color::RGB(0x9A4A00)).set_background_color(Color::RGB(0xFDE7D3)),
        Style::Total => f.set_bold(),
        Style::Static => f.set_background_color(Color::RGB(0xE6F4EA)),
        Style::Note => f.set_italic().set_font_color(Color::RGB(0x666666)),
    }
}

/// The cached result of a formula as Excel stores it.
fn result(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Num(x) => Some(format!("{x}")),
        Value::Bool(b) => Some(if *b { "TRUE" } else { "FALSE" }.into()),
        Value::Text(t) => Some(t.clone()),
        Value::Empty => Some("0".into()),
        Value::Error(_) => None,
    }
}

pub fn write(book: &Book, values: &Values) -> Result<Vec<u8>, String> {
    to_bytes(book, values).map_err(|e| e.to_string())
}

fn to_bytes(book: &Book, values: &Values) -> Result<Vec<u8>, XlsxError> {
    let mut wb = Workbook::new();
    for (s, sheet) in book.sheets.iter().enumerate() {
        let ws = wb.add_worksheet();
        ws.set_name(&sheet.name)?;
        for (&c, &w) in &sheet.widths {
            ws.set_column_width(c, w)?;
        }
        if let Some((r, c)) = sheet.freeze {
            ws.set_freeze_panes(r, c)?;
        }
        for (&(r, c), cell) in &sheet.cells {
            let f = format(cell.style, cell.fmt);
            match &cell.content {
                Content::Number(x) => ws.write_number_with_format(r, c, *x, &f)?,
                Content::Bool(b) => ws.write_boolean_with_format(r, c, *b, &f)?,
                Content::Text(t) => ws.write_string_with_format(r, c, t, &f)?,
                Content::Formula(text) => {
                    let mut formula = Formula::new(text);
                    if let Some(v) = result(values.get(&(s, r, c))) {
                        formula = formula.set_result(v);
                    }
                    ws.write_formula_with_format(r, c, formula, &f)?
                }
            };
        }
    }
    for (name, &addr) in &book.names {
        wb.define_name(name, &format!("={}", book.absolute(addr)))?;
    }
    wb.save_to_buffer()
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use crate::monte_carlo::Uncertainty;
    use crate::params::test_defaults;

    fn file(zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>, name: &str) -> String {
        let mut s = String::new();
        zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
        s
    }

    #[test]
    fn writes_formulas_results_and_names() {
        let bytes = super::super::workbook(&test_defaults(), &Uncertainty::default(), 4, 1).unwrap();
        assert_eq!(&bytes[..2], b"PK");
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let wb = file(&mut zip, "xl/workbook.xml");
        for sheet in
            ["Read me", "Result", "Inputs", "Derived", "Calendar", "A Stay", "B Severance", "Pension", "Analysis"]
        {
            assert!(wb.contains(&format!("name=\"{sheet}\"")), "{sheet}");
        }
        assert!(wb.contains("<definedName name=\"salary\">Inputs!$B$"), "{wb}");
        assert!(wb.contains("fullCalcOnLoad=\"1\""), "Excel recalculates on opening");
        // Calendar (sheet 5): monthly salary of the current job, with its cached result.
        let cal = file(&mut zip, "xl/worksheets/sheet5.xml");
        assert!(cal.contains("<f>salary*(1+stay_growth)^(B2-1)/12</f><v>7500</v>"), "{}", &cal[..cal.len().min(3000)]);
        // Pension (sheet 8): text results are stored as strings.
        let pension = file(&mut zip, "xl/worksheets/sheet8.xml");
        assert!(pension.contains("t=\"str\"><f>IF(ABS("), "check cell");
    }
}
