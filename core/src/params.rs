//! Model parameters as entered in the UI.

use serde::Deserialize;

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

impl Params {
    /// Sparer-Pauschbetrag of the household (doubled for joint assessment).
    pub fn sparer_total(&self) -> f64 {
        self.sparer * if self.joint { 2.0 } else { 1.0 }
    }
}

/// The UI defaults, shared by the tests of all modules.
#[cfg(test)]
pub fn test_defaults() -> Params {
    serde_json::from_value(test_defaults_json()).unwrap()
}

#[cfg(test)]
pub fn test_defaults_json() -> serde_json::Value {
    serde_json::json!({
        "salary": 90000.0, "stayGrowth": 0.02, "severance": 80000.0, "exitMonth": 0.0, "payDelay": 0.0,
        "fuenftel": true, "searchMonths": 6.0, "algMonths": 12.0, "algRate": 0.6, "newSalary": 80000.0,
        "newGrowth": 0.02, "horizon": 10.0, "spending": 45000.0, "inflation": 0.02, "startWealth": 0.0,
        "ret": 0.06, "ter": 0.002, "debtRate": 0.06, "joint": false, "partnerZve": 0.0, "church": 0.0,
        "extraDeduct": 0.0, "tf": 0.3, "deferred": true, "zusatz": 0.029, "pvRate": 0.024,
        "bbgRV": 101400.0, "bbgKV": 69750.0, "wk": 1230.0, "sparer": 1000.0, "soliFree": 20350.0
    })
}
