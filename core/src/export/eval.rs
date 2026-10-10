//! Evaluates the formulas of a `Book`, for the subset of Excel the export writes.
//!
//! Supported: numbers, strings, `TRUE`/`FALSE`, cell references (`C5`, `$C$5`, `Sheet!C5`,
//! `'A Stay'!C5`), ranges inside functions (`C5:C9`), defined names, `+ - * / ^`, comparisons and
//! `IF AND OR NOT MIN MAX SUM INT ROUND ROUNDUP EXP LN ABS`, with Excel's precedence: unary minus
//! binds tighter than `^` (`-2^2 = 4`) and `^` is left-associative.
//!
//! The export writes the results as the formulas' cached values, and the tests check them against
//! the simulation. Cells are evaluated in dependency order without recursion, so long chains of
//! references (year after year) cannot overflow the stack.

use std::collections::HashMap;

use super::book::{col_index, Addr, Book, Content};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Empty,
    Num(f64),
    Bool(bool),
    Text(String),
    /// An Excel error such as `#DIV/0!`.
    Error(String),
}

impl Value {
    /// The numeric value, as Excel uses it in arithmetic.
    pub fn num(&self) -> Result<f64, String> {
        match self {
            Value::Empty => Ok(0.0),
            Value::Num(x) => Ok(*x),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Text(_) => Err("#VALUE!".into()),
            Value::Error(e) => Err(e.clone()),
        }
    }

    fn truthy(&self) -> Result<bool, String> {
        self.num().map(|x| x != 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Num(f64),
    Bool(bool),
    Text(String),
    Ref(Addr),
    Range(Addr, Addr),
    Neg(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

/// The value of every formula cell.
pub type Values = HashMap<Addr, Value>;

/// Evaluate all formulas of `book`. Fails on a syntax error, an unknown name or sheet, or a cycle;
/// Excel errors such as `#DIV/0!` are values, not failures.
pub fn evaluate(book: &Book) -> Result<Values, String> {
    let mut exprs = HashMap::new();
    for (s, sheet) in book.sheets.iter().enumerate() {
        for (&(r, c), cell) in &sheet.cells {
            if let Content::Formula(f) = &cell.content {
                let e = parse(book, s, f).map_err(|e| format!("{}!{}{}: {e}: {f}", sheet.name, col(c), r + 1))?;
                exprs.insert((s, r, c), e);
            }
        }
    }
    let order = topological_order(book, &exprs)?;
    let mut values = Values::new();
    for addr in order {
        let v = Ctx { book, values: &values }.eval(&exprs[&addr]);
        values.insert(addr, v);
    }
    Ok(values)
}

fn col(c: u16) -> String {
    super::book::col_letters(c)
}

/// Formula cells ordered so that each comes after the formula cells it refers to.
fn topological_order(book: &Book, exprs: &HashMap<Addr, Expr>) -> Result<Vec<Addr>, String> {
    let deps: HashMap<Addr, Vec<Addr>> = exprs
        .iter()
        .map(|(&a, e)| {
            let mut d = Vec::new();
            refs(book, e, &mut d);
            d.retain(|x| exprs.contains_key(x));
            (a, d)
        })
        .collect();
    // 0 = new, 1 = on the stack, 2 = done.
    let mut state: HashMap<Addr, u8> = HashMap::new();
    let mut order = Vec::with_capacity(exprs.len());
    let mut starts: Vec<Addr> = exprs.keys().copied().collect();
    starts.sort();
    for start in starts {
        if state.get(&start).copied().unwrap_or(0) != 0 {
            continue;
        }
        let mut stack = vec![(start, 0usize)];
        state.insert(start, 1);
        while let Some(&mut (addr, ref mut next)) = stack.last_mut() {
            let d = &deps[&addr];
            if *next < d.len() {
                let dep = d[*next];
                *next += 1;
                match state.get(&dep).copied().unwrap_or(0) {
                    0 => {
                        state.insert(dep, 1);
                        stack.push((dep, 0));
                    }
                    1 => return Err(format!("circular reference at {}", describe(book, dep))),
                    _ => {}
                }
            } else {
                state.insert(addr, 2);
                order.push(addr);
                stack.pop();
            }
        }
    }
    Ok(order)
}

fn describe(book: &Book, (s, r, c): Addr) -> String {
    format!("{}!{}{}", book.sheets[s].name, col(c), r + 1)
}

/// Every cell `e` refers to; ranges are expanded to their non-empty cells.
fn refs(book: &Book, e: &Expr, out: &mut Vec<Addr>) {
    match e {
        Expr::Ref(a) => out.push(*a),
        Expr::Range(a, b) => out.extend(range_cells(book, *a, *b)),
        Expr::Neg(x) => refs(book, x, out),
        Expr::Bin(_, x, y) => {
            refs(book, x, out);
            refs(book, y, out);
        }
        Expr::Call(_, args) => args.iter().for_each(|x| refs(book, x, out)),
        Expr::Num(_) | Expr::Bool(_) | Expr::Text(_) => {}
    }
}

fn range_cells(book: &Book, (s, r0, c0): Addr, (_, r1, c1): Addr) -> impl Iterator<Item = Addr> + '_ {
    let (rl, rh, cl, ch) = (r0.min(r1), r0.max(r1), c0.min(c1), c0.max(c1));
    book.sheets[s]
        .cells
        .range((rl, 0)..=(rh, u16::MAX))
        .filter(move |(&(_, c), _)| c >= cl && c <= ch)
        .map(move |(&(r, c), _)| (s, r, c))
}

struct Ctx<'a> {
    book: &'a Book,
    values: &'a Values,
}

impl Ctx<'_> {
    fn cell(&self, a: Addr) -> Value {
        if let Some(v) = self.values.get(&a) {
            return v.clone();
        }
        match self.book.cell(a).map(|c| &c.content) {
            None => Value::Empty,
            Some(Content::Number(x)) => Value::Num(*x),
            Some(Content::Bool(b)) => Value::Bool(*b),
            Some(Content::Text(t)) => Value::Text(t.clone()),
            Some(Content::Formula(_)) => unreachable!("formula evaluated out of order"),
        }
    }

    fn eval(&self, e: &Expr) -> Value {
        self.try_eval(e).unwrap_or_else(Value::Error)
    }

    fn num(&self, e: &Expr) -> Result<f64, String> {
        self.try_eval(e)?.num()
    }

    fn try_eval(&self, e: &Expr) -> Result<Value, String> {
        Ok(match e {
            Expr::Num(x) => Value::Num(*x),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Text(t) => Value::Text(t.clone()),
            Expr::Ref(a) => match self.cell(*a) {
                Value::Error(e) => return Err(e),
                v => v,
            },
            Expr::Range(..) => return Err("#VALUE!".into()),
            Expr::Neg(x) => Value::Num(-self.num(x)?),
            Expr::Bin(op, x, y) => self.binary(*op, x, y)?,
            Expr::Call(f, args) => self.call(f, args)?,
        })
    }

    fn binary(&self, op: Op, x: &Expr, y: &Expr) -> Result<Value, String> {
        let (a, b) = (self.num(x)?, self.num(y)?);
        let v = match op {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div if b == 0.0 => return Err("#DIV/0!".into()),
            Op::Div => a / b,
            Op::Pow => a.powf(b),
            Op::Eq => return Ok(Value::Bool(a == b)),
            Op::Ne => return Ok(Value::Bool(a != b)),
            Op::Lt => return Ok(Value::Bool(a < b)),
            Op::Le => return Ok(Value::Bool(a <= b)),
            Op::Gt => return Ok(Value::Bool(a > b)),
            Op::Ge => return Ok(Value::Bool(a >= b)),
        };
        finite(v)
    }

    /// Numbers of all arguments, ranges expanded (empty and text cells in ranges are skipped, as in Excel).
    fn numbers(&self, args: &[Expr]) -> Result<Vec<f64>, String> {
        let mut out = Vec::new();
        for a in args {
            match a {
                Expr::Range(x, y) => {
                    for c in range_cells(self.book, *x, *y) {
                        match self.cell(c) {
                            Value::Num(v) => out.push(v),
                            Value::Bool(b) => out.push(if b { 1.0 } else { 0.0 }),
                            Value::Error(e) => return Err(e),
                            Value::Empty | Value::Text(_) => {}
                        }
                    }
                }
                _ => out.push(self.num(a)?),
            }
        }
        Ok(out)
    }

    fn call(&self, f: &str, args: &[Expr]) -> Result<Value, String> {
        let arity = |n: usize| if args.len() == n { Ok(()) } else { Err(format!("{f} takes {n} arguments")) };
        let one = || -> Result<f64, String> {
            arity(1)?;
            self.num(&args[0])
        };
        let v = match f {
            "IF" => {
                if args.len() != 2 && args.len() != 3 {
                    return Err("IF takes 2 or 3 arguments".into());
                }
                return if self.try_eval(&args[0])?.truthy()? {
                    self.try_eval(&args[1])
                } else if args.len() == 3 {
                    self.try_eval(&args[2])
                } else {
                    Ok(Value::Bool(false))
                };
            }
            "AND" => return Ok(Value::Bool(self.numbers(args)?.iter().all(|&x| x != 0.0))),
            "OR" => return Ok(Value::Bool(self.numbers(args)?.iter().any(|&x| x != 0.0))),
            "NOT" => return Ok(Value::Bool(one()? == 0.0)),
            "MIN" => self.numbers(args)?.into_iter().reduce(f64::min).unwrap_or(0.0),
            "MAX" => self.numbers(args)?.into_iter().reduce(f64::max).unwrap_or(0.0),
            "SUM" => self.numbers(args)?.iter().sum(),
            "INT" => one()?.floor(),
            "ROUND" | "ROUNDUP" => {
                arity(2)?;
                let (x, d) = (self.num(&args[0])?, self.num(&args[1])?.trunc());
                let scale = 10f64.powf(d);
                let y = x * scale;
                if f == "ROUND" {
                    y.round() / scale
                } else {
                    (if y < 0.0 { y.floor() } else { y.ceil() }) / scale
                }
            }
            "EXP" => one()?.exp(),
            "LN" => {
                let x = one()?;
                if x <= 0.0 {
                    return Err("#NUM!".into());
                }
                x.ln()
            }
            "ABS" => one()?.abs(),
            _ => return Err(format!("#NAME? ({f})")),
        };
        finite(v)
    }
}

fn finite(v: f64) -> Result<Value, String> {
    if v.is_finite() {
        Ok(Value::Num(v))
    } else {
        Err("#NUM!".into())
    }
}

const FUNCTIONS: [&str; 13] =
    ["IF", "AND", "OR", "NOT", "MIN", "MAX", "SUM", "INT", "ROUND", "ROUNDUP", "EXP", "LN", "ABS"];

// ---- parser ----

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    /// Identifier: function, name, cell reference or unquoted sheet name.
    Id(String),
    /// Quoted sheet name.
    Sheet(String),
    Op(&'static str),
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'E' || chars[i] == 'e') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| format!("bad number {s}"))?));
        } else if c == '"' || c == '\'' {
            let mut s = String::new();
            i += 1;
            loop {
                match chars.get(i) {
                    None => return Err("unterminated quote".into()),
                    Some(&q) if q == c && chars.get(i + 1) == Some(&c) => {
                        s.push(c);
                        i += 2;
                    }
                    Some(&q) if q == c => {
                        i += 1;
                        break;
                    }
                    Some(&q) => {
                        s.push(q);
                        i += 1;
                    }
                }
            }
            out.push(if c == '"' { Tok::Str(s) } else { Tok::Sheet(s) });
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '.' | '$')) {
                i += 1;
            }
            out.push(Tok::Id(chars[start..i].iter().collect()));
        } else {
            let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let op = ["<=", ">=", "<>"].into_iter().find(|o| *o == two);
            if let Some(op) = op {
                out.push(Tok::Op(op));
                i += 2;
            } else {
                let op = ["+", "-", "*", "/", "^", "=", "<", ">", "(", ")", ",", ":", "!"]
                    .into_iter()
                    .find(|o| o.starts_with(c))
                    .ok_or_else(|| format!("unexpected character {c}"))?;
                out.push(Tok::Op(op));
                i += 1;
            }
        }
    }
    Ok(out)
}

/// `C5`, `$C$5` → (row, col).
fn cell_ref(id: &str) -> Option<(u32, u16)> {
    let s = id.replace('$', "");
    let split = s.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = s.split_at(split);
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let row: u32 = digits.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, col_index(letters)?))
}

fn parse(book: &Book, sheet: usize, formula: &str) -> Result<Expr, String> {
    let src = formula.strip_prefix('=').ok_or("formula must start with =")?;
    let mut p = Parser { book, sheet, toks: tokenize(src)?, pos: 0 };
    let e = p.comparison()?;
    if p.pos != p.toks.len() {
        return Err(format!("unexpected {:?}", p.toks[p.pos]));
    }
    Ok(e)
}

struct Parser<'a> {
    book: &'a Book,
    sheet: usize,
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser<'_> {
    fn peek_op(&self) -> Option<&'static str> {
        match self.toks.get(self.pos) {
            Some(Tok::Op(o)) => Some(*o),
            _ => None,
        }
    }

    fn eat(&mut self, op: &str) -> bool {
        if self.peek_op() == Some(op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, op: &str) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(format!("expected {op}"))
        }
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let mut lhs = self.additive()?;
        loop {
            let op = match self.peek_op() {
                Some("=") => Op::Eq,
                Some("<>") => Op::Ne,
                Some("<") => Op::Lt,
                Some("<=") => Op::Le,
                Some(">") => Op::Gt,
                Some(">=") => Op::Ge,
                _ => return Ok(lhs),
            };
            self.pos += 1;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(self.additive()?));
        }
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.multiplicative()?;
        loop {
            let op = match self.peek_op() {
                Some("+") => Op::Add,
                Some("-") => Op::Sub,
                _ => return Ok(lhs),
            };
            self.pos += 1;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(self.multiplicative()?));
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut lhs = self.power()?;
        loop {
            let op = match self.peek_op() {
                Some("*") => Op::Mul,
                Some("/") => Op::Div,
                _ => return Ok(lhs),
            };
            self.pos += 1;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(self.power()?));
        }
    }

    fn power(&mut self) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        while self.eat("^") {
            lhs = Expr::Bin(Op::Pow, Box::new(lhs), Box::new(self.unary()?));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat("-") {
            Ok(Expr::Neg(Box::new(self.unary()?)))
        } else if self.eat("+") {
            self.unary()
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let tok = self.toks.get(self.pos).cloned().ok_or("unexpected end of formula")?;
        self.pos += 1;
        match tok {
            Tok::Num(x) => Ok(Expr::Num(x)),
            Tok::Str(s) => Ok(Expr::Text(s)),
            Tok::Op("(") => {
                let e = self.comparison()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Sheet(name) => {
                self.expect("!")?;
                let s = self.book.sheet_index(&name).ok_or_else(|| format!("unknown sheet {name}"))?;
                self.reference(s)
            }
            Tok::Id(id) => {
                if self.peek_op() == Some("(") {
                    self.pos += 1;
                    return self.call(id.to_ascii_uppercase());
                }
                if self.peek_op() == Some("!") {
                    self.pos += 1;
                    let s = self.book.sheet_index(&id).ok_or_else(|| format!("unknown sheet {id}"))?;
                    return self.reference(s);
                }
                match id.to_ascii_uppercase().as_str() {
                    "TRUE" => return Ok(Expr::Bool(true)),
                    "FALSE" => return Ok(Expr::Bool(false)),
                    _ => {}
                }
                if cell_ref(&id).is_some() {
                    self.pos -= 1;
                    return self.reference(self.sheet);
                }
                let &a = self.book.names.get(&id).ok_or_else(|| format!("#NAME? ({id})"))?;
                Ok(Expr::Ref(a))
            }
            t => Err(format!("unexpected {t:?}")),
        }
    }

    /// A cell or range on sheet `s`, at the current token.
    fn reference(&mut self, s: usize) -> Result<Expr, String> {
        let first = self.cell_tok()?;
        if self.eat(":") {
            let second = self.cell_tok()?;
            return Ok(Expr::Range((s, first.0, first.1), (s, second.0, second.1)));
        }
        Ok(Expr::Ref((s, first.0, first.1)))
    }

    fn cell_tok(&mut self) -> Result<(u32, u16), String> {
        match self.toks.get(self.pos) {
            Some(Tok::Id(id)) => {
                let r = cell_ref(id).ok_or_else(|| format!("bad cell reference {id}"))?;
                self.pos += 1;
                Ok(r)
            }
            t => Err(format!("expected a cell reference, got {t:?}")),
        }
    }

    fn call(&mut self, name: String) -> Result<Expr, String> {
        if !FUNCTIONS.contains(&name.as_str()) {
            return Err(format!("unsupported function {name}"));
        }
        let mut args = Vec::new();
        if !self.eat(")") {
            loop {
                args.push(self.comparison()?);
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        Ok(Expr::Call(name, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::book::{Cell, Fmt, Style};

    fn book() -> Book {
        let mut b = Book::default();
        let inputs = b.add_sheet("Inputs");
        b.add_sheet("A Stay");
        b.set((inputs, 0, 2), Cell::new(Content::Number(90000.0), Style::Input, Fmt::Eur));
        b.set((inputs, 1, 2), Cell::new(Content::Bool(true), Style::Input, Fmt::General));
        b.define("salary", (inputs, 0, 2));
        b.define("joint", (inputs, 1, 2));
        b
    }

    /// Value of `formula` placed on sheet 1 (`A Stay`).
    fn calc_in(mut b: Book, formula: &str) -> Value {
        b.set((1, 50, 50), Cell::new(Content::Formula(formula.into()), Style::Formula, Fmt::General));
        evaluate(&b).unwrap()[&(1, 50, 50)].clone()
    }

    fn calc(formula: &str) -> Value {
        calc_in(book(), formula)
    }

    fn num(formula: &str) -> f64 {
        match calc(formula) {
            Value::Num(x) => x,
            v => panic!("{formula}: {v:?}"),
        }
    }

    #[test]
    fn arithmetic_follows_excel_precedence() {
        assert_eq!(num("=1+2*3"), 7.0);
        assert_eq!(num("=(1+2)*3"), 9.0);
        assert_eq!(num("=-2^2"), 4.0, "unary minus binds tighter than ^");
        assert_eq!(num("=0-2^2"), -4.0);
        assert_eq!(num("=2^-1"), 0.5);
        assert_eq!(num("=2^3^2"), 64.0, "^ is left-associative");
        assert_eq!(num("=10/4/5"), 0.5);
        assert_eq!(num("=1E4+2.5e-1+.5"), 10000.75);
        assert_eq!(num("=7-2-1"), 4.0);
    }

    #[test]
    fn comparisons_and_logic() {
        assert_eq!(calc("=1+1=2"), Value::Bool(true));
        assert_eq!(calc("=3<>3"), Value::Bool(false));
        assert_eq!(calc("=AND(1<2,2<=2,3>=4)"), Value::Bool(false));
        assert_eq!(calc("=OR(FALSE,1>0)"), Value::Bool(true));
        assert_eq!(calc("=NOT(joint)"), Value::Bool(false));
        assert_eq!(num("=IF(joint,2,1)*3"), 6.0);
        assert_eq!(num("=TRUE+TRUE"), 2.0);
        assert_eq!(calc("=IF(1>2,\"yes\",\"no \"\"quoted\"\"\")"), Value::Text("no \"quoted\"".into()));
    }

    #[test]
    fn if_evaluates_only_the_chosen_branch() {
        assert_eq!(num("=IF(0>1,1/0,5)"), 5.0);
        assert_eq!(calc("=IF(1>0,1/0,5)"), Value::Error("#DIV/0!".into()));
    }

    #[test]
    fn functions() {
        assert_eq!(num("=MIN(3,1,2)+MAX(3,1,2)"), 4.0);
        assert_eq!(num("=INT(2.7)+INT(-2.5)"), -1.0);
        assert_eq!(num("=ROUND(2.5,0)+ROUND(-2.5,0)+ROUND(1.234,2)"), 1.23);
        assert_eq!(num("=ROUNDUP(2.1,0)+ROUNDUP(-2.1,0)"), 0.0);
        assert_eq!(num("=LN(EXP(2))"), 2.0);
        assert_eq!(num("=ABS(-3)"), 3.0);
        assert_eq!(num("=min(1,2)"), 1.0, "function names are case-insensitive");
    }

    #[test]
    fn errors_are_values() {
        assert_eq!(calc("=1/0"), Value::Error("#DIV/0!".into()));
        assert_eq!(calc("=LN(0)"), Value::Error("#NUM!".into()));
        assert_eq!(calc("=(-8)^0.5"), Value::Error("#NUM!".into()));
        assert_eq!(calc("=1+\"a\""), Value::Error("#VALUE!".into()));
    }

    #[test]
    fn references_names_and_sheets() {
        let mut b = book();
        b.set((1, 0, 2), Cell::new(Content::Number(2.0), Style::Plain, Fmt::General));
        b.set((1, 1, 2), Cell::new(Content::Formula("=C1*salary".into()), Style::Formula, Fmt::General));
        assert_eq!(calc_in(b, "=C2+$C$1+Inputs!C1+'A Stay'!C1"), Value::Num(180000.0 + 2.0 + 90000.0 + 2.0));
        assert_eq!(num("=Z99+1"), 1.0, "empty cells are 0");
    }

    #[test]
    fn sum_over_ranges_skips_empty_and_text_cells() {
        let mut b = book();
        for (r, x) in [(0, 1.0), (1, 2.0), (3, 4.0)] {
            b.set((1, r, 3), Cell::new(Content::Number(x), Style::Plain, Fmt::General));
        }
        b.set((1, 2, 3), Cell::new(Content::Text("x".into()), Style::Plain, Fmt::General));
        b.set((1, 0, 4), Cell::new(Content::Number(100.0), Style::Plain, Fmt::General));
        assert_eq!(calc_in(b, "=SUM(D1:D4)+MAX(D1:D4)"), Value::Num(11.0));
    }

    #[test]
    fn long_reference_chains_do_not_overflow_the_stack() {
        let mut b = book();
        b.set((1, 0, 0), Cell::new(Content::Number(1.0), Style::Plain, Fmt::General));
        // Each row refers to the one below it, so a top-down evaluation would recurse 20,000 deep.
        for r in 1..20_000u32 {
            let f = format!("=A{}+1", r + 2);
            b.set((1, r, 0), Cell::new(Content::Formula(f), Style::Formula, Fmt::General));
        }
        b.set((1, 20_000, 0), Cell::new(Content::Number(0.0), Style::Plain, Fmt::General));
        let v = evaluate(&b).unwrap();
        assert_eq!(v[&(1, 1, 0)], Value::Num(19_999.0));
    }

    #[test]
    fn invalid_formulas_fail_with_their_location() {
        for (f, msg) in [
            ("=nope+1", "#NAME? (nope)"),
            ("=FOO(1)", "unsupported function FOO"),
            ("=Missing!A1", "unknown sheet Missing"),
            ("=(1+2", "expected )"),
            ("=1+", "unexpected end"),
            ("1+2", "must start with ="),
        ] {
            let mut b = book();
            b.set((1, 3, 1), Cell::new(Content::Formula(f.into()), Style::Formula, Fmt::General));
            let err = evaluate(&b).unwrap_err();
            assert!(err.contains("A Stay!B4") && err.contains(msg), "{f}: {err}");
        }
    }

    #[test]
    fn cycles_are_reported() {
        let mut b = book();
        b.set((1, 0, 0), Cell::new(Content::Formula("=A2".into()), Style::Formula, Fmt::General));
        b.set((1, 1, 0), Cell::new(Content::Formula("=A1".into()), Style::Formula, Fmt::General));
        assert!(evaluate(&b).unwrap_err().contains("circular"));
    }
}
