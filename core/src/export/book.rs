//! A small in-memory workbook: sheets of cells (numbers, text, booleans, formulas) and defined names.
//!
//! The export builds one of these, evaluates its formulas (`eval`) and writes it as xlsx (`xlsx`).

use std::collections::BTreeMap;

/// A cell position: sheet index, row and column (all 0-based).
pub type Addr = (usize, u32, u16);

#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Number(f64),
    Bool(bool),
    Text(String),
    /// Excel formula including the leading `=`.
    Formula(String),
}

/// What a cell is, which sets its look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
    #[default]
    Plain,
    Title,
    /// Column and section headings.
    Header,
    /// An input of the calculator: edit it to recompute the workbook.
    Input,
    /// A legal or model constant from the calculator's code; editable too.
    Constant,
    /// Fixed when the file was exported (sets the layout or comes from an iterative fit): re-export to change.
    Fixed,
    Formula,
    /// A result row: formula, highlighted.
    Total,
    /// A value copied from the calculator, not a formula.
    Static,
    /// Explanations next to the figures.
    Note,
    /// The defined name of the cell to its left, as formulas use it.
    Name,
}

/// Number format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Fmt {
    #[default]
    General,
    Eur,
    Pct,
    /// Entgeltpunkte.
    Points,
    /// Factors and other small numbers shown with many decimals.
    Factor,
    /// Whole numbers (months, years, counts).
    Int,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub content: Content,
    pub style: Style,
    pub fmt: Fmt,
}

impl Cell {
    pub fn new(content: Content, style: Style, fmt: Fmt) -> Self {
        Cell { content, style, fmt }
    }
}

#[derive(Debug, Default)]
pub struct Sheet {
    pub name: String,
    pub cells: BTreeMap<(u32, u16), Cell>,
    /// Column widths in characters.
    pub widths: BTreeMap<u16, f64>,
    /// First row and column that scroll (everything above and left stays visible).
    pub freeze: Option<(u32, u16)>,
}

#[derive(Debug, Default)]
pub struct Book {
    pub sheets: Vec<Sheet>,
    pub names: BTreeMap<String, Addr>,
}

impl Book {
    pub fn add_sheet(&mut self, name: &str) -> usize {
        self.sheets.push(Sheet { name: name.to_string(), ..Default::default() });
        self.sheets.len() - 1
    }

    pub fn sheet_index(&self, name: &str) -> Option<usize> {
        self.sheets.iter().position(|s| s.name == name)
    }

    pub fn set(&mut self, (s, r, c): Addr, cell: Cell) {
        self.sheets[s].cells.insert((r, c), cell);
    }

    pub fn cell(&self, (s, r, c): Addr) -> Option<&Cell> {
        self.sheets.get(s)?.cells.get(&(r, c))
    }

    pub fn define(&mut self, name: &str, addr: Addr) {
        assert!(self.names.insert(name.to_string(), addr).is_none(), "name {name} defined twice");
    }

    /// Replace the value of the input behind a defined name (the cell keeps its style).
    #[cfg(test)]
    pub fn set_input(&mut self, name: &str, content: Content) {
        let addr = self.names[name];
        let s = &mut self.sheets[addr.0];
        s.cells.get_mut(&(addr.1, addr.2)).expect("defined name points to an empty cell").content = content;
    }

    /// Reference to `addr` as written in a formula on sheet `from`: `C5`, or `'A Stay'!C5` on another sheet.
    pub fn a1(&self, from: usize, (s, r, c): Addr) -> String {
        let cell = format!("{}{}", col_letters(c), r + 1);
        if s == from {
            cell
        } else {
            format!("{}!{cell}", quote_sheet(&self.sheets[s].name))
        }
    }

    /// Absolute reference with sheet, as used by defined names: `Inputs!$C$5`.
    pub fn absolute(&self, (s, r, c): Addr) -> String {
        format!("{}!${}${}", quote_sheet(&self.sheets[s].name), col_letters(c), r + 1)
    }
}

/// Sheet names with anything but letters, digits and `_` must be quoted in references.
pub fn quote_sheet(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

/// 0 → A, 25 → Z, 26 → AA.
pub fn col_letters(c: u16) -> String {
    let mut n = c as u32 + 1;
    let mut s = Vec::new();
    while n > 0 {
        let m = (n - 1) % 26;
        s.push(b'A' + m as u8);
        n = (n - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

/// Inverse of `col_letters`.
pub fn col_index(letters: &str) -> Option<u16> {
    let mut n: u32 = 0;
    for b in letters.bytes() {
        if !b.is_ascii_uppercase() {
            return None;
        }
        n = n * 26 + (b - b'A') as u32 + 1;
    }
    if n == 0 || n > 16384 {
        None
    } else {
        Some((n - 1) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_letters_round_trip() {
        for (c, s) in [(0, "A"), (2, "C"), (25, "Z"), (26, "AA"), (51, "AZ"), (52, "BA"), (701, "ZZ"), (702, "AAA")] {
            assert_eq!(col_letters(c), s);
            assert_eq!(col_index(s), Some(c));
        }
        assert_eq!(col_index("a"), None);
        assert_eq!(col_index(""), None);
    }

    #[test]
    fn references_name_the_sheet_only_when_needed() {
        let mut b = Book::default();
        let inputs = b.add_sheet("Inputs");
        let stay = b.add_sheet("A Stay");
        assert_eq!(b.a1(inputs, (inputs, 4, 2)), "C5");
        assert_eq!(b.a1(stay, (inputs, 4, 2)), "Inputs!C5");
        assert_eq!(b.a1(inputs, (stay, 0, 27)), "'A Stay'!AB1");
        assert_eq!(b.absolute((stay, 9, 3)), "'A Stay'!$D$10");
    }

    #[test]
    fn set_input_keeps_the_style() {
        let mut b = Book::default();
        let s = b.add_sheet("Inputs");
        b.set((s, 0, 2), Cell::new(Content::Number(1.0), Style::Input, Fmt::Eur));
        b.define("salary", (s, 0, 2));
        b.set_input("salary", Content::Number(2.0));
        assert_eq!(b.cell((s, 0, 2)), Some(&Cell::new(Content::Number(2.0), Style::Input, Fmt::Eur)));
    }
}
