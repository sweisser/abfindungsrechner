//! Excel export: the expected run of both scenarios as a workbook of live formulas, so that every
//! figure can be traced back to the inputs, plus the Monte Carlo and other analyses as values.
//!
//! The sheets mirror the Rust code row by row. The formulas are evaluated here (`eval`) to store
//! their results in the file, and the tests check them against `simulate`.

mod book;
mod eval;
mod grid;
mod inputs;
mod pension;
mod scenarios;
mod summary;
mod taxes;
mod xlsx;

use crate::analysis::{analyze_params, Analysis};
use crate::monte_carlo::{self, MonteCarlo, Uncertainty};
use crate::params::Params;
use crate::simulation::periods;
use grid::Builder;

/// Sheet indices, in tab order.
pub struct Sheets {
    pub readme: usize,
    pub result: usize,
    pub inputs: usize,
    pub derived: usize,
    pub calendar: usize,
    pub a: usize,
    pub b: usize,
    pub pension: usize,
    pub analysis: usize,
}

/// Build all sheets for `p`; `analysis` and `mc` provide the calculator's values.
fn build(p: &Params, analysis: &Analysis, mc: &MonteCarlo) -> (Builder, Sheets) {
    let n = periods(p).len() as u16;
    let mut b = Builder::new();
    let sh = Sheets {
        readme: b.add_sheet("Read me", 1),
        result: b.add_sheet("Result", 3),
        inputs: b.add_named_sheet("Inputs"),
        derived: b.add_named_sheet("Derived"),
        calendar: b.add_sheet("Calendar", n),
        a: b.add_sheet("A Stay", n),
        b: b.add_sheet("B Severance", n),
        pension: b.add_named_sheet("Pension"),
        analysis: b.add_sheet("Analysis", 7),
    };
    inputs::build(&mut b, sh.inputs, p);
    scenarios::derived(&mut b, &sh);
    scenarios::calendar(&mut b, &sh, p);
    scenarios::stay(&mut b, &sh, p);
    scenarios::severance(&mut b, &sh, p);
    pension::build(&mut b, &sh, p);
    summary::result(&mut b, &sh, analysis, p);
    summary::analysis(&mut b, &sh, analysis, mc, p);
    summary::readme(&mut b, &sh);
    b.finish();
    (b, sh)
}

/// The xlsx file for `p`, with the Monte Carlo run as the calculator runs it.
pub fn workbook(p: &Params, uncertainty: &Uncertainty, runs: u32, seed: u64) -> Result<Vec<u8>, String> {
    if periods(p).is_empty() {
        return Err("the horizon must be at least one year".into());
    }
    let analysis = analyze_params(p);
    let mc = monte_carlo::run(p, uncertainty, runs, seed);
    let (b, _) = build(p, &analysis, &mc);
    let values = eval::evaluate(&b.book)?;
    xlsx::write(&b.book, &values)
}

#[cfg(test)]
mod tests {
    use super::book::{Addr, Content};
    use super::eval::{evaluate, Value, Values};
    use super::*;
    use crate::params::test_defaults;
    use crate::simulation::{simulate, Sim};

    fn mc(p: &Params) -> MonteCarlo {
        monte_carlo::run(p, &Uncertainty::default(), 4, 1)
    }

    fn built(p: &Params) -> (Builder, Sheets) {
        build(p, &analyze_params(p), &mc(p))
    }

    fn num(v: &Values, a: Addr) -> f64 {
        match &v[&a] {
            Value::Num(x) => *x,
            other => panic!("{a:?}: {other:?}"),
        }
    }

    fn close(got: f64, want: f64) -> bool {
        (got - want).abs() <= 1e-9 * want.abs().max(1.0)
    }

    /// Every figure of `sim` equals the workbook's formula of it.
    fn assert_matches(b: &Builder, sh: &Sheets, v: &Values, sim: &Sim, what: &str) {
        for (sheet, rows, name) in [(sh.a, &sim.a, "A"), (sh.b, &sim.b, "B")] {
            assert_eq!(b.ncols(sheet) as usize, rows.len());
            for (c, row) in rows.iter().enumerate() {
                let c = c as u16;
                let mut fields = vec![
                    ("wage", row.wage),
                    ("sv", row.sv),
                    ("tax_cash", row.tax),
                    ("net", row.net),
                    ("spend", row.spend),
                    ("wealth", row.wealth),
                    ("points", row.points),
                ];
                if sheet == sh.b {
                    fields.extend([
                        ("alg", row.alg),
                        ("sev", row.sev),
                        ("tax_on_sev", row.tax_on_sev),
                        ("sev_net", row.sev_net),
                    ]);
                }
                for (key, want) in fields {
                    let got = num(v, b.addr(sheet, key, c));
                    assert!(close(got, want), "{what}: {name} year {} {key}: workbook {got}, code {want}", c + 1);
                }
                let price = num(v, b.addr(sh.calendar, "price_end", c));
                assert!(close(price, sim.price_levels[c as usize + 1]), "{what}: price level {}", c + 1);
            }
        }
        for (key, want) in [
            ("a_liquid", sim.liquid_a),
            ("b_liquid", sim.liquid_b),
            ("a_points", sim.points_a),
            ("b_points", sim.points_b),
            ("a_pension", sim.pension_a),
            ("b_pension", sim.pension_b),
            ("a_final", sim.final_a),
            ("b_final", sim.final_b),
            ("a_real", sim.real_a),
            ("b_real", sim.real_b),
            ("diff", sim.diff),
            ("real_diff", sim.real_diff),
            ("deflator", sim.deflator),
            ("sev_tax", sim.sev_tax),
            ("sev_net", sim.sev_net),
            ("alg_monthly", sim.alg_monthly),
        ] {
            let got = num(v, b.addr(sh.result, key, 0));
            assert!(close(got, want), "{what}: {key}: workbook {got}, code {want}");
        }
    }

    fn assert_no_errors(b: &Builder, v: &Values, what: &str) {
        for (a, x) in v {
            assert!(!matches!(x, Value::Error(_)), "{what}: {} {a:?} is {x:?}", b.book.sheets[a.0].name);
        }
    }

    fn cases() -> Vec<(&'static str, Params)> {
        let d = test_defaults();
        let with = |f: &dyn Fn(&mut Params)| {
            let mut p = d.clone();
            f(&mut p);
            p
        };
        vec![
            ("defaults", d.clone()),
            ("starting in October", with(&|p| p.start_month = 10.0)),
            ("joint with partner income", with(&|p| (p.joint, p.partner_zve) = (true, 30_000.0))),
            ("joint, partner without income", with(&|p| p.joint = true)),
            ("church tax", with(&|p| p.church = 0.09)),
            ("without Fünftelregelung", with(&|p| p.fuenftel = false)),
            ("gains taxed yearly", with(&|p| (p.deferred, p.start_wealth) = (false, 50_000.0))),
            ("paid in January", with(&|p| (p.start_month, p.exit_month, p.pay_delay) = (10.0, 1.0, 2.0))),
            ("leaving later", with(&|p| (p.exit_month, p.search_months) = (14.0, 9.0))),
            ("wealth goes negative", with(&|p| p.spending = 80_000.0)),
            ("sells with gains", with(&|p| (p.start_wealth, p.spending) = (200_000.0, 70_000.0))),
            ("sells everything", with(&|p| (p.start_wealth, p.spending, p.deferred) = (60_000.0, 90_000.0, false))),
            ("no new job", with(&|p| p.new_salary = 0.0)),
            ("no ALG I", with(&|p| p.alg_months = 0.0)),
            ("pension counted", with(&|p| p.pension_in_wealth = true)),
            ("high salary above the ceilings", with(&|p| (p.salary, p.new_salary) = (180_000.0, 150_000.0))),
            ("retiring within the horizon", with(&|p| (p.age, p.horizon) = (62.0, 8.0))),
            ("severance after the horizon", with(&|p| (p.horizon, p.exit_month) = (2.0, 30.0))),
            ("long horizon", with(&|p| (p.horizon, p.start_month) = (30.0, 7.0))),
        ]
    }

    #[test]
    fn workbook_reproduces_the_simulation() {
        for (what, p) in cases() {
            let (b, sh) = built(&p);
            let v = evaluate(&b.book).unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_no_errors(&b, &v, what);
            assert_matches(&b, &sh, &v, &simulate(&p), what);
        }
    }

    #[test]
    fn calculator_values_match_and_differences_are_zero() {
        let p = test_defaults();
        let (b, sh) = built(&p);
        let v = evaluate(&b.book).unwrap();
        let s = simulate(&p);
        assert_eq!(b.book.cell(b.addr(sh.result, "real_diff", 1)).unwrap().content, Content::Number(s.real_diff));
        assert!(num(&v, b.addr(sh.result, "real_diff", 2)).abs() < 1e-6);
    }

    /// Edit inputs in the workbook (as a user would in Excel) and compare with a fresh simulation.
    fn edited(base: &Params, edit: &dyn Fn(&mut Params)) -> (Builder, Sheets, Params) {
        let (mut b, sh) = built(base);
        let mut q = base.clone();
        edit(&mut q);
        let (old, new) = (inputs::values(base), inputs::values(&q));
        for i in inputs::INPUTS.iter().flat_map(|(_, g)| g.iter()) {
            if old[i.key] != new[i.key] {
                b.book.set_input(i.name, inputs::content(&new[i.key]));
            }
        }
        (b, sh, q)
    }

    #[test]
    fn editing_inputs_recomputes_the_workbook() {
        let base = test_defaults();
        type Edit = Box<dyn Fn(&mut Params)>;
        let edits: Vec<(&str, Edit)> = vec![
            ("salary and severance", Box::new(|p| (p.salary, p.severance) = (70_000.0, 150_000.0))),
            ("severance moves to the next year", Box::new(|p| (p.exit_month, p.pay_delay) = (11.0, 1.0))),
            ("joint assessment", Box::new(|p| (p.joint, p.partner_zve, p.church) = (true, 25_000.0, 0.08))),
            (
                "market and spending",
                Box::new(|p| (p.ret, p.inflation, p.spending, p.start_wealth) = (0.03, 0.04, 70_000.0, 20_000.0)),
            ),
            ("job search and ALG I", Box::new(|p| (p.search_months, p.alg_months, p.alg_rate) = (15.0, 12.0, 0.67))),
            ("taxes on funds", Box::new(|p| (p.deferred, p.tf, p.start_wealth) = (false, 0.15, 100_000.0))),
            (
                "pension inputs",
                Box::new(|p| (p.pension_in_wealth, p.age, p.pension_discount, p.rentenwert) = (true, 50.0, 0.02, 45.0)),
            ),
            ("constants", Box::new(|p| (p.bbg_rv, p.bbg_kv, p.wk, p.sparer) = (110_000.0, 75_000.0, 1500.0, 2000.0))),
        ];
        for (what, edit) in edits {
            let (b, sh, q) = edited(&base, &*edit);
            let v = evaluate(&b.book).unwrap();
            assert_no_errors(&b, &v, what);
            assert_matches(&b, &sh, &v, &simulate(&q), what);
        }
    }

    #[test]
    fn the_mortality_check_flags_a_stale_fit() {
        let p = test_defaults();
        for (what, edit, ok) in [
            ("unchanged", Box::new(|_: &mut Params| {}) as Box<dyn Fn(&mut Params)>, true),
            ("retirement age", Box::new(|p: &mut Params| p.retirement_age = 63.0), false),
            ("life expectancy", Box::new(|p: &mut Params| p.life_expectancy = 23.0), false),
        ] {
            let (b, sh, _) = edited(&p, &*edit);
            let v = evaluate(&b.book).unwrap();
            let status = &v[&b.addr(sh.pension, "le_status", 0)];
            assert_eq!(status == &Value::Text("OK".into()), ok, "{what}: {status:?}");
        }
    }

    #[test]
    fn every_defined_name_is_shown_next_to_its_cell() {
        let (b, _) = built(&test_defaults());
        assert!(b.book.names.len() > 80);
        for (name, &(s, r, c)) in &b.book.names {
            let shown = b.book.cell((s, r, c + 1)).map(|x| x.content.clone());
            assert_eq!(shown, Some(Content::Text(name.clone())), "{name}");
        }
    }

    #[test]
    fn rejects_an_empty_horizon() {
        let mut p = test_defaults();
        p.horizon = 0.0;
        assert!(workbook(&p, &Uncertainty::default(), 4, 1).unwrap_err().contains("horizon"));
    }
}
