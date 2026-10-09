//! An investment portfolio with German fund taxation (Teilfreistellung, Sparer-Pauschbetrag).

use crate::params::Params;
use crate::tax::cap_tax_rate;

/// Portfolio: market value `w`, cost basis `b`, remaining Sparer-Pauschbetrag `pb` of the year.
pub struct Port {
    pub w: f64,
    b: f64,
    pb: f64,
}

impl Port {
    pub fn new(p: &Params) -> Self {
        Port { w: p.start_wealth, b: p.start_wealth, pb: 0.0 }
    }

    /// Growth over `years` (one calendar year or part of it) at the yearly gross market return `ret`
    /// (costs are deducted here). Starts a new tax year, so the Sparer-Pauschbetrag is available again.
    pub fn grow(&mut self, p: &Params, ret: f64, years: f64) {
        let r = ret - p.ter;
        self.pb = p.sparer_total();
        if self.w < 0.0 {
            self.w *= (1.0 + p.debt_rate).powf(years);
            self.b = self.w;
            return;
        }
        let gain = self.w * ((1.0 + r).max(0.0).powf(years) - 1.0);
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
    pub fn flow(&mut self, p: &Params, scaled: f64, raw: f64) {
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
    pub fn liquidate(&self, p: &Params) -> f64 {
        if self.w <= 0.0 {
            return self.w;
        }
        let taxable = ((self.w - self.b) * (1.0 - p.tf) - p.sparer_total()).max(0.0);
        self.w - taxable * cap_tax_rate(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;

    #[test]
    fn growth_over_part_of_a_year_compounds() {
        let mut p = test_defaults();
        p.start_wealth = 100_000.0;
        let mut port = Port::new(&p);
        port.grow(&p, 0.06, 0.5);
        assert!((port.w - 100_000.0 * 1.058f64.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn debt_over_part_of_a_year_compounds() {
        let mut p = test_defaults();
        p.start_wealth = -10_000.0;
        let mut port = Port::new(&p);
        port.grow(&p, 0.06, 0.25);
        assert!((port.w + 10_000.0 * 1.06f64.powf(0.25)).abs() < 1e-6);
    }
}
