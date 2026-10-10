//! The Inputs sheet: every calculator input and every constant of the code as a named cell.

use serde_json::Value as Json;

use super::book::{Content, Fmt, Style};
use super::grid::Builder;
use crate::params::Params;

/// One calculator input: its JSON key in `Params`, its defined name in the workbook and how it is shown.
pub struct Input {
    pub key: &'static str,
    pub name: &'static str,
    pub label: &'static str,
    pub fmt: Fmt,
    /// Sets the layout of the workbook (number of year columns): editing it needs a re-export.
    pub fixed: bool,
}

const fn input(key: &'static str, name: &'static str, label: &'static str, fmt: Fmt) -> Input {
    Input { key, name, label, fmt, fixed: false }
}

const fn fixed(key: &'static str, name: &'static str, label: &'static str, fmt: Fmt) -> Input {
    Input { key, name, label, fmt, fixed: true }
}

/// Groups of inputs, as in the calculator.
pub const INPUTS: &[(&str, &[Input])] = &[
    (
        "Current job",
        &[
            input("salary", "salary", "Gross salary p.a.", Fmt::Eur),
            input("stayGrowth", "stay_growth", "Salary growth p.a. (if you stay)", Fmt::Pct),
        ],
    ),
    (
        "Severance",
        &[
            input("severance", "severance", "Severance (gross)", Fmt::Eur),
            fixed("startMonth", "start_month", "Current month (1 = January)", Fmt::Int),
            input("exitMonth", "exit_month", "Months until you leave (salary still paid)", Fmt::Int),
            input("payDelay", "pay_delay", "Severance paid X months after leaving", Fmt::Int),
            input("fuenftel", "fuenftel", "Use Fünftelregelung (§34 EStG)", Fmt::General),
        ],
    ),
    (
        "After leaving",
        &[
            input("searchMonths", "search_months", "Months until new job starts", Fmt::Int),
            input("algMonths", "alg_months", "Max. months of unemployment benefit (ALG I)", Fmt::Int),
            input("algRate", "alg_rate", "ALG I rate", Fmt::Pct),
            input("newSalary", "new_salary", "New job gross salary p.a.", Fmt::Eur),
            input("newGrowth", "new_growth", "New job salary growth p.a.", Fmt::Pct),
        ],
    ),
    (
        "Market & life",
        &[
            fixed("horizon", "horizon", "Horizon (years)", Fmt::Int),
            input("spending", "spending", "Net spending p.a. (today’s money)", Fmt::Eur),
            input("inflation", "inflation", "Inflation p.a.", Fmt::Pct),
            input("startWealth", "start_wealth", "Existing investments (both scenarios)", Fmt::Eur),
            input("ret", "ret", "Gross return p.a.", Fmt::Pct),
            input("ter", "ter", "Costs (TER) p.a.", Fmt::Pct),
            input("debtRate", "debt_rate", "Interest if wealth goes negative", Fmt::Pct),
        ],
    ),
    (
        "Pension",
        &[
            input("pensionInWealth", "pension_in_wealth", "Count pension rights in final wealth", Fmt::General),
            input("age", "age", "Your age today", Fmt::Int),
            input("retirementAge", "retirement_age", "Retirement age", Fmt::Int),
            input("lifeExpectancy", "life_expectancy", "Life expectancy at retirement", Fmt::Factor),
            input("pensionGrowth", "pension_growth", "Wage and pension growth p.a.", Fmt::Pct),
            input("pensionDiscount", "pension_discount", "Discount rate for pensions p.a.", Fmt::Pct),
            input("pensionTax", "pension_tax", "Tax rate in retirement", Fmt::Pct),
        ],
    ),
    (
        "Taxes",
        &[
            input("joint", "joint", "Joint assessment (Splitting)", Fmt::General),
            input("partnerZve", "partner_zve", "Partner taxable income p.a. (joint only)", Fmt::Eur),
            input("church", "church", "Church tax rate", Fmt::Pct),
            input("extraDeduct", "extra_deduct", "Extra deductions p.a. (Werbungskosten etc.)", Fmt::Eur),
            input("tf", "tf", "Teilfreistellung on fund gains", Fmt::Pct),
            input("deferred", "deferred", "Accumulating fund (tax at sale)", Fmt::General),
        ],
    ),
    (
        "Social security",
        &[
            input("zusatz", "zusatz", "KV Zusatzbeitrag (total)", Fmt::Pct),
            input("pvRate", "pv_rate", "Pflegeversicherung (employee share)", Fmt::Pct),
        ],
    ),
    (
        "Constants 2026",
        &[
            input("bbgRV", "bbg_rv", "Beitragsbemessungsgrenze RV/AV", Fmt::Eur),
            input("bbgKV", "bbg_kv", "Beitragsbemessungsgrenze KV/PV", Fmt::Eur),
            input("rentenwert", "rentenwert", "Rentenwert (per point and month)", Fmt::Eur),
            input("avgEarnings", "avg_earnings", "Durchschnittsentgelt (provisional)", Fmt::Eur),
            input("wk", "wk", "Werbungskosten-Pauschale", Fmt::Eur),
            input("sparer", "sparer", "Sparer-Pauschbetrag (single)", Fmt::Eur),
            input("soliFree", "soli_free", "Soli Freigrenze (single)", Fmt::Eur),
        ],
    ),
];

/// A constant of the calculator's code: name, label, value (number or formula), format, source.
type Constant = (&'static str, &'static str, &'static str, Fmt, &'static str);

const CONSTANTS: &[(&str, &[Constant])] = &[
    (
        "Income tax tariff 2026 (§32a EStG)",
        &[
            ("t_free", "Grundfreibetrag: no tax up to", "12348", Fmt::Eur, "zone 1"),
            ("t_z1", "End of zone 2", "17799", Fmt::Eur, "zone 2: (a1·y + b1)·y, y = (x − Grundfreibetrag) / 10,000"),
            (
                "t_z2",
                "End of zone 3",
                "69878",
                Fmt::Eur,
                "zone 3: (a2·z + b2)·z + c2, z = (x − end of zone 2) / 10,000",
            ),
            ("t_z3", "End of zone 4", "277825", Fmt::Eur, "zone 4: 42 % · x − d3; zone 5 above: 45 % · x − d4"),
            ("t_a1", "Zone 2: a1", "914.51", Fmt::Factor, ""),
            ("t_b1", "Zone 2: b1", "1400", Fmt::Factor, ""),
            ("t_a2", "Zone 3: a2", "173.1", Fmt::Factor, ""),
            ("t_b2", "Zone 3: b2", "2397", Fmt::Factor, ""),
            ("t_c2", "Zone 3: c2", "1034.87", Fmt::Factor, ""),
            ("t_r3", "Zone 4: rate", "0.42", Fmt::Pct, ""),
            ("t_d3", "Zone 4: d3", "11135.63", Fmt::Factor, ""),
            ("t_r4", "Zone 5: rate", "0.45", Fmt::Pct, ""),
            ("t_d4", "Zone 5: d4", "19470.38", Fmt::Factor, ""),
        ],
    ),
    (
        "Taxes and social security",
        &[
            ("soli_rate", "Soli rate", "0.055", Fmt::Pct, "of income tax"),
            (
                "soli_cap",
                "Soli phase-in rate above the Freigrenze",
                "0.119",
                Fmt::Pct,
                "Soli ≤ 11.9 % × (tax − Freigrenze)",
            ),
            ("sa_single", "Sonderausgaben-Pauschbetrag (single)", "36", Fmt::Eur, ""),
            ("sa_joint", "Sonderausgaben-Pauschbetrag (joint)", "72", Fmt::Eur, ""),
            ("abgeltung", "Abgeltungsteuer", "0.25", Fmt::Pct, "on capital gains, before Soli and church tax"),
            ("rv_rate", "Pension insurance, employee share", "0.093", Fmt::Pct, ""),
            ("av_rate", "Unemployment insurance, employee share", "0.013", Fmt::Pct, ""),
            ("kv_rate", "Health insurance, employee share (without Zusatzbeitrag)", "0.073", Fmt::Pct, ""),
        ],
    ),
    (
        "Statutory pension",
        &[
            ("alg_points_share", "Share of the old salary that earns points on ALG I", "0.8", Fmt::Pct, "§166 SGB VI"),
            (
                "retiree_pv_extra",
                "Retirees’ extra care contribution",
                "0.018",
                Fmt::Pct,
                "retirees pay the full care rate",
            ),
            ("tax_share_base", "Taxable share of a pension starting in 2023", "0.825", Fmt::Pct, "§22 EStG"),
            (
                "tax_share_step",
                "Rise of the taxable share per later starting year",
                "0.005",
                Fmt::Pct,
                "100 % from 2058",
            ),
            ("tax_share_year", "Base year of the taxable share", "2023", Fmt::Int, ""),
            ("this_year", "Calendar year of year 1", "2026", Fmt::Int, "the year of the 2026 constants"),
            (
                "gompertz_slope",
                "Gompertz slope",
                "=LN(2)/7",
                Fmt::Factor,
                "adult mortality doubles about every 7 years of age",
            ),
            ("max_age", "Maximum age", "120", Fmt::Int, "nobody is assumed to live longer"),
        ],
    ),
];

/// The values of `p` by JSON key.
pub fn values(p: &Params) -> serde_json::Map<String, Json> {
    match serde_json::to_value(p) {
        Ok(Json::Object(m)) => m,
        _ => unreachable!("Params serializes to an object"),
    }
}

/// A JSON input value as cell content.
pub fn content(v: &Json) -> Content {
    match v {
        Json::Bool(b) => Content::Bool(*b),
        Json::Number(n) => Content::Number(n.as_f64().unwrap_or(0.0)),
        _ => unreachable!("inputs are numbers or booleans"),
    }
}

pub fn build(b: &mut Builder, s: usize, p: &Params) {
    b.on(s);
    b.text("Inputs", Style::Title);
    b.text(
        "Blue: your inputs from the calculator, edit them to recompute the workbook. Orange: fixed at export \
         (they set the year columns), re-export from the calculator to change them.",
        Style::Note,
    );
    let values = values(p);
    for (group, inputs) in INPUTS {
        b.heading(group);
        for i in *inputs {
            let (style, note) = if i.fixed {
                (Style::Fixed, "Fixed at export: sets the year columns. Re-export to change.")
            } else {
                (Style::Input, "")
            };
            let v = content(&values[i.key]);
            b.put(i.name, i.label, style, i.fmt, note, |_| v.clone());
            b.name(i.name);
        }
    }
    b.heading("Constants from the calculator’s code");
    b.text("Grey: legal and model constants. You can edit them too.", Style::Note);
    for (group, constants) in CONSTANTS {
        b.heading(group);
        for &(name, label, value, fmt, note) in *constants {
            let v = match value.strip_prefix('=') {
                Some(_) => Content::Formula(value.into()),
                None => Content::Number(value.parse().unwrap()),
            };
            b.put(name, label, Style::Constant, fmt, note, |_| v.clone());
            b.name(name);
        }
    }
    b.freeze(1, 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults_json;

    #[test]
    fn every_parameter_is_an_input_exactly_once() {
        let mut keys: Vec<&str> = INPUTS.iter().flat_map(|(_, g)| g.iter().map(|i| i.key)).collect();
        keys.sort();
        let mut expected: Vec<String> = test_defaults_json().as_object().unwrap().keys().cloned().collect();
        expected.sort();
        assert_eq!(keys, expected);
    }

    #[test]
    fn names_cannot_be_mistaken_for_cell_references() {
        let names = INPUTS.iter().flat_map(|(_, g)| g.iter().map(|i| i.name));
        let constants = CONSTANTS.iter().flat_map(|(_, g)| g.iter().map(|c| c.0));
        for n in names.chain(constants) {
            assert!(n.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()), "{n}");
            assert!(n.len() > 1, "{n}: single letters are reserved in Excel");
        }
    }
}
