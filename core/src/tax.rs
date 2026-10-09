//! Income tax (§32a EStG, tariff 2026), Soli and the flat capital-gains tax.

use crate::params::Params;

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
pub fn est(x: f64, joint: bool) -> f64 {
    if joint {
        2.0 * tarif(x / 2.0)
    } else {
        tarif(x)
    }
}

pub fn soli(e: f64, p: &Params) -> f64 {
    let thr = p.soli_free * if p.joint { 2.0 } else { 1.0 };
    if e <= thr {
        0.0
    } else {
        (0.055 * e).min(0.119 * (e - thr))
    }
}

/// Total capital-gains tax rate: Abgeltungsteuer 25 % + Soli + church tax (church tax is deductible).
pub fn cap_tax_rate(p: &Params) -> f64 {
    let k = p.church;
    0.25 / (1.0 + 0.25 * k) * (1.0 + 0.055 + k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tariff_reference_points() {
        assert_eq!(tarif(12348.0), 0.0);
        assert!((tarif(17799.0) - 1035.0).abs() < 1.0);
        assert!((tarif(69878.0) - 18213.0).abs() < 2.0);
        assert!((tarif(277825.0) - 105551.0).abs() < 2.0);
    }
}
