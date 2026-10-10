//! Income tax as formulas, mirroring `tax.rs` and the household tax in `income.rs`.

use super::book::Fmt;
use super::grid::Builder;

/// §32a EStG tariff of the (already floored, non-negative) taxable income in cell `x`.
pub fn tariff(x: &str) -> String {
    format!(
        "IF({x}<=t_free,0,IF({x}<=t_z1,(t_a1*(({x}-t_free)/1E4)+t_b1)*(({x}-t_free)/1E4),\
         IF({x}<=t_z2,(t_a2*(({x}-t_z1)/1E4)+t_b2)*(({x}-t_z1)/1E4)+t_c2,\
         IF({x}<=t_z3,t_r3*{x}-t_d3,t_r4*{x}-t_d4))))"
    )
}

/// Two rows for the income tax of the household on `income`: the tariff argument (halved with
/// joint assessment, floored to whole euros) and the tax (doubled with joint assessment: splitting).
fn est_rows(b: &mut Builder, key: &str, label: &str, note: &str, income: impl Fn(&Builder, u16) -> String) {
    b.row(&format!("x_{key}"), &format!("Tariff argument: {label}"), Fmt::Eur, note, |b, c| {
        let x = income(b, c);
        format!("=INT(MAX(0,IF(joint,({x})/2,{x})))")
    });
    b.row(
        &format!("est_{key}"),
        &format!("Income tax on {label}"),
        Fmt::Eur,
        "§32a EStG; ×2 with joint assessment (splitting)",
        |b, c| format!("=IF(joint,2,1)*{}", tariff(&b.x(&format!("x_{key}"), c))),
    );
}

/// Rows for the Soli + church tax on top of the income tax in row `e`, and their sum.
fn surcharge_rows(b: &mut Builder, e: &str, suffix: &str, label: &str) {
    b.row(
        &format!("soli{suffix}"),
        &format!("Soli{label}"),
        Fmt::Eur,
        "5.5 % of the income tax above the Freigrenze, phased in at 11.9 % of the excess",
        |b, c| {
            let e = b.x(e, c);
            format!("=IF({e}<=soli_threshold,0,MIN(soli_rate*{e},soli_cap*({e}-soli_threshold)))")
        },
    );
    b.row(
        &format!("church_tax{suffix}"),
        &format!("Church tax{label}"),
        Fmt::Eur,
        "church tax rate × income tax",
        |b, c| format!("=church*{}", b.x(e, c)),
    );
    b.row(
        &format!("household_tax{suffix}"),
        &format!("Household tax{label}"),
        Fmt::Eur,
        "income tax + Soli + church tax",
        |b, c| {
            format!("={}+{}+{}", b.x(e, c), b.x(&format!("soli{suffix}"), c), b.x(&format!("church_tax{suffix}"), c))
        },
    );
}

/// Row keys of the yearly items the tax block reads, on the current sheet.
pub struct Items<'a> {
    pub wage: &'a str,
    pub sv: &'a str,
    /// ALG I and severance, if the scenario has them.
    pub alg: Option<&'a str>,
    pub sev: Option<&'a str>,
}

/// The rows of `income::user_tax` for one year: taxable income, income tax (with the
/// Progressionsvorbehalt for ALG I and the Fünftelregelung for the severance), Soli, church tax,
/// minus what the partner would pay alone. Ends with row `tax` and, with a severance, `tax_on_sev`.
pub fn block(b: &mut Builder, it: &Items) {
    let Items { wage, sv, .. } = *it;
    b.row("wk_used", "Werbungskosten-Pauschale", Fmt::Eur, "only in years with wages", |b, c| {
        format!("=IF({}>0,wk,0)", b.x(wage, c))
    });
    b.row(
        "own_zve",
        "Your taxable income",
        Fmt::Eur,
        "wage − Werbungskosten − social security (as Vorsorgeaufwand) − Sonderausgaben − extra deductions, at least 0",
        |b, c| format!("=MAX(0,{}-{}-{}-sa-extra_deduct)", b.x(wage, c), b.x("wk_used", c), b.x(sv, c)),
    );
    b.row(
        "zve",
        "Household taxable income (zvE)",
        Fmt::Eur,
        "plus the partner’s taxable income with joint assessment",
        |b, c| format!("={}+IF(joint,partner_zve,0)", b.x("own_zve", c)),
    );
    est_rows(b, "zve", "zvE", "", |b, c| b.x("zve", c));
    let Some(alg) = it.alg else {
        b.row("income_tax", "Income tax", Fmt::Eur, "", |b, c| format!("={}", b.x("est_zve", c)));
        surcharge_rows(b, "income_tax", "", "");
        user_rows(b, "household_tax", "tax", "");
        return;
    };
    b.row("zve_r", "zvE + ALG I", Fmt::Eur, "base for the tax rate (Progressionsvorbehalt, §32b EStG)", |b, c| {
        format!("={}+{}", b.x("zve", c), b.x(alg, c))
    });
    est_rows(b, "zve_r", "zvE + ALG I", "", |b, c| b.x("zve_r", c));
    b.row(
        "base_tax",
        "Income tax without the severance",
        Fmt::Eur,
        "with ALG I: the average rate on zvE + ALG I, applied to the zvE alone",
        |b, c| {
            let (a, zve, zr) = (b.x(alg, c), b.x("zve", c), b.x("zve_r", c));
            format!("=IF(AND({a}>0,{zve}>0,{zr}>0),{}/{zr}*{zve},{})", b.x("est_zve_r", c), b.x("est_zve", c))
        },
    );
    let sev = it.sev.expect("a scenario with a severance also has ALG I rows");
    est_rows(b, "sev", "zvE + ALG I + severance (÷ 5 with Fünftelregelung)", "", |b, c| {
        let s = b.x(sev, c);
        format!("{}+IF(fuenftel,{s}/5,{s})", b.x("zve_r", c))
    });
    b.row(
        "sev_income_tax",
        "Income tax on the severance",
        Fmt::Eur,
        "Fünftelregelung (§34 EStG): 5 × the extra tax on a fifth of it; else the extra tax on all of it",
        |b, c| format!("=IF({}>0,IF(fuenftel,5,1)*({}-{}),0)", b.x(sev, c), b.x("est_sev", c), b.x("est_zve_r", c)),
    );
    b.row("income_tax", "Income tax", Fmt::Eur, "", |b, c| {
        format!("={}+{}", b.x("base_tax", c), b.x("sev_income_tax", c))
    });
    surcharge_rows(b, "income_tax", "", "");
    user_rows(b, "household_tax", "tax", "");
    surcharge_rows(b, "base_tax", "_ns", " without the severance");
    user_rows(b, "household_tax_ns", "tax_ns", " without the severance");
    b.row(
        "tax_on_sev",
        "Tax on the severance",
        Fmt::Eur,
        "your tax minus your tax without the severance (incl. Soli and church tax)",
        |b, c| format!("=IF({}>0,{}-{},0)", b.x(sev, c), b.x("tax", c), b.x("tax_ns", c)),
    );
}

fn user_rows(b: &mut Builder, household: &str, key: &str, label: &str) {
    b.row(
        key,
        &format!("Your income tax{label}"),
        Fmt::Eur,
        "household tax minus what your partner would pay alone (joint assessment only)",
        |b, c| format!("={}-partner_tax", b.x(household, c)),
    );
}

/// The tax block for the partner alone (`household_tax` with the user left out), single column.
pub fn partner_block(b: &mut Builder) {
    b.row("partner_alone_zve", "Partner’s taxable income (joint only)", Fmt::Eur, "", |_, _| {
        "=IF(joint,partner_zve,0)".into()
    });
    est_rows(b, "partner", "the partner’s income alone", "", |b, c| b.x("partner_alone_zve", c));
    surcharge_rows(b, "est_partner", "_partner", " of the partner alone");
    b.total(
        "partner_tax",
        "Partner’s tax alone",
        Fmt::Eur,
        "subtracted from the household tax, so you carry the extra tax your income causes",
        |b, c| format!("={}", b.x("household_tax_partner", c)),
    );
    b.name("partner_tax");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::book::{Content, Style};
    use crate::export::eval::{evaluate, Value};
    use crate::tax::{est, tarif};

    #[test]
    fn tariff_formula_matches_the_code() {
        let incomes = [
            0.0, 5000.0, 12348.0, 12349.0, 15000.0, 17799.0, 17800.0, 40000.0, 69878.0, 69879.0, 100000.0, 277825.0,
            277826.0, 400000.0, 1e6,
        ];
        for joint in [false, true] {
            let mut b = Builder::new();
            let s = b.add_sheet("Inputs", 1);
            b.on(s);
            b.put("joint", "", Style::Input, Fmt::General, "", |_| Content::Bool(joint));
            b.name("joint");
            for (name, v) in [
                ("t_free", 12348.0),
                ("t_z1", 17799.0),
                ("t_z2", 69878.0),
                ("t_z3", 277825.0),
                ("t_a1", 914.51),
                ("t_b1", 1400.0),
                ("t_a2", 173.1),
                ("t_b2", 2397.0),
                ("t_c2", 1034.87),
                ("t_r3", 0.42),
                ("t_d3", 11135.63),
                ("t_r4", 0.45),
                ("t_d4", 19470.38),
            ] {
                b.put(name, name, Style::Constant, Fmt::General, "", |_| Content::Number(v));
                b.name(name);
            }
            for (i, x) in incomes.iter().enumerate() {
                b.put(&format!("in{i}"), "", Style::Input, Fmt::Eur, "", |_| Content::Number(*x + 0.6));
                est_rows(&mut b, &format!("e{i}"), "", "", |b, c| b.x(&format!("in{i}"), c));
            }
            let v = evaluate(&b.book).unwrap();
            for (i, x) in incomes.iter().enumerate() {
                let got = v[&b.addr(s, &format!("est_e{i}"), 0)].clone();
                let want = est(x + 0.6, joint);
                assert_eq!(got, Value::Num(want), "income {x}, joint {joint}");
                assert!(!joint || want == 2.0 * tarif((x + 0.6) / 2.0));
            }
        }
    }
}
