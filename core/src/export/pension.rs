//! The Pension sheet: `pension::value_per_point` as a monthly table.

use super::book::{Cell, Content, Fmt, Style};
use super::grid::Builder;
use super::Sheets;
use crate::params::Params;
use crate::pension::Mortality;

/// Months from age 0 to the maximum age of 120: enough rows for any age.
pub const TABLE_ROWS: u32 = 12 * 120;
/// First column of the monthly table (E), right of the scalar rows and their notes.
const TABLE_COL: u16 = 4;

/// Survival from `age` to `age + t` under the Gompertz mortality (all arguments are formula terms).
fn survival(age: &str, t: &str) -> String {
    format!("EXP(-gompertz_b/gompertz_slope*EXP(gompertz_slope*{age})*(EXP(gompertz_slope*{t})-1))")
}

pub fn build(b: &mut Builder, sh: &Sheets, p: &Params) {
    b.on(sh.pension);
    b.text("Statutory pension: value of one Entgeltpunkt at the end of the horizon", Style::Title);
    b.text(
        "Paid monthly from the retirement age (or the end of the horizon, if later) for life, growing with wages, net of \
         contributions and tax, weighted by the chance to be alive and discounted to the end of the horizon.",
        Style::Note,
    );
    let named = |b: &mut Builder, key: &str, label: &str, fmt: Fmt, note: &str, f: &str| {
        let f = f.to_string();
        b.row(key, label, fmt, note, move |_, _| f.clone());
        b.name(key);
    };
    b.heading("Timing");
    named(b, "age_end", "Age at the end of the horizon", Fmt::Factor, "", "=age+horizon_years");
    named(
        b,
        "wait_years",
        "Years from the end of the horizon to the first payment",
        Fmt::Factor,
        "",
        "=MAX(0,retirement_age-age_end)",
    );
    named(
        b,
        "pension_start_year",
        "Year the pension starts",
        Fmt::Int,
        "fixes the taxable share",
        "=this_year+(retirement_age-age)",
    );
    b.heading("Net pension per euro of gross pension");
    named(
        b,
        "taxable_share",
        "Taxable share (§22 EStG)",
        Fmt::Pct,
        "82.5 % for 2023, + 0.5 points per year, 100 % from 2058",
        "=MIN(1,MAX(0,tax_share_base+tax_share_step*(pension_start_year-tax_share_year)))",
    );
    named(
        b,
        "retiree_contributions",
        "Retirees’ health and care contributions",
        Fmt::Pct,
        "half of 14.6 % + half the Zusatzbeitrag + the full care rate",
        "=kv_rate+zusatz/2+pv_rate+retiree_pv_extra",
    );
    named(
        b,
        "net_share",
        "Net share",
        Fmt::Pct,
        "1 − contributions − tax rate × (taxable share − contributions); contributions are deductible",
        "=1-retiree_contributions-pension_tax*MAX(0,taxable_share-retiree_contributions)",
    );
    b.heading("Payments");
    named(
        b,
        "first_t",
        "First payment: years after the end of the horizon",
        Fmt::Factor,
        "payments in the middle of each month",
        "=wait_years+0.5/12",
    );
    named(
        b,
        "growth_vs_discount",
        "Growth / discount factor per year",
        Fmt::Factor,
        "(1 + wage growth) / (1 + discount rate)",
        "=(1+pension_growth)/(1+pension_discount)",
    );
    named(
        b,
        "first_amount",
        "First monthly payment per point, discounted",
        Fmt::Eur,
        "Rentenwert × net share × factor ^ first_t × (1 + wage growth) ^ horizon",
        "=rentenwert*net_share*growth_vs_discount^first_t*(1+pension_growth)^horizon_years",
    );
    named(
        b,
        "month_step",
        "Change from one month to the next",
        Fmt::Factor,
        "factor ^ (1/12)",
        "=growth_vs_discount^(1/12)",
    );
    named(
        b,
        "months_paid",
        "Months up to the maximum age",
        Fmt::Int,
        "",
        "=MAX(0,ROUNDUP(12*(max_age-(age_end+wait_years)),0))",
    );
    b.heading("Mortality (Gompertz: force of mortality b · e^(slope · age))");
    let mortality = Mortality::calibrated(p.retirement_age, p.life_expectancy);
    let gb = mortality.b();
    b.put(
        "gompertz_b",
        "Mortality level b",
        Style::Fixed,
        Fmt::General,
        "Fixed at export: fitted by bisection so that the life expectancy at retirement matches your input. Re-export after changing retirement age or life expectancy.",
        |_| Content::Number(gb),
    );
    b.name("gompertz_b");
    named(
        b,
        "le_months",
        "Months from retirement to the maximum age",
        Fmt::Int,
        "",
        "=MAX(0,ROUNDUP(12*(max_age-retirement_age),0))",
    );
    let col = |i: u16| super::book::col_letters(TABLE_COL + i);
    let (first, last) = (3, 3 + TABLE_ROWS);
    let range = |i: u16| format!("{}{}:{}{}", col(i), first + 1, col(i), last);
    b.row(
        "le_check",
        "Life expectancy at retirement implied by b",
        Fmt::Factor,
        "sum of the survival column of the table / 12",
        |_, _| format!("=SUM({})", range(6)),
    );
    b.row("le_status", "Check", Fmt::General, "", |b, c| {
        format!("=IF(ABS({}-life_expectancy)<0.01,\"OK\",\"b no longer fits: re-export\")", b.x("le_check", c))
    });
    b.heading("Result");
    b.total(
        "value_per_point",
        "Value of one Entgeltpunkt at the end of the horizon",
        Fmt::Eur,
        "sum of the value column of the table",
        |_, _| format!("=SUM({})", range(5)),
    );
    b.name("value_per_point");

    // The monthly table, right of the scalars.
    let s = sh.pension;
    let heads = [
        "Month",
        "Paid",
        "Years after horizon end",
        "Survival",
        "Payment (discounted)",
        "Value",
        "Survival from retirement",
    ];
    for (i, h) in heads.iter().enumerate() {
        b.book.set(
            (s, first - 1, TABLE_COL + i as u16),
            Cell::new(Content::Text((*h).into()), Style::Header, Fmt::General),
        );
        b.book.sheets[s].widths.insert(TABLE_COL + i as u16, 16.0);
    }
    for m in 0..TABLE_ROWS {
        let r = first + m;
        let a = |i: u16| format!("{}{}", col(i), r + 1);
        let f = |i: u16, fmt: Fmt, formula: String| (i, Cell::new(Content::Formula(formula), Style::Formula, fmt));
        let cells = [
            (0, Cell::new(Content::Number(m as f64), Style::Plain, Fmt::Int)),
            f(1, Fmt::Int, format!("=IF({}<months_paid,1,0)", a(0))),
            f(2, Fmt::Factor, format!("=first_t+{}/12", a(0))),
            f(3, Fmt::Factor, format!("={}", survival("age_end", &a(2)))),
            f(4, Fmt::Eur, format!("=first_amount*month_step^{}", a(0))),
            f(5, Fmt::Eur, format!("=IF({}=1,{}*{},0)", a(1), a(3), a(4))),
            f(
                6,
                Fmt::Factor,
                format!("=IF({}<le_months,{},0)/12", a(0), survival("retirement_age", &format!("(({}+0.5)/12)", a(0)))),
            ),
        ];
        for (i, cell) in cells {
            b.book.set((s, r, TABLE_COL + i), cell);
        }
    }
    b.freeze(1, 0);
}
