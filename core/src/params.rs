//! Model parameters as entered in the UI.

use serde::{Deserialize, Serialize};

/// Model parameters. Rates are fractions (0.06 = 6 %), money in EUR, months as numbers.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    /// Calendar month (1–12) in which the simulation starts, i.e. today.
    pub start_month: f64,
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
    /// Age today, in years.
    pub age: f64,
    pub retirement_age: f64,
    /// Remaining life expectancy at the retirement age, in years.
    pub life_expectancy: f64,
    /// Monthly pension per Entgeltpunkt today (aktueller Rentenwert).
    pub rentenwert: f64,
    /// Durchschnittsentgelt of the current year: a salary this high earns one Entgeltpunkt per year.
    pub avg_earnings: f64,
    /// Yearly growth of average earnings, the Rentenwert and the pension contribution ceiling.
    pub pension_growth: f64,
    /// Yearly rate at which future pension payments are discounted to the end of the horizon.
    pub pension_discount: f64,
    /// Marginal income tax rate in retirement (incl. Soli and church tax).
    pub pension_tax: f64,
    /// Add the value of the pension rights earned within the horizon to the final wealth.
    pub pension_in_wealth: bool,
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
        "startMonth": 1.0, "salary": 90000.0, "stayGrowth": 0.02, "severance": 80000.0, "exitMonth": 0.0, "payDelay": 0.0,
        "fuenftel": true, "searchMonths": 6.0, "algMonths": 12.0, "algRate": 0.6, "newSalary": 80000.0,
        "newGrowth": 0.02, "horizon": 10.0, "spending": 45000.0, "inflation": 0.02, "startWealth": 0.0,
        "ret": 0.06, "ter": 0.002, "debtRate": 0.06, "joint": false, "partnerZve": 0.0, "church": 0.0,
        "extraDeduct": 0.0, "tf": 0.3, "deferred": true, "zusatz": 0.029, "pvRate": 0.024,
        "bbgRV": 101400.0, "bbgKV": 69750.0, "wk": 1230.0, "sparer": 1000.0, "soliFree": 20350.0,
        "age": 45.0, "retirementAge": 67.0, "lifeExpectancy": 20.0, "rentenwert": 42.52, "avgEarnings": 51944.0,
        "pensionGrowth": 0.025, "pensionDiscount": 0.03, "pensionTax": 0.25, "pensionInWealth": false
    })
}
