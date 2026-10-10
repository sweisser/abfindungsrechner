//! Builds sheets row by row. Each sheet is a grid: label in column A, one value column per year (or a
//! single value column), and an explanation after the last value column. Sheets of named cells show
//! each cell's defined name in a column between value and explanation. Rows have keys, so formulas
//! refer to `stay_m` instead of hard-coded addresses.

use std::collections::HashMap;

use super::book::{Addr, Book, Cell, Content, Fmt, Style};

/// Column of the first value.
pub const FIRST: u16 = 1;

pub struct Builder {
    pub book: Book,
    rows: HashMap<(usize, String), u32>,
    /// Per sheet: number of value columns, whether it has a name column, and the next free row.
    cols: Vec<u16>,
    named: Vec<bool>,
    next: Vec<u32>,
    /// The sheet rows are added to.
    cur: usize,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            book: Book::default(),
            rows: HashMap::new(),
            cols: Vec::new(),
            named: Vec::new(),
            next: Vec::new(),
            cur: 0,
        }
    }

    /// Add a sheet with `ncols` value columns; its first row is the title.
    pub fn add_sheet(&mut self, name: &str, ncols: u16) -> usize {
        self.sheet(name, ncols, false)
    }

    /// Add a sheet with one value column and a column showing the defined names (see `name`).
    pub fn add_named_sheet(&mut self, name: &str) -> usize {
        self.sheet(name, 1, true)
    }

    fn sheet(&mut self, name: &str, ncols: u16, named: bool) -> usize {
        let s = self.book.add_sheet(name);
        self.cols.push(ncols);
        self.named.push(named);
        self.next.push(0);
        let note_col = self.note_col(s);
        let widths = &mut self.book.sheets[s].widths;
        widths.insert(0, 46.0);
        for c in 0..ncols {
            widths.insert(FIRST + c, 14.0);
        }
        if named {
            widths.insert(FIRST + ncols, 22.0);
        }
        widths.insert(note_col, 80.0);
        s
    }

    /// Column of the explanations on sheet `s`.
    pub fn note_col(&self, s: usize) -> u16 {
        FIRST + self.cols[s] + self.named[s] as u16
    }

    /// Continue adding rows to sheet `s`.
    pub fn on(&mut self, s: usize) {
        self.cur = s;
    }

    pub fn cur(&self) -> usize {
        self.cur
    }

    #[cfg(test)]
    pub fn ncols(&self, s: usize) -> u16 {
        self.cols[s]
    }

    pub fn row_of(&self, s: usize, key: &str) -> Option<u32> {
        self.rows.get(&(s, key.to_string())).copied()
    }

    /// Address of row `key`, value column `c`, on sheet `s`.
    pub fn addr(&self, s: usize, key: &str, c: u16) -> Addr {
        let r = self.row_of(s, key).unwrap_or_else(|| panic!("no row {key} on {}", self.book.sheets[s].name));
        (s, r, FIRST + c)
    }

    /// Reference to row `key`, column `c` of sheet `s`, as written in a formula on the current sheet.
    pub fn r(&self, s: usize, key: &str, c: u16) -> String {
        self.book.a1(self.cur, self.addr(s, key, c))
    }

    /// Reference to a row that may not exist yet; resolved by `finish`.
    pub fn later(&self, s: usize, key: &str, c: u16) -> String {
        format!("⟦{s}|{key}|{c}⟧")
    }

    /// Resolve the references made with `later`. Call once all rows exist.
    pub fn finish(&mut self) {
        let mut edits = Vec::new();
        for (s, sheet) in self.book.sheets.iter().enumerate() {
            for (&(r, c), cell) in &sheet.cells {
                if let Content::Formula(f) = &cell.content {
                    if f.contains('⟦') {
                        edits.push(((s, r, c), self.resolve(s, f)));
                    }
                }
            }
        }
        for ((s, r, c), f) in edits {
            self.book.sheets[s].cells.get_mut(&(r, c)).unwrap().content = Content::Formula(f);
        }
    }

    fn resolve(&self, from: usize, f: &str) -> String {
        let mut out = String::new();
        let mut rest = f;
        while let Some(i) = rest.find('⟦') {
            out.push_str(&rest[..i]);
            let j = rest[i..].find('⟧').expect("unterminated placeholder") + i;
            let parts: Vec<&str> = rest[i + '⟦'.len_utf8()..j].split('|').collect();
            let (s, key, c) = (parts[0].parse().unwrap(), parts[1], parts[2].parse().unwrap());
            out.push_str(&self.book.a1(from, self.addr(s, key, c)));
            rest = &rest[j + '⟧'.len_utf8()..];
        }
        out.push_str(rest);
        out
    }

    /// Reference to the same row key on the current sheet.
    pub fn x(&self, key: &str, c: u16) -> String {
        self.r(self.cur, key, c)
    }

    /// Range over all value columns of row `key` on sheet `s`.
    pub fn span(&self, s: usize, key: &str) -> String {
        let (first, last) = (self.r(s, key, 0), self.r(s, key, self.cols[s] - 1));
        let last = last.rsplit('!').next().unwrap().to_string();
        format!("{first}:{last}")
    }

    /// Reference to the last value column of row `key` on sheet `s`.
    pub fn last(&self, s: usize, key: &str) -> String {
        self.r(s, key, self.cols[s] - 1)
    }

    fn take_row(&mut self) -> u32 {
        let r = self.next[self.cur];
        self.next[self.cur] += 1;
        r
    }

    pub fn skip(&mut self) {
        self.take_row();
    }

    /// A line of text in column A.
    pub fn text(&mut self, text: &str, style: Style) {
        let r = self.take_row();
        self.book.set((self.cur, r, 0), Cell::new(Content::Text(text.into()), style, Fmt::General));
    }

    /// A section heading: text in column A.
    pub fn heading(&mut self, text: &str) {
        self.skip();
        self.text(text, Style::Header);
    }

    /// A header row with a label and a cell per value column.
    pub fn header(&mut self, key: &str, label: &str, cells: impl Fn(u16) -> Content) {
        self.put(key, label, Style::Header, Fmt::General, "", cells);
    }

    /// Column headings of a named sheet: value, name, explanation.
    pub fn named_header(&mut self) {
        let (s, r) = (self.cur, self.take_row());
        assert!(self.named[s], "only on sheets made with add_named_sheet");
        for (c, h) in [(FIRST, "Value"), (FIRST + 1, "Name (used in formulas)"), (FIRST + 2, "Explanation")] {
            self.book.set((s, r, c), Cell::new(Content::Text(h.into()), Style::Header, Fmt::General));
        }
    }

    /// A row whose value columns are formulas `f(builder, column)`.
    pub fn row(&mut self, key: &str, label: &str, fmt: Fmt, note: &str, f: impl Fn(&Self, u16) -> String) {
        self.formula_row(key, label, Style::Formula, fmt, note, f);
    }

    /// Like `row`, highlighted as a result.
    pub fn total(&mut self, key: &str, label: &str, fmt: Fmt, note: &str, f: impl Fn(&Self, u16) -> String) {
        self.formula_row(key, label, Style::Total, fmt, note, f);
    }

    fn formula_row(
        &mut self,
        key: &str,
        label: &str,
        style: Style,
        fmt: Fmt,
        note: &str,
        f: impl Fn(&Self, u16) -> String,
    ) {
        let formulas: Vec<String> = (0..self.cols[self.cur]).map(|c| f(self, c)).collect();
        self.put(key, label, style, fmt, note, |c| Content::Formula(formulas[c as usize].clone()));
    }

    /// A row of fixed contents.
    pub fn put(&mut self, key: &str, label: &str, style: Style, fmt: Fmt, note: &str, cells: impl Fn(u16) -> Content) {
        let (s, r) = (self.cur, self.take_row());
        if !key.is_empty() {
            assert!(self.rows.insert((s, key.to_string()), r).is_none(), "row {key} added twice");
        }
        let label_style = if style == Style::Total { Style::Total } else { Style::Plain };
        let label_style = if style == Style::Header { Style::Header } else { label_style };
        self.book.set((s, r, 0), Cell::new(Content::Text(label.into()), label_style, Fmt::General));
        for c in 0..self.cols[s] {
            self.book.set((s, r, FIRST + c), Cell::new(cells(c), style, fmt));
        }
        if !note.is_empty() {
            self.book.set((s, r, self.note_col(s)), Cell::new(Content::Text(note.into()), Style::Note, Fmt::General));
        }
    }

    /// A row whose value columns each have their own content, style and format.
    pub fn put_cells(&mut self, key: &str, label: &str, note: &str, cells: Vec<Cell>) {
        let (s, r) = (self.cur, self.take_row());
        if !key.is_empty() {
            assert!(self.rows.insert((s, key.to_string()), r).is_none(), "row {key} added twice");
        }
        self.book.set((s, r, 0), Cell::new(Content::Text(label.into()), Style::Plain, Fmt::General));
        for (c, cell) in cells.into_iter().enumerate() {
            self.book.set((s, r, FIRST + c as u16), cell);
        }
        if !note.is_empty() {
            self.book.set((s, r, self.note_col(s)), Cell::new(Content::Text(note.into()), Style::Note, Fmt::General));
        }
    }

    /// Make the single value of row `key` on the current (named) sheet a defined name, shown next to it.
    pub fn name(&mut self, key: &str) {
        assert!(self.named[self.cur], "names only on sheets made with add_named_sheet");
        let (s, r, c) = self.addr(self.cur, key, 0);
        self.book.define(key, (s, r, c));
        self.book.set((s, r, c + 1), Cell::new(Content::Text(key.into()), Style::Name, Fmt::General));
    }

    pub fn freeze(&mut self, row: u32, col: u16) {
        self.book.sheets[self.cur].freeze = Some((row, col));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_refer_to_each_other_by_key() {
        let mut b = Builder::new();
        let s = b.add_sheet("A Stay", 3);
        b.on(s);
        b.put("k", "Year", Style::Header, Fmt::Int, "", |c| Content::Number(c as f64 + 1.0));
        b.row("double", "Twice the year", Fmt::Int, "k × 2", |b, c| format!("={}*2", b.x("k", c)));
        let t = b.add_sheet("Result", 1);
        b.on(t);
        b.row("sum", "Sum", Fmt::Int, "", |b, _| format!("=SUM({})", b.span(s, "double")));
        b.row("last", "Last", Fmt::Int, "", |b, _| format!("={}", b.last(s, "double")));
        b.row("ahead", "Ahead", Fmt::Int, "", |b, _| format!("={}+1", b.later(t, "after", 0)));
        b.row("after", "After", Fmt::Int, "", |_, _| "=1".into());
        b.finish();
        let cell = |a| b.book.cell(a).unwrap().content.clone();
        assert_eq!(cell((s, 1, 2)), Content::Formula("=C1*2".into()));
        assert_eq!(cell((s, 1, 4)), Content::Text("k × 2".into()));
        assert_eq!(cell((t, 0, 1)), Content::Formula("=SUM('A Stay'!B2:D2)".into()));
        assert_eq!(cell((t, 1, 1)), Content::Formula("='A Stay'!D2".into()));
        assert_eq!(cell((t, 2, 1)), Content::Formula("=B4+1".into()));
    }

    #[test]
    fn named_sheets_show_the_name_between_value_and_note() {
        let mut b = Builder::new();
        let s = b.add_named_sheet("Inputs");
        b.on(s);
        b.put("salary", "Gross salary", Style::Input, Fmt::Eur, "per year", |_| Content::Number(1.0));
        b.name("salary");
        b.put("label_only", "Unnamed row", Style::Input, Fmt::Eur, "", |_| Content::Number(2.0));
        let cell = |a| b.book.cell(a).map(|c| (c.content.clone(), c.style));
        assert_eq!(b.book.names["salary"], (s, 0, 1));
        assert_eq!(cell((s, 0, 2)), Some((Content::Text("salary".into()), Style::Name)));
        assert_eq!(cell((s, 0, 3)), Some((Content::Text("per year".into()), Style::Note)));
        assert_eq!(cell((s, 1, 2)), None);
    }
}
