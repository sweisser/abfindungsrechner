//! One year of a person's cash flow: wages, social security, ALG I, severance and income tax.

use serde::Serialize;

use crate::params::Params;
use crate::tax::{est, soli};

/// A block of months with the same monthly gross salary.
pub struct Seg {
    pub m: f64,
    pub n: f64,
}

/// Employee share of social security (RV, AV, KV, PV), month by month against the ceilings.
fn social_security(segs: &[Seg], p: &Params) -> f64 {
    let rvav = 0.093 + 0.013;
    let kvpv = 0.073 + p.zusatz / 2.0 + p.pv_rate;
    segs.iter().map(|s| s.n * (rvav * s.m.min(p.bbg_rv / 12.0) + kvpv * s.m.min(p.bbg_kv / 12.0))).sum()
}

struct Items {
    wage: f64,
    sv: f64,
    alg: f64,
    sev: f64,
}

/// Income tax + Soli + church tax of the household; `user_on = false` leaves out the user.
fn household_tax(p: &Params, it: &Items, user_on: bool) -> f64 {
    let (w, s, a, v) = if user_on { (it.wage, it.sev, it.alg, it.sv) } else { (0.0, 0.0, 0.0, 0.0) };
    let wk = if w > 0.0 { p.wk } else { 0.0 };
    let sa = if p.joint { 72.0 } else { 36.0 };
    let own = if user_on { (w - wk - v - sa - p.extra_deduct).max(0.0) } else { 0.0 };
    let partner = if p.joint { p.partner_zve } else { 0.0 };
    let zve = own + partner;
    let zve_r = zve + a; // base for the Progressionsvorbehalt (ALG I)
    let base = if a > 0.0 && zve > 0.0 && zve_r > 0.0 { est(zve_r, p.joint) / zve_r * zve } else { est(zve, p.joint) };
    let sev_tax = if s > 0.0 {
        if p.fuenftel {
            5.0 * (est(zve_r + s / 5.0, p.joint) - est(zve_r, p.joint))
        } else {
            est(zve_r + s, p.joint) - est(zve_r, p.joint)
        }
    } else {
        0.0
    };
    let e = base + sev_tax;
    e + soli(e, p) + p.church * e
}

/// Tax attributable to the user (marginal against the partner alone).
fn user_tax(p: &Params, it: &Items) -> f64 {
    let t1 = household_tax(p, it, true);
    let t0 = if p.joint { household_tax(p, it, false) } else { 0.0 };
    t1 - t0
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct YearRow {
    pub year: u32,
    pub spend: f64,
    pub wealth: f64,
    pub wage: f64,
    pub sv: f64,
    pub alg: f64,
    pub sev: f64,
    pub tax: f64,
    pub tax_on_sev: f64,
    pub net: f64,
    pub sev_net: f64,
}

pub fn year_cash(p: &Params, segs: &[Seg], alg: f64, sev: f64) -> YearRow {
    let wage: f64 = segs.iter().map(|s| s.m * s.n).sum();
    let sv = social_security(segs, p);
    let tax = user_tax(p, &Items { wage, sv, alg, sev });
    let tax_no_sev = if sev > 0.0 { user_tax(p, &Items { wage, sv, alg, sev: 0.0 }) } else { tax };
    YearRow {
        wage,
        sv,
        alg,
        sev,
        tax,
        tax_on_sev: tax - tax_no_sev,
        net: wage + alg + sev - sv - tax,
        ..Default::default()
    }
}

/// Monthly ALG I: rate × net salary, with the salary capped at the RV ceiling.
pub fn alg_monthly(p: &Params) -> f64 {
    let m = (p.salary / 12.0).min(p.bbg_rv / 12.0);
    let c = year_cash(p, &[Seg { m, n: 12.0 }], 0.0, 0.0);
    p.alg_rate * (c.net / 12.0)
}
