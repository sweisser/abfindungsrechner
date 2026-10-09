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

    /// One year of growth at the gross market return `ret` (costs are deducted here).
    pub fn grow_year(&mut self, p: &Params, ret: f64) {
        let r = ret - p.ter;
        self.pb = p.sparer_total();
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
