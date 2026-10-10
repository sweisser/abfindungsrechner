//! The Result sheet (final wealth, checked against the calculator), the Analysis sheet (values
//! that need many simulations) and the Read me sheet.

use super::book::{Cell, Content, Fmt, Style};
use super::grid::Builder;
use super::Sheets;
use crate::analysis::{Analysis, Breakeven};
use crate::monte_carlo::{MonteCarlo, Summary};
use crate::params::Params;

/// A Result row (`key, label, note`): the workbook's formula, the calculator's value and their difference.
fn checked(
    b: &mut Builder,
    (key, label, note): (&str, &str, &str),
    fmt: Fmt,
    calculator: f64,
    formula: String,
    total: bool,
) {
    let style = if total { Style::Total } else { Style::Formula };
    b.put_cells(
        key,
        label,
        note,
        vec![
            Cell::new(Content::Formula(formula), style, fmt),
            Cell::new(Content::Number(calculator), Style::Static, fmt),
        ],
    );
    let diff = format!("={}-{}", b.x(key, 0), b.x(key, 1));
    b.book.set(b.addr(b.cur(), key, 2), Cell::new(Content::Formula(diff), Style::Formula, fmt));
}

pub fn result(b: &mut Builder, sh: &Sheets, analysis: &Analysis, p: &Params) {
    let sim = &analysis.sim;
    b.on(sh.result);
    b.text("Result: final wealth of both scenarios", Style::Title);
    b.text(
        "Workbook: the formulas of this file. Calculator: the value the calculator showed when you exported. \
         Difference: should be 0 (up to rounding); it changes when you edit inputs here.",
        Style::Note,
    );
    b.header("columns", "", |c| Content::Text(["Workbook", "Calculator", "Difference"][c as usize].into()));
    b.freeze(3, 1);
    for (x, s, name) in [("a", sh.a, "A: stay"), ("b", sh.b, "B: take the severance")] {
        let (rows, liquid, points, pension, fin, real) = if x == "a" {
            (&sim.a, sim.liquid_a, sim.points_a, sim.pension_a, sim.final_a, sim.real_a)
        } else {
            (&sim.b, sim.liquid_b, sim.points_b, sim.pension_b, sim.final_b, sim.real_b)
        };
        let k = |name: &str| format!("{x}_{name}");
        b.heading(name);
        let wealth_end = rows.last().map_or(0.0, |r| r.wealth);
        checked(
            b,
            (
                &k("wealth_end"),
                "Wealth (end) of the last year",
                "last column of the scenario sheet: nominal, before selling",
            ),
            Fmt::Eur,
            wealth_end,
            format!("={}", b.last(s, "wealth")),
            false,
        );
        let (w, basis) = (b.r(b.cur(), &k("wealth_end"), 0), b.last(s, if x == "a" { "f_b" } else { "r_b" }));
        checked(
            b,
            (
                &k("sale_tax"),
                "− Tax when selling everything",
                "(value − cost basis) × (1 − Teilfreistellung) − Sparer-Pauschbetrag, × capital-gains tax rate",
            ),
            Fmt::Eur,
            wealth_end - liquid,
            format!("=IF({w}<=0,0,MAX(0,({w}-{basis})*(1-tf)-sparer_total)*cap_rate)"),
            false,
        );
        checked(
            b,
            (&k("liquid"), "= Wealth after selling", ""),
            Fmt::Eur,
            liquid,
            format!("={w}-{}", b.r(b.cur(), &k("sale_tax"), 0)),
            false,
        );
        checked(
            b,
            (&k("points"), "Pension points earned within the horizon", "sum of the scenario’s points row"),
            Fmt::Points,
            points,
            format!("=SUM({})", b.span(s, "points")),
            false,
        );
        checked(
            b,
            (&k("pension"), "Value of these pension rights", "points × value per point (Pension sheet)"),
            Fmt::Eur,
            pension,
            format!("={}*value_per_point", b.r(b.cur(), &k("points"), 0)),
            false,
        );
        let counted = if p.pension_in_wealth { pension } else { 0.0 };
        checked(
            b,
            (&k("counted"), "+ Pension rights counted", "only if “Count pension rights in final wealth” is on"),
            Fmt::Eur,
            counted,
            format!("=IF(pension_in_wealth,{},0)", b.r(b.cur(), &k("pension"), 0)),
            false,
        );
        checked(
            b,
            (&k("final"), "= Final wealth, nominal", ""),
            Fmt::Eur,
            fin,
            format!("={}+{}", b.r(b.cur(), &k("liquid"), 0), b.r(b.cur(), &k("counted"), 0)),
            true,
        );
        checked(
            b,
            (&k("real"), "= Final wealth in today’s money", "÷ price level at the end of the horizon"),
            Fmt::Eur,
            real,
            format!("={}/{}", b.r(b.cur(), &k("final"), 0), b.last(sh.calendar, "price_end")),
            true,
        );
    }
    b.heading("B minus A");
    let (fa, fb) = (b.r(b.cur(), "a_final", 0), b.r(b.cur(), "b_final", 0));
    checked(b, ("diff", "B − A, nominal", ""), Fmt::Eur, sim.diff, format!("={fb}-{fa}"), true);
    checked(
        b,
        ("real_diff", "B − A in today’s money", "the figure the calculator’s verdict compares"),
        Fmt::Eur,
        sim.real_diff,
        format!("=({fb}-{fa})/{}", b.last(sh.calendar, "price_end")),
        true,
    );
    b.heading("Other figures shown in the calculator");
    checked(
        b,
        ("deflator", "Price level at the end of the horizon", ""),
        Fmt::Factor,
        sim.deflator,
        format!("={}", b.last(sh.calendar, "price_end")),
        false,
    );
    checked(
        b,
        ("sev_tax", "Tax on the severance", "in the year it is paid"),
        Fmt::Eur,
        sim.sev_tax,
        format!("=SUM({})", b.span(sh.b, "tax_on_sev")),
        false,
    );
    checked(
        b,
        ("sev_net", "Severance after tax", ""),
        Fmt::Eur,
        sim.sev_net,
        format!("=SUM({})", b.span(sh.b, "sev_net")),
        false,
    );
    checked(
        b,
        ("alg_monthly", "ALG I per month", "Derived sheet"),
        Fmt::Eur,
        sim.alg_monthly,
        "=alg_monthly".into(),
        false,
    );
}

fn static_row(b: &mut Builder, label: &str, fmt: Fmt, note: &str, values: &[Content]) {
    let cells = values.iter().map(|v| Cell::new(v.clone(), Style::Static, fmt)).collect();
    b.put_cells("", label, note, cells);
}

fn breakeven(be: &Breakeven, max: &str) -> (Content, &'static str) {
    match (be.value, be.at_lower_bound) {
        (_, true) => (Content::Number(0.0), "B already wins at 0"),
        (Some(v), false) => (Content::Number(v), ""),
        (None, false) => (Content::Text(format!("above {max}")), "B does not catch up within the search range"),
    }
}

pub fn analysis(b: &mut Builder, sh: &Sheets, analysis: &Analysis, mc: &MonteCarlo, p: &Params) {
    b.on(sh.analysis);
    b.text("Analysis: values copied from the calculator (not formulas)", Style::Title);
    b.text(
        "These figures come from many runs of the whole calculation (bisection, a grid of inputs, random simulations), \
         which a spreadsheet formula cannot reproduce. They do not change when you edit inputs in this file.",
        Style::Note,
    );
    b.heading("Break-evens (expected run): value at which B and A end equal (nominal)");
    let (v, note) = breakeven(&analysis.be_new_salary, "500,000 €");
    static_row(b, "New job salary p.a. at break-even", Fmt::Eur, note, &[v]);
    let (v, note) = breakeven(&analysis.be_severance, "3,000,000 €");
    static_row(b, "Severance at break-even", Fmt::Eur, note, &[v]);

    b.heading("Sensitivity: final wealth B − A in today’s money");
    let g = &analysis.grid;
    let mut head = vec![Content::Text("Months until new job →".into())];
    head.extend(g.months.iter().map(|&m| Content::Number(m)));
    static_row(b, "", Fmt::Int, "", &head[1..]);
    for (s, row) in g.salaries.iter().zip(&g.cells) {
        let label = format!("New salary {} €", fmt_thousands(*s));
        static_row(b, &label, Fmt::Eur, "", &row.iter().map(|&x| Content::Number(x)).collect::<Vec<_>>());
    }

    b.heading(&format!("Monte Carlo: {} simulated futures, in today’s money, after the tax due when selling", mc.runs));
    let cols = ["Mean", "5 %", "25 %", "Median", "75 %", "95 %", "Share > 0"];
    static_row(b, "", Fmt::General, "", &cols.iter().map(|c| Content::Text((*c).into())).collect::<Vec<_>>());
    for (label, s) in [("A: final wealth", &mc.real_a), ("B: final wealth", &mc.real_b), ("B − A", &mc.real_diff)] {
        static_row(b, label, Fmt::Eur, "", &summary_cells(s));
    }
    b.skip();
    b.text(
        &format!(
            "Uncertainty: months until new job, new salary and both salary growth rates drawn from PERT ranges; \
             returns lognormal around {:.1} %, inflation persistent around {:.1} % (see the calculator’s Uncertainty inputs).",
            100.0 * p.ret,
            100.0 * p.inflation
        ),
        Style::Note,
    );
}

fn summary_cells(s: &Summary) -> Vec<Content> {
    let q = &s.percentiles;
    let mut v: Vec<Content> = [s.mean, q.p5, q.p25, q.p50, q.p75, q.p95].iter().map(|&x| Content::Number(x)).collect();
    v.push(Content::Text(format!("{:.0} %", 100.0 * s.prob_positive)));
    v
}

fn fmt_thousands(x: f64) -> String {
    let s = format!("{:.0}", x.abs());
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if x < 0.0 {
        format!("−{out}")
    } else {
        out
    }
}

pub fn readme(b: &mut Builder, sh: &Sheets) {
    b.on(sh.readme);
    b.text("Severance calculator: how every figure is computed", Style::Title);
    for line in [
        "This workbook recomputes the calculator’s expected run (one calculation with your main inputs) with live formulas, \
         for A (stay) and B (take the severance). Click any cell to see its formula; every row has an explanation on the right.",
        "",
        "Sheets",
        "Result: final wealth of both scenarios and B − A, next to the calculator’s values.",
        "Inputs: your inputs and the constants of the calculator’s code. Formulas refer to them by name (e.g. salary), \
         shown next to each value; type a name into Excel’s Name Box to jump to its cell.",
        "Derived: values computed once from the inputs (rounded months, combined rates, the partner’s tax, monthly ALG I).",
        "Calendar: one column per calendar (= tax) year: months, prices, salaries, the severance year.",
        "A Stay / B Severance: income, social security, income tax, cash, investments and pension points per year.",
        "Pension: value of one pension point (Entgeltpunkt) at the end of the horizon, month by month.",
        "Analysis: break-evens, sensitivity grid and Monte Carlo results as plain values (they need many simulations).",
        "",
        "Colours",
        "Blue: inputs. Edit them and the workbook recomputes, like the calculator.",
        "Orange: fixed at export (horizon, current month, mortality level). Re-export from the calculator to change them.",
        "Grey: constants of the calculator’s code (tax tariff, contribution rates). Editable too.",
        "Green: values copied from the calculator, not formulas.",
        "",
        "How it works and what it leaves out: see METHODOLOGY.md of the calculator.",
        "",
        "Disclaimer: provided as is, without any warranty. We cannot guarantee that the calculations, tax rules or results \
         are correct, complete or up to date. Check all values yourself (ideally with a tax advisor) before making any \
         decision based on them. This is not tax, legal or financial advice.",
    ] {
        let style = if matches!(line, "Sheets" | "Colours") { Style::Header } else { Style::Plain };
        b.text(line, style);
    }
    b.book.sheets[sh.readme].widths.insert(0, 140.0);
}
