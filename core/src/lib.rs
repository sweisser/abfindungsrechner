//! Severance-vs-stay simulation (German tax rules, tariff 2026).
//!
//! Pure computation; the browser talks to it through the tiny C ABI at the bottom
//! (JSON in, JSON out), see `analyze`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- parameters

/// Model parameters. Rates are fractions (0.06 = 6 %), money in EUR, months as numbers.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub salary: f64,
    pub stay_growth: f64,
    pub severance: f64,
    pub exit_month: f64,
    pub pay_delay: f64,
    pub fuenftel: bool,
    pub search_months: f64,
    pub alg_months: f64,
    pub alg_rate: f64,
    pub new_salary: f64,
    pub new_growth: f64,
    pub horizon: f64,
    pub spending: f64,
    pub inflation: f64,
    pub start_wealth: f64,
    pub ret: f64,
    pub ter: f64,
    pub debt_rate: f64,
    pub joint: bool,
    pub partner_zve: f64,
    pub church: f64,
    pub extra_deduct: f64,
    pub tf: f64,
    pub deferred: bool,
    pub zusatz: f64,
    pub pv_rate: f64,
    #[serde(rename = "bbgRV")]
    pub bbg_rv: f64,
    #[serde(rename = "bbgKV")]
    pub bbg_kv: f64,
    pub wk: f64,
    pub sparer: f64,
    pub soli_free: f64,
}

// --------------------------------------------------------------------- taxes

/// §32a EStG, basic tariff 2026.
pub fn tarif(x: f64) -> f64 {
    let x = x.max(0.0).floor();
    if x <= 12348.0 {
        0.0
    } else if x <= 17799.0 {
        let y = (x - 12348.0) / 1e4;
        (914.51 * y + 1400.0) * y
    } else if x <= 69878.0 {
        let z = (x - 17799.0) / 1e4;
        (173.1 * z + 2397.0) * z + 1034.87
    } else if x <= 277825.0 {
        0.42 * x - 11135.63
    } else {
        0.45 * x - 19470.38
    }
}

/// Income tax; joint assessment uses the splitting tariff.
fn est(x: f64, joint: bool) -> f64 {
    if joint {
        2.0 * tarif(x / 2.0)
    } else {
        tarif(x)
    }
}

fn soli(e: f64, p: &Params) -> f64 {
    let thr = p.soli_free * if p.joint { 2.0 } else { 1.0 };
    if e <= thr {
        0.0
    } else {
        (0.055 * e).min(0.119 * (e - thr))
    }
}

/// A block of months with the same monthly gross salary.
struct Seg {
    m: f64,
    n: f64,
}

/// Employee share of social security (RV, AV, KV, PV), month by month against the ceilings.
fn social_security(segs: &[Seg], p: &Params) -> f64 {
    let rvav = 0.093 + 0.013;
    let kvpv = 0.073 + p.zusatz / 2.0 + p.pv_rate;
    segs.iter()
        .map(|s| s.n * (rvav * s.m.min(p.bbg_rv / 12.0) + kvpv * s.m.min(p.bbg_kv / 12.0)))
        .sum()
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
    let base = if a > 0.0 && zve > 0.0 && zve_r > 0.0 {
        est(zve_r, p.joint) / zve_r * zve
    } else {
        est(zve, p.joint)
    };
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

fn year_cash(p: &Params, segs: &[Seg], alg: f64, sev: f64) -> YearRow {
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

// ----------------------------------------------------------------- portfolio

/// Total capital-gains tax rate: Abgeltungsteuer 25 % + Soli + church tax (church tax is deductible).
fn cap_tax_rate(p: &Params) -> f64 {
    let k = p.church;
    0.25 / (1.0 + 0.25 * k) * (1.0 + 0.055 + k)
}

/// Portfolio: market value `w`, cost basis `b`, remaining Sparer-Pauschbetrag `pb` of the year.
struct Port {
    w: f64,
    b: f64,
    pb: f64,
}

impl Port {
    fn new(p: &Params) -> Self {
        Port { w: p.start_wealth, b: p.start_wealth, pb: 0.0 }
    }

    fn grow_year(&mut self, p: &Params) {
        let r = p.ret - p.ter;
        self.pb = p.sparer * if p.joint { 2.0 } else { 1.0 };
        if self.w < 0.0 {
            self.w *= 1.0 + p.debt_rate;
            self.b = self.w;
            return;
        }
        let gain = self.w * r;
        if p.deferred || gain <= 0.0 {
            self.w += gain;
            if !p.deferred {
                self.b = self.w;
            }
            return;
        }
        let taxable = (gain * (1.0 - p.tf) - self.pb).max(0.0);
        self.pb = (self.pb - gain * (1.0 - p.tf)).max(0.0);
        self.w += gain - taxable * cap_tax_rate(p);
        self.b = self.w;
    }

    /// `scaled`: amount at year end (incl. growth), `raw`: cash actually moved (for the basis).
    fn flow(&mut self, p: &Params, scaled: f64, raw: f64) {
        if scaled >= 0.0 {
            self.w += scaled;
            self.b += raw;
            if self.w < 0.0 {
                self.b = self.w;
            }
            return;
        }
        let need = -scaled;
        let t = cap_tax_rate(p);
        if self.w <= 0.0 {
            self.w -= need;
            self.b = self.w;
            return;
        }
        let g = (1.0 - self.b / self.w).max(0.0) * (1.0 - p.tf);
        let mut x = need; // amount to sell so that, after tax, `need` remains
        if need * g > self.pb {
            x = (need - t * self.pb) / (1.0 - t * g);
        }
        if x >= self.w {
            let taxable = ((self.w - self.b) * (1.0 - p.tf) - self.pb).max(0.0);
            let net = self.w - taxable * t;
            self.w = if net >= need { net - need } else { -(need - net) };
            self.b = self.w;
            return;
        }
        self.pb = (self.pb - x * g).max(0.0);
        self.b *= 1.0 - x / self.w;
        self.w -= x;
    }

    /// Value after selling everything at the end of the horizon.
    fn liquidate(&self, p: &Params) -> f64 {
        if self.w <= 0.0 {
            return self.w;
        }
        let pb_all = p.sparer * if p.joint { 2.0 } else { 1.0 };
        let taxable = ((self.w - self.b) * (1.0 - p.tf) - pb_all).max(0.0);
        self.w - taxable * cap_tax_rate(p)
    }
}

// ---------------------------------------------------------------- simulation

fn overlap(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (b.min(d) - a.max(c)).max(0.0)
}

fn alg_monthly(p: &Params) -> f64 {
    let m = (p.salary / 12.0).min(p.bbg_rv / 12.0);
    let c = year_cash(p, &[Seg { m, n: 12.0 }], 0.0, 0.0);
    p.alg_rate * (c.net / 12.0)
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sim {
    pub a: Vec<YearRow>,
    pub b: Vec<YearRow>,
    pub alg_monthly: f64,
    pub n: u32,
    pub deflator: f64,
    pub final_a: f64,
    pub final_b: f64,
    pub diff: f64,
    pub real_a: f64,
    pub real_b: f64,
    pub real_diff: f64,
    pub sev_year: u32,
    pub sev_tax: f64,
    pub sev_net: f64,
}

pub fn simulate(p: &Params) -> Sim {
    let n = p.horizon.round().max(0.0) as u32;
    let e = p.exit_month.round();
    let j = p.search_months.round();
    let a = p.alg_months.round().min(j);
    let pay = e + p.pay_delay.round();
    let ky = (pay / 12.0).floor() as u32 + 1;
    let pay_frac = (pay % 12.0) / 12.0;
    let alg_m = alg_monthly(p);
    let r = p.ret - p.ter;

    let (mut sa, mut sb) = (Port::new(p), Port::new(p));
    let (mut rows_a, mut rows_b) = (Vec::new(), Vec::new());

    for k in 1..=n {
        let (y0, y1) = (12.0 * (k - 1) as f64, 12.0 * k as f64);
        let spend = p.spending * (1.0 + p.inflation).powi(k as i32 - 1);
        let stay_m = p.salary * (1.0 + p.stay_growth).powi(k as i32 - 1) / 12.0;
        let new_m = p.new_salary * (1.0 + p.new_growth).powi(k as i32 - 1) / 12.0;

        // A: stay
        let mut ca = year_cash(p, &[Seg { m: stay_m, n: 12.0 }], 0.0, 0.0);
        sa.grow_year(p);
        sa.flow(p, (ca.net - spend) * (1.0 + r).powf(0.5), ca.net - spend);
        ca.year = k;
        ca.spend = spend;
        ca.wealth = sa.w;
        rows_a.push(ca);

        // B: take the severance
        let segs = [
            Seg { m: stay_m, n: overlap(0.0, e, y0, y1) },
            Seg { m: new_m, n: if p.new_salary > 0.0 { overlap(e + j, f64::INFINITY, y0, y1) } else { 0.0 } },
        ];
        let alg_mo = overlap(e, e + a, y0, y1);
        let sev = if k == ky { p.severance } else { 0.0 };
        let mut cb = year_cash(p, &segs, alg_mo * alg_m, sev);
        let sev_net = sev - cb.tax_on_sev;
        let rest = cb.net - sev + cb.tax_on_sev - spend;
        sb.grow_year(p);
        sb.flow(p, sev_net * (1.0 + r).powf(1.0 - pay_frac), sev_net);
        sb.flow(p, rest * (1.0 + r).powf(0.5), rest);
        cb.year = k;
        cb.spend = spend;
        cb.wealth = sb.w;
        cb.sev_net = sev_net;
        rows_b.push(cb);
    }

    let defl = (1.0 + p.inflation).powi(n as i32);
    let (fa, fb) = (sa.liquidate(p), sb.liquidate(p));
    let sev_row = rows_b.get((ky.min(n) as usize).wrapping_sub(1));
    Sim {
        alg_monthly: alg_m,
        n,
        deflator: defl,
        final_a: fa,
        final_b: fb,
        diff: fb - fa,
        real_a: fa / defl,
        real_b: fb / defl,
        real_diff: (fb - fa) / defl,
        sev_year: ky,
        sev_tax: sev_row.map_or(0.0, |r| r.tax_on_sev),
        sev_net: sev_row.map_or(0.0, |r| r.sev_net),
        a: rows_a,
        b: rows_b,
    }
}

// ------------------------------------------------------------------ analysis

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Breakeven {
    /// `None`: B never catches up within the search range.
    pub value: Option<f64>,
    /// B already wins at the lower bound of the search range.
    pub at_lower_bound: bool,
}

/// Bisection on one parameter (diff is monotone increasing in it) until B == A.
fn breakeven(p: &Params, set: impl Fn(&mut Params, f64), lo: f64, hi: f64) -> Breakeven {
    let f = |v: f64| {
        let mut q = p.clone();
        set(&mut q, v);
        simulate(&q).diff
    };
    let (mut lo, mut hi) = (lo, hi);
    if f(lo) >= 0.0 {
        return Breakeven { value: Some(lo), at_lower_bound: true };
    }
    if f(hi) < 0.0 {
        return Breakeven { value: None, at_lower_bound: false };
    }
    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Breakeven { value: Some((lo + hi) / 2.0), at_lower_bound: false }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Grid {
    pub salaries: Vec<f64>,
    pub months: Vec<f64>,
    /// `cells[row][col]`: real final wealth of B minus A.
    pub cells: Vec<Vec<f64>>,
}

fn sensitivity(p: &Params) -> Grid {
    let salaries: Vec<f64> =
        [0.6, 0.8, 1.0, 1.2].iter().map(|m| (p.new_salary * m / 1000.0).round() * 1000.0).collect();
    let months = vec![0.0, 3.0, 6.0, 12.0, 18.0, 24.0];
    let cells = salaries
        .iter()
        .map(|&s| {
            months
                .iter()
                .map(|&m| {
                    let mut q = p.clone();
                    q.new_salary = s;
                    q.search_months = m;
                    simulate(&q).real_diff
                })
                .collect()
        })
        .collect();
    Grid { salaries, months, cells }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub sim: Sim,
    pub be_new_salary: Breakeven,
    pub be_severance: Breakeven,
    pub grid: Grid,
}

pub fn analyze_params(p: &Params) -> Analysis {
    Analysis {
        sim: simulate(p),
        be_new_salary: breakeven(p, |q, v| q.new_salary = v, 0.0, 500_000.0),
        be_severance: breakeven(p, |q, v| q.severance = v, 0.0, 3e6),
        grid: sensitivity(p),
    }
}

/// JSON in (`Params`), JSON out (`Analysis` or `{"error": ...}`).
pub fn analyze_json(input: &str) -> String {
    match serde_json::from_str::<Params>(input) {
        Ok(p) => serde_json::to_string(&analyze_params(&p)).unwrap(),
        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string(),
    }
}

// ------------------------------------------------------------------- C ABI
// Minimal glue, no wasm-bindgen: JS allocates a buffer with `alloc`, writes the JSON request,
// calls `analyze`, reads `out_len()` bytes at the returned pointer, frees the input with `dealloc`.

thread_local! {
    static OUT: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[no_mangle]
pub extern "C" fn alloc(n: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(n);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `p`/`n` must come from a previous `alloc(n)`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(p: *mut u8, n: usize) {
    drop(Vec::from_raw_parts(p, 0, n));
}

/// # Safety
/// `p` must point to `n` readable bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn analyze(p: *const u8, n: usize) -> *const u8 {
    let input = std::str::from_utf8(std::slice::from_raw_parts(p, n)).unwrap_or("");
    let out = analyze_json(input).into_bytes();
    OUT.with(|o| {
        *o.borrow_mut() = out;
        o.borrow().as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

// --------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    pub fn defaults() -> Params {
        serde_json::from_value(serde_json::json!({
            "salary": 90000.0, "stayGrowth": 0.02, "severance": 80000.0, "exitMonth": 0.0, "payDelay": 0.0,
            "fuenftel": true, "searchMonths": 6.0, "algMonths": 12.0, "algRate": 0.6, "newSalary": 80000.0,
            "newGrowth": 0.02, "horizon": 10.0, "spending": 45000.0, "inflation": 0.02, "startWealth": 0.0,
            "ret": 0.06, "ter": 0.002, "debtRate": 0.06, "joint": false, "partnerZve": 0.0, "church": 0.0,
            "extraDeduct": 0.0, "tf": 0.3, "deferred": true, "zusatz": 0.029, "pvRate": 0.024,
            "bbgRV": 101400.0, "bbgKV": 69750.0, "wk": 1230.0, "sparer": 1000.0, "soliFree": 20350.0
        }))
        .unwrap()
    }

    #[test]
    fn tariff_reference_points() {
        assert_eq!(tarif(12348.0), 0.0);
        assert!((tarif(17799.0) - 1035.0).abs() < 1.0);
        assert!((tarif(69878.0) - 18213.0).abs() < 2.0);
        assert!((tarif(277825.0) - 105551.0).abs() < 2.0);
    }

    #[test]
    fn identical_scenarios_have_zero_difference() {
        let mut p = defaults();
        p.severance = 0.0;
        p.new_salary = 90000.0;
        p.search_months = 0.0;
        p.alg_months = 0.0;
        assert!(simulate(&p).diff.abs() < 1e-6);
    }

    #[test]
    fn default_case_matches_reference() {
        // Reference values from the original JS implementation.
        let s = simulate(&defaults());
        assert!((s.final_a - 113127.0).abs() < 1.0, "{}", s.final_a);
        assert!((s.final_b - 110892.0).abs() < 1.0, "{}", s.final_b);
        assert!((s.sev_tax - 31222.989).abs() < 0.01);
    }

    #[test]
    fn json_roundtrip_and_error() {
        let ok = analyze_json(&serde_json::to_string(&serde_json::json!({})).unwrap());
        assert!(ok.contains("error"));
    }
}
