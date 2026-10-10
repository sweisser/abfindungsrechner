//! The Derived, Calendar, "A Stay" and "B Severance" sheets: `simulation::simulate_path` with the
//! constant market path, as formulas. One column per calendar year (`simulation::periods`).

use super::book::{Content, Fmt, Style};
use super::grid::Builder;
use super::taxes::{self, Items};
use super::Sheets;
use crate::params::Params;
use crate::simulation::periods;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Months of the current calendar year before today, as in `simulation::months_before_start`.
fn before(p: &Params) -> f64 {
    p.start_month.round().clamp(1.0, 12.0) - 1.0
}

/// Header rows of a sheet with year columns: the year number `k` and its months.
pub fn year_header(b: &mut Builder, p: &Params) {
    let ps = periods(p);
    let before = before(p);
    b.put("k", "Year of the horizon", Style::Header, Fmt::Int, "", |c| Content::Number(c as f64 + 1.0));
    b.put("months_label", "Calendar months", Style::Header, Fmt::General, "", |c| {
        let q = ps[c as usize];
        let (first, last) = ((before + q.start) as usize % 12, (before + q.end - 1.0) as usize % 12);
        let year = 2026 + c as usize;
        Content::Text(if q.end - q.start >= 12.0 {
            format!("{year}")
        } else {
            format!("{year} {}–{}", MONTHS[first], MONTHS[last])
        })
    });
}

/// Scalars used everywhere: rounded month counts, combined rates, the partner's tax and monthly ALG I.
pub fn derived(b: &mut Builder, sh: &Sheets) {
    b.on(sh.derived);
    b.text("Derived values", Style::Title);
    b.text("Values computed once from the inputs and used by the other sheets (all are defined names).", Style::Note);
    let named = |b: &mut Builder, key: &str, label: &str, fmt: Fmt, note: &str, f: &str| {
        let f = f.to_string();
        b.row(key, label, fmt, note, move |_, _| f.clone());
        b.name(key);
    };
    b.heading("Calendar");
    named(
        b,
        "before",
        "Months of this year before today",
        Fmt::Int,
        "current month − 1; they count for this year’s income tax, but their pay is already spent",
        "=MIN(12,MAX(1,ROUND(start_month,0)))-1",
    );
    named(b, "horizon_years", "Horizon in whole years", Fmt::Int, "", "=MAX(0,ROUND(horizon,0))");
    named(b, "horizon_months", "Horizon in months", Fmt::Int, "", "=12*horizon_years");
    named(
        b,
        "exit_m",
        "Month you leave (from today)",
        Fmt::Int,
        "inputs are rounded to whole months",
        "=ROUND(exit_month,0)",
    );
    named(b, "search_m", "Months until the new job", Fmt::Int, "", "=ROUND(search_months,0)");
    named(
        b,
        "alg_m_max",
        "Months on ALG I",
        Fmt::Int,
        "the entitlement, but at most until the new job starts",
        "=MIN(ROUND(alg_months,0),search_m)",
    );
    named(b, "pay_m", "Month the severance is paid (from today)", Fmt::Int, "", "=exit_m+ROUND(pay_delay,0)");
    b.heading("Rates and allowances");
    named(
        b,
        "rvav_rate",
        "Pension + unemployment insurance",
        Fmt::Pct,
        "employee share, up to the RV ceiling",
        "=rv_rate+av_rate",
    );
    named(
        b,
        "kvpv_rate",
        "Health + care insurance",
        Fmt::Pct,
        "employee share incl. half the Zusatzbeitrag, up to the KV ceiling",
        "=kv_rate+zusatz/2+pv_rate",
    );
    named(b, "sa", "Sonderausgaben-Pauschbetrag", Fmt::Eur, "", "=IF(joint,sa_joint,sa_single)");
    named(
        b,
        "soli_threshold",
        "Soli Freigrenze",
        Fmt::Eur,
        "doubled with joint assessment",
        "=soli_free*IF(joint,2,1)",
    );
    named(
        b,
        "sparer_total",
        "Sparer-Pauschbetrag of the household",
        Fmt::Eur,
        "doubled with joint assessment",
        "=sparer*IF(joint,2,1)",
    );
    named(b, "cap_rate", "Tax rate on capital gains", Fmt::Pct, "Abgeltungsteuer + Soli + church tax; church tax is deductible: 25 % / (1 + 25 % × church) × (1 + 5.5 % + church)", "=abgeltung/(1+abgeltung*church)*(1+soli_rate+church)");
    b.heading("Partner’s income tax alone (joint assessment)");
    taxes::partner_block(b);
    b.heading("Monthly ALG I");
    b.row(
        "alg_salary_m",
        "Monthly gross salary it is based on",
        Fmt::Eur,
        "current salary, capped at the RV ceiling",
        |_, _| "=MIN(salary/12,bbg_rv/12)".into(),
    );
    b.row("alg_wage", "Yearly gross at that salary", Fmt::Eur, "", |b, c| format!("={}*12", b.x("alg_salary_m", c)));
    b.row("alg_sv", "Social security on it", Fmt::Eur, "", |b, c| {
        let m = b.x("alg_salary_m", c);
        format!("=12*(rvav_rate*MIN({m},bbg_rv/12)+kvpv_rate*MIN({m},bbg_kv/12))")
    });
    taxes::block(b, &Items { wage: "alg_wage", sv: "alg_sv", alg: None, sev: None });
    b.row("alg_net", "Yearly net salary", Fmt::Eur, "", |b, c| {
        format!("={}-{}-{}", b.x("alg_wage", c), b.x("alg_sv", c), b.x("tax", c))
    });
    b.total("alg_monthly", "ALG I per month", Fmt::Eur, "ALG I rate × monthly net salary", |b, c| {
        format!("=alg_rate*({}/12)", b.x("alg_net", c))
    });
    b.name("alg_monthly");
    b.freeze(1, 0);
}

/// Per calendar year: months, market, price level, salaries and the severance year.
pub fn calendar(b: &mut Builder, sh: &Sheets, p: &Params) {
    b.on(sh.calendar);
    b.text("Calendar: one column per calendar (= tax) year of the horizon", Style::Title);
    year_header(b, p);
    b.freeze(3, 1);
    b.heading("Months (counted from today)");
    b.row("start", "Start", Fmt::Int, "", |b, c| format!("=MAX(0,12*({}-1)-before)", b.x("k", c)));
    b.row("end", "End", Fmt::Int, "", |b, c| format!("=MIN(12*{}-before,horizon_months)", b.x("k", c)));
    b.row("months", "Months inside the horizon", Fmt::Int, "", |b, c| {
        format!("={}-{}", b.x("end", c), b.x("start", c))
    });
    b.row("years", "Years inside the horizon", Fmt::Factor, "months / 12", |b, c| format!("={}/12", b.x("months", c)));
    b.row(
        "earlier",
        "Months before today (first year only)",
        Fmt::Int,
        "taxed with this year, already paid",
        |b, c| format!("=IF({}=1,before,0)", b.x("k", c)),
    );
    b.heading("Market and prices");
    b.row(
        "ret_k",
        "Gross return",
        Fmt::Pct,
        "expected return every year (the Monte Carlo varies it, not shown here)",
        |_, _| "=ret".into(),
    );
    b.row(
        "growth_half",
        "Growth of mid-year flows",
        Fmt::Factor,
        "(1 + return − TER) ^ (years / 2): a year’s surplus arrives in the middle of its months",
        |b, c| format!("=(1+{}-ter)^({}/2)", b.x("ret_k", c), b.x("years", c)),
    );
    b.row("price_start", "Price level at the start", Fmt::Factor, "today = 1", |b, c| {
        if c == 0 {
            "=1".into()
        } else {
            format!("={}", b.later(b.cur(), "price_end", c - 1))
        }
    });
    b.row("price_end", "Price level at the end", Fmt::Factor, "× (1 + inflation) ^ years", |b, c| {
        format!("={}*(1+inflation)^{}", b.x("price_start", c), b.x("years", c))
    });
    b.row("spend", "Spending", Fmt::Eur, "spending in today’s money × price level at the start × years", |b, c| {
        format!("=spending*{}*{}", b.x("price_start", c), b.x("years", c))
    });
    b.heading("Salaries and pension index (raises every January)");
    b.row("stay_m", "Monthly salary, current job", Fmt::Eur, "salary × (1 + growth) ^ (year − 1) / 12", |b, c| {
        format!("=salary*(1+stay_growth)^({}-1)/12", b.x("k", c))
    });
    b.row("new_m", "Monthly salary, new job", Fmt::Eur, "", |b, c| {
        format!("=new_salary*(1+new_growth)^({}-1)/12", b.x("k", c))
    });
    b.row(
        "wage_index",
        "Wage index",
        Fmt::Factor,
        "average earnings and RV ceiling grow with the wage and pension growth",
        |b, c| format!("=(1+pension_growth)^({}-1)", b.x("k", c)),
    );
    b.row("sev_flag", "Severance paid this year", Fmt::Int, "1 if the payment month falls into this year", |b, c| {
        format!("=IF(AND({}<=pay_m,pay_m<{}),1,0)", b.x("start", c), b.x("end", c))
    });
}

/// Social security on `months` at monthly salary `m`, against the ceilings.
fn sv(m: &str, months: &str) -> String {
    format!("{months}*(rvav_rate*MIN({m},bbg_rv/12)+kvpv_rate*MIN({m},bbg_kv/12))")
}

/// Entgeltpunkte for `months` at monthly gross `m` in the year of column `c`.
fn points(b: &Builder, sh: &Sheets, m: &str, months: &str, c: u16) -> String {
    let i = b.r(sh.calendar, "wage_index", c);
    format!("{months}*MIN({m},bbg_rv/12*{i})/(avg_earnings*{i})")
}

/// `Port::grow`: return (or debt interest) over the months of the year; taxed yearly unless deferred.
fn grow_rows(b: &mut Builder, sh: &Sheets, end_w: &str, end_b: &str) {
    let (end_w, end_b) = (end_w.to_string(), end_b.to_string());
    b.row(
        "w0",
        "Wealth at the start",
        Fmt::Eur,
        "existing investments in the first year, then last year’s end",
        move |b, c| {
            if c == 0 {
                "=start_wealth".into()
            } else {
                format!("={}", b.later(b.cur(), &end_w, c - 1))
            }
        },
    );
    b.row(
        "b0",
        "Cost basis at the start",
        Fmt::Eur,
        "what was paid for the investments; gains above it are taxed at sale",
        move |b, c| {
            if c == 0 {
                "=start_wealth".into()
            } else {
                format!("={}", b.later(b.cur(), &end_b, c - 1))
            }
        },
    );
    b.row(
        "gain",
        "Return over the year",
        Fmt::Eur,
        "wealth × ((1 + return − TER) ^ years − 1); none on debt",
        |b, c| {
            let (w, cal) = (b.x("w0", c), sh.calendar);
            format!("=IF({w}<0,0,{w}*(MAX(0,1+({}-ter))^{}-1))", b.r(cal, "ret_k", c), b.r(cal, "years", c))
        },
    );
    b.row(
        "gain_taxable",
        "Taxable return (if taxed yearly)",
        Fmt::Eur,
        "not accumulating: gain × (1 − Teilfreistellung) − Sparer-Pauschbetrag",
        |b, c| {
            let (w, g) = (b.x("w0", c), b.x("gain", c));
            format!("=IF(OR({w}<0,deferred,{g}<=0),0,MAX(0,{g}*(1-tf)-sparer_total))")
        },
    );
    b.row(
        "w_grown",
        "Wealth after the return",
        Fmt::Eur,
        "debt grows with the debt interest; else + gain − tax on the taxable gain",
        |b, c| {
            let (w, g, t) = (b.x("w0", c), b.x("gain", c), b.x("gain_taxable", c));
            format!("=IF({w}<0,{w}*(1+debt_rate)^{},{w}+{g}-{t}*cap_rate)", b.r(sh.calendar, "years", c))
        },
    );
    b.row(
        "b_grown",
        "Cost basis after the return",
        Fmt::Eur,
        "taxed gains (and debt) raise the basis to the value",
        |b, c| format!("=IF(OR({}<0,NOT(deferred)),{},{})", b.x("w0", c), b.x("w_grown", c), b.x("b0", c)),
    );
    b.row(
        "pb_grown",
        "Sparer-Pauschbetrag left this year",
        Fmt::Eur,
        "new every year, minus what the yearly taxed gain used",
        |b, c| {
            let (w, g) = (b.x("w0", c), b.x("gain", c));
            format!("=IF(OR({w}<0,deferred,{g}<=0),sparer_total,MAX(0,sparer_total-{g}*(1-tf)))")
        },
    );
}

/// Row keys of the portfolio state a flow starts from.
struct State<'a> {
    w: &'a str,
    b: &'a str,
    pb: &'a str,
}

/// `Port::flow`: invest a surplus, or sell (and pay tax on the gains sold) to cover a shortfall.
/// Rows `{f}w`, `{f}b`, `{f}pb` hold the new state.
fn flow_rows(b: &mut Builder, f: &str, s: State, amount: &str, label: &str, growth: impl Fn(&Builder, u16) -> String) {
    let k = |name: &str| format!("{f}{name}");
    let (w, bs, pb) = (s.w.to_string(), s.b.to_string(), s.pb.to_string());
    b.row(
        &k("scaled"),
        &format!("{label}: at the end of the year"),
        Fmt::Eur,
        "amount × growth from the time it arrives to the year end",
        |b, c| format!("={}*{}", b.x(amount, c), growth(b, c)),
    );
    b.row(&k("need"), "Shortfall to cover", Fmt::Eur, "", |b, c| format!("=MAX(0,-{})", b.x(&k("scaled"), c)));
    b.row(
        &k("gain_share"),
        "Taxable share of a sale",
        Fmt::Pct,
        "(1 − basis / value) × (1 − Teilfreistellung)",
        |b, c| {
            let (w, bs) = (b.x(&w, c), b.x(&bs, c));
            format!("=IF({w}>0,MAX(0,1-{bs}/{w})*(1-tf),0)")
        },
    );
    b.row(
        &k("sell"),
        "Amount to sell",
        Fmt::Eur,
        "so that the shortfall remains after tax; the Sparer-Pauschbetrag left is used first",
        |b, c| {
            let (n, g, pb) = (b.x(&k("need"), c), b.x(&k("gain_share"), c), b.x(&pb, c));
            format!("=IF({n}*{g}>{pb},({n}-cap_rate*{pb})/(1-cap_rate*{g}),{n})")
        },
    );
    b.row(
        &k("case"),
        &format!("{label}: what happens"),
        Fmt::General,
        "invest a surplus; cover a shortfall by selling or, with no investments left, by debt",
        |b, c| {
            let (x, w) = (b.x(&k("scaled"), c), b.x(&w, c));
            format!(
                "=IF({x}>=0,\"invest\",IF({w}<=0,\"borrow\",IF({}>={w},\"sell all\",\"sell part\")))",
                b.x(&k("sell"), c)
            )
        },
    );
    b.row(
        &k("sale_tax_all"),
        "Tax if everything is sold",
        Fmt::Eur,
        "used when the shortfall exceeds the whole portfolio",
        |b, c| {
            let (w, bs, pb) = (b.x(&w, c), b.x(&bs, c), b.x(&pb, c));
            format!("=IF({w}>0,MAX(0,({w}-{bs})*(1-tf)-{pb})*cap_rate,0)")
        },
    );
    b.row(&k("w"), &format!("Wealth after {}", label.to_lowercase()), Fmt::Eur, "", |b, c| {
        let (x, w, n, sell) = (b.x(&k("scaled"), c), b.x(&w, c), b.x(&k("need"), c), b.x(&k("sell"), c));
        format!(
            "=IF({x}>=0,{w}+{x},IF({w}<=0,{w}-{n},IF({sell}>={w},{w}-{}-{n},{w}-{sell})))",
            b.x(&k("sale_tax_all"), c)
        )
    });
    b.row(
        &k("b"),
        "Cost basis",
        Fmt::Eur,
        "a surplus adds the amount actually paid in; a partial sale removes the basis pro rata",
        |b, c| {
            let (x, w, bs, sell, nw) =
                (b.x(&k("scaled"), c), b.x(&w, c), b.x(&bs, c), b.x(&k("sell"), c), b.x(&k("w"), c));
            format!(
                "=IF({x}>=0,IF({w}+{x}<0,{nw},{bs}+{}),IF(OR({w}<=0,{sell}>={w}),{nw},{bs}*(1-{sell}/{w})))",
                b.x(amount, c)
            )
        },
    );
    b.row(&k("pb"), "Sparer-Pauschbetrag left", Fmt::Eur, "", |b, c| {
        let (x, w, pb, sell) = (b.x(&k("scaled"), c), b.x(&w, c), b.x(&pb, c), b.x(&k("sell"), c));
        format!("=IF(OR({x}>=0,{w}<=0,{sell}>={w}),{pb},MAX(0,{pb}-{sell}*{}))", b.x(&k("gain_share"), c))
    });
}

/// Scenario A: stay in the job.
pub fn stay(b: &mut Builder, sh: &Sheets, p: &Params) {
    let cal = sh.calendar;
    b.on(sh.a);
    b.text("A: stay in your job", Style::Title);
    year_header(b, p);
    b.freeze(3, 1);
    b.heading("Income of the whole calendar year (incl. the months before today)");
    b.row(
        "n_full",
        "Months at the current salary",
        Fmt::Int,
        "months before today + months inside the horizon",
        |b, c| format!("={}+{}", b.r(cal, "earlier", c), b.r(cal, "months", c)),
    );
    b.row("wage_full", "Gross wage", Fmt::Eur, "", |b, c| format!("={}*{}", b.r(cal, "stay_m", c), b.x("n_full", c)));
    b.row("sv_full", "Social security", Fmt::Eur, "employee share, month by month up to the ceilings", |b, c| {
        format!("={}", sv(&b.r(cal, "stay_m", c), &b.x("n_full", c)))
    });
    b.heading("Income tax of the calendar year");
    taxes::block(b, &Items { wage: "wage_full", sv: "sv_full", alg: None, sev: None });
    b.row("net_full", "Net income", Fmt::Eur, "", |b, c| {
        format!("={}-{}-{}", b.x("wage_full", c), b.x("sv_full", c), b.x("tax", c))
    });
    b.heading("Cash inside the horizon (the months before today are already paid and spent)");
    b.row(
        "share_paid",
        "Share of the year already paid",
        Fmt::Pct,
        "months before today / all months at this salary; the salary is constant within the year",
        |b, c| {
            let e = b.r(cal, "earlier", c);
            format!("={e}/({e}+{})", b.r(cal, "months", c))
        },
    );
    for (key, full, label) in [
        ("wage", "wage_full", "Gross wage"),
        ("sv", "sv_full", "Social security"),
        ("tax_cash", "tax", "Income tax"),
        ("net", "net_full", "Net income"),
    ] {
        b.total(key, label, Fmt::Eur, "whole year − share already paid", |b, c| {
            let f = b.x(full, c);
            format!("={f}-{f}*{}", b.x("share_paid", c))
        });
    }
    b.total("spend", "Spending", Fmt::Eur, "from the Calendar sheet", |b, c| format!("={}", b.r(cal, "spend", c)));
    b.row("surplus", "Surplus (net income − spending)", Fmt::Eur, "", |b, c| {
        format!("={}-{}", b.x("net", c), b.x("spend", c))
    });
    b.heading("Investments");
    grow_rows(b, sh, "f_w", "f_b");
    flow_rows(b, "f_", State { w: "w_grown", b: "b_grown", pb: "pb_grown" }, "surplus", "Surplus", |b, c| {
        b.r(cal, "growth_half", c)
    });
    b.total(
        "wealth",
        "Wealth (end)",
        Fmt::Eur,
        "nominal, before the tax due when selling; as in the calculator’s table",
        |b, c| format!("={}", b.x("f_w", c)),
    );
    b.heading("Statutory pension");
    b.total(
        "points",
        "Pension points (Entgeltpunkte)",
        Fmt::Points,
        "months × salary up to the RV ceiling / average earnings (both × wage index)",
        |b, c| format!("={}", points(b, sh, &b.r(cal, "stay_m", c), &b.r(cal, "months", c), c)),
    );
}

/// Scenario B: take the severance.
pub fn severance(b: &mut Builder, sh: &Sheets, p: &Params) {
    let (cal, a) = (sh.calendar, sh.a);
    b.on(sh.b);
    b.text("B: take the severance", Style::Title);
    year_header(b, p);
    b.freeze(3, 1);
    b.heading("Months of the calendar year");
    b.row("n_old_after", "Months at the old salary from today", Fmt::Int, "until you leave", |b, c| {
        format!("=MAX(0,MIN(exit_m,{})-MAX(0,{}))", b.r(cal, "end", c), b.r(cal, "start", c))
    });
    b.row("n_old", "Months at the old salary", Fmt::Int, "incl. the months before today", |b, c| {
        format!("={}+{}", b.r(cal, "earlier", c), b.x("n_old_after", c))
    });
    b.row("n_alg", "Months on ALG I", Fmt::Int, "from leaving, for the ALG I months", |b, c| {
        format!("=MAX(0,MIN(exit_m+alg_m_max,{})-MAX(exit_m,{}))", b.r(cal, "end", c), b.r(cal, "start", c))
    });
    b.row(
        "n_new",
        "Months in the new job",
        Fmt::Int,
        "from leaving + months until the new job; none if the new salary is 0",
        |b, c| {
            format!("=IF(new_salary>0,MAX(0,{}-MAX(exit_m+search_m,{})),0)", b.r(cal, "end", c), b.r(cal, "start", c))
        },
    );
    b.heading("Income of the whole calendar year (incl. the months before today)");
    b.row("wage_full", "Gross wage", Fmt::Eur, "old salary × its months + new salary × its months", |b, c| {
        format!("={}*{}+{}*{}", b.r(cal, "stay_m", c), b.x("n_old", c), b.r(cal, "new_m", c), b.x("n_new", c))
    });
    b.row("sv_full", "Social security", Fmt::Eur, "on wages only: none on ALG I and severance", |b, c| {
        format!("={}+{}", sv(&b.r(cal, "stay_m", c), &b.x("n_old", c)), sv(&b.r(cal, "new_m", c), &b.x("n_new", c)))
    });
    b.total("alg", "ALG I", Fmt::Eur, "months on ALG I × ALG I per month (Derived sheet)", |b, c| {
        format!("={}*alg_monthly", b.x("n_alg", c))
    });
    b.total("sev", "Severance", Fmt::Eur, "in the year it is paid", |b, c| {
        format!("=IF({}=1,severance,0)", b.r(cal, "sev_flag", c))
    });
    b.heading("Income tax of the calendar year");
    taxes::block(b, &Items { wage: "wage_full", sv: "sv_full", alg: Some("alg"), sev: Some("sev") });
    b.row("net_full", "Net income", Fmt::Eur, "wage + ALG I + severance − social security − tax", |b, c| {
        format!("={}+{}+{}-{}-{}", b.x("wage_full", c), b.x("alg", c), b.x("sev", c), b.x("sv_full", c), b.x("tax", c))
    });
    b.heading("Cash inside the horizon (the months before today are already paid and spent)");
    for (key, full, label) in [
        ("wage", "wage_full", "Gross wage"),
        ("sv", "sv_full", "Social security"),
        ("tax_cash", "tax", "Income tax"),
        ("net", "net_full", "Net income"),
    ] {
        b.total(key, label, Fmt::Eur, "whole year − what A was paid before today (same job, same months)", |b, c| {
            format!("={}-{}*{}", b.x(full, c), b.r(a, full, c), b.r(a, "share_paid", c))
        });
    }
    b.total("spend", "Spending", Fmt::Eur, "from the Calendar sheet", |b, c| format!("={}", b.r(cal, "spend", c)));
    b.row("sev_net", "Severance after tax", Fmt::Eur, "", |b, c| {
        format!("={}-{}", b.x("sev", c), b.x("tax_on_sev", c))
    });
    b.row("rest", "Other surplus (net income without the severance − spending)", Fmt::Eur, "", |b, c| {
        format!("={}-{}+{}-{}", b.x("net", c), b.x("sev", c), b.x("tax_on_sev", c), b.x("spend", c))
    });
    b.heading("Investments");
    grow_rows(b, sh, "r_w", "r_b");
    flow_rows(b, "s_", State { w: "w_grown", b: "b_grown", pb: "pb_grown" }, "sev_net", "Severance", |b, c| {
        format!("(1+{}-ter)^(({}-pay_m)/12)", b.r(cal, "ret_k", c), b.r(cal, "end", c))
    });
    flow_rows(b, "r_", State { w: "s_w", b: "s_b", pb: "s_pb" }, "rest", "Other surplus", |b, c| {
        b.r(cal, "growth_half", c)
    });
    b.total(
        "wealth",
        "Wealth (end)",
        Fmt::Eur,
        "nominal, before the tax due when selling; as in the calculator’s table",
        |b, c| format!("={}", b.x("r_w", c)),
    );
    b.heading("Statutory pension");
    b.total(
        "points",
        "Pension points (Entgeltpunkte)",
        Fmt::Points,
        "old job from today + new job + ALG I on 80 % of the old salary; none for the severance",
        |b, c| {
            let alg_m = "alg_points_share*MIN(salary,bbg_rv)/12";
            format!(
                "={}+{}+{}",
                points(b, sh, &b.r(cal, "stay_m", c), &b.x("n_old_after", c), c),
                points(b, sh, &b.r(cal, "new_m", c), &b.x("n_new", c), c),
                points(b, sh, alg_m, &b.x("n_alg", c), c)
            )
        },
    );
}
