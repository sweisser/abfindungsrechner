// Parameter spec / defaults for the UI plus the loader for the Rust/WASM core.
// Usable in browser (window.Model) and node (require).
(function (root) {
  'use strict';

  // ---- Parameter spec (single source of truth for defaults + UI) ----------
  // pct: entered as percent in the UI, converted to fraction in normalize().
  const SPEC = [
    { g: 'Current job', k: 'salary', label: 'Gross salary p.a.', def: 90000, step: 1000, unit: '€' },
    { g: 'Current job', k: 'stayGrowth', label: 'Salary growth p.a. (if you stay)', def: 2, step: 0.1, unit: '%', pct: 1 },
    { g: 'Severance', k: 'severance', label: 'Severance (gross)', def: 80000, step: 5000, unit: '€' },
    { g: 'Severance', k: 'exitMonth', label: 'Months until you leave (salary still paid)', def: 0, step: 1, unit: 'mo' },
    { g: 'Severance', k: 'payDelay', label: 'Severance paid X months after leaving', def: 0, step: 1, unit: 'mo',
      hint: 'Shifting payout into January of next year can lower the tax.' },
    { g: 'Severance', k: 'fuenftel', label: 'Use Fünftelregelung (§34 EStG)', def: true },
    { g: 'After leaving', k: 'searchMonths', label: 'Months until new job starts', def: 6, step: 1, unit: 'mo' },
    { g: 'After leaving', k: 'algMonths', label: 'Max. months of unemployment benefit (ALG I)', def: 12, step: 1, unit: 'mo',
      hint: 'Depends on age and insurance history; 0 = none.' },
    { g: 'After leaving', k: 'algRate', label: 'ALG I rate', def: 60, step: 1, unit: '%', pct: 1, hint: '60 %, or 67 % with child.' },
    { g: 'After leaving', k: 'newSalary', label: 'New job gross salary p.a.', def: 80000, step: 1000, unit: '€', hint: '0 = never work again (within horizon).' },
    { g: 'After leaving', k: 'newGrowth', label: 'New job salary growth p.a.', def: 2, step: 0.1, unit: '%', pct: 1 },
    { g: 'Market & life', k: 'horizon', label: 'Horizon (years)', def: 10, step: 1, unit: 'y' },
    { g: 'Market & life', k: 'spending', label: 'Net spending p.a. (today’s money)', def: 45000, step: 1000, unit: '€',
      hint: 'Same in both scenarios; surplus is invested, deficit is withdrawn.' },
    { g: 'Market & life', k: 'inflation', label: 'Inflation p.a.', def: 2, step: 0.1, unit: '%', pct: 1 },
    { g: 'Market & life', k: 'startWealth', label: 'Existing investments (both scenarios)', def: 0, step: 5000, unit: '€' },
    { g: 'Market & life', k: 'ret', label: 'Gross return p.a.', def: 6, step: 0.1, unit: '%', pct: 1 },
    { g: 'Market & life', k: 'ter', label: 'Costs (TER) p.a.', def: 0.2, step: 0.05, unit: '%', pct: 1 },
    { g: 'Market & life', k: 'debtRate', label: 'Interest if wealth goes negative', def: 6, step: 0.1, unit: '%', pct: 1 },
    { g: 'Taxes', k: 'joint', label: 'Joint assessment (Splitting)', def: false },
    { g: 'Taxes', k: 'partnerZve', label: 'Partner taxable income p.a. (joint only)', def: 0, step: 1000, unit: '€' },
    { g: 'Taxes', k: 'church', label: 'Church tax rate (0 / 8 / 9)', def: 0, step: 1, unit: '%', pct: 1 },
    { g: 'Taxes', k: 'extraDeduct', label: 'Extra deductions p.a. (Werbungskosten etc.)', def: 0, step: 100, unit: '€' },
    { g: 'Taxes', k: 'tf', label: 'Teilfreistellung on fund gains', def: 30, step: 1, unit: '%', pct: 1, hint: '30 % equity funds, 0 % for interest/bonds.' },
    { g: 'Taxes', k: 'deferred', label: 'Accumulating fund (tax at sale, ignores Vorabpauschale)', def: true,
      hint: 'Off = gains taxed every year (e.g. interest).' },
    { g: 'Social security', k: 'zusatz', label: 'KV Zusatzbeitrag (total)', def: 2.9, step: 0.1, unit: '%', pct: 1 },
    { g: 'Social security', k: 'pvRate', label: 'Pflegeversicherung (employee share)', def: 2.4, step: 0.1, unit: '%', pct: 1, hint: '1.8 % + 0.6 % if childless.' },
    { g: 'Uncertainty', k: 'searchMin', label: 'Months until new job: at best', def: 2, step: 1, unit: 'mo',
      hint: 'The main inputs are the most likely values; these ranges feed the Monte Carlo simulation.' },
    { g: 'Uncertainty', k: 'searchMax', label: 'Months until new job: at worst', def: 18, step: 1, unit: 'mo' },
    { g: 'Uncertainty', k: 'newSalaryMin', label: 'New job salary: at worst', def: 65000, step: 1000, unit: '€' },
    { g: 'Uncertainty', k: 'newSalaryMax', label: 'New job salary: at best', def: 90000, step: 1000, unit: '€' },
    { g: 'Uncertainty', k: 'stayGrowthMin', label: 'Salary growth if you stay: lowest', def: 0, step: 0.1, unit: '%', pct: 1 },
    { g: 'Uncertainty', k: 'stayGrowthMax', label: 'Salary growth if you stay: highest', def: 3, step: 0.1, unit: '%', pct: 1 },
    { g: 'Uncertainty', k: 'newGrowthMin', label: 'New job salary growth: lowest', def: 0, step: 0.1, unit: '%', pct: 1 },
    { g: 'Uncertainty', k: 'newGrowthMax', label: 'New job salary growth: highest', def: 4, step: 0.1, unit: '%', pct: 1 },
    { g: 'Uncertainty', k: 'returnVol', label: 'Return volatility p.a.', def: 16, step: 1, unit: '%', pct: 1,
      hint: 'About 15–18 % for a global equity fund, 0 = returns as expected every year.' },
    { g: 'Uncertainty', k: 'inflationVol', label: 'Inflation volatility p.a.', def: 1, step: 0.1, unit: '%', pct: 1 },
    { g: 'Constants 2026', k: 'bbgRV', label: 'Beitragsbemessungsgrenze RV/AV', def: 101400, step: 100, unit: '€' },
    { g: 'Constants 2026', k: 'bbgKV', label: 'Beitragsbemessungsgrenze KV/PV', def: 69750, step: 100, unit: '€' },
    { g: 'Constants 2026', k: 'wk', label: 'Werbungskosten-Pauschale', def: 1230, step: 10, unit: '€' },
    { g: 'Constants 2026', k: 'sparer', label: 'Sparer-Pauschbetrag (single)', def: 1000, step: 10, unit: '€' },
    { g: 'Constants 2026', k: 'soliFree', label: 'Soli Freigrenze (single)', def: 20350, step: 50, unit: '€' },
  ];

  function defaults() {
    const p = {};
    SPEC.forEach(s => (p[s.k] = s.def));
    return p;
  }

  // Convert UI values (percent) to model values (fractions).
  function normalize(raw) {
    const p = Object.assign(defaults(), raw);
    SPEC.forEach(s => { if (s.pct) p[s.k] = p[s.k] / 100; });
    return p;
  }

  // Three-point range around the most likely value; the bounds widen to include it.
  function range(likely, low, high) {
    return { min: Math.min(low, likely), mode: likely, max: Math.max(high, likely) };
  }

  // Uncertainty spec for the Monte Carlo core, from normalized parameters.
  function uncertainty(P) {
    return {
      searchMonths: range(P.searchMonths, P.searchMin, P.searchMax),
      newSalary: range(P.newSalary, P.newSalaryMin, P.newSalaryMax),
      stayGrowth: range(P.stayGrowth, P.stayGrowthMin, P.stayGrowthMax),
      newGrowth: range(P.newGrowth, P.newGrowthMin, P.newGrowthMax),
      market: { returnVol: P.returnVol, inflationVol: P.inflationVol },
    };
  }

  // ---- Rust/WASM core ------------------------------------------------------
  // The computation lives in core/ (Rust). `just build` compiles it with wasm-bindgen into
  // core.js (JS bindings, global `wasm_bindgen`) and core.wasm.js (the module as base64,
  // so the page also works when opened from disk).
  // Resolves to { analyze(P), monteCarlo(P, runs, seed) }; errors are thrown as exceptions.
  function loadCore(bindings, b64) {
    const bytes = typeof atob === 'function' ? Uint8Array.from(atob(b64), c => c.charCodeAt(0))
                                             : Uint8Array.from(Buffer.from(b64, 'base64'));
    return bindings({ module_or_path: bytes }).then(() => ({
      analyze: P => JSON.parse(bindings.analyze(JSON.stringify(P))),
      monteCarlo: (P, runs, seed) => JSON.parse(bindings.monteCarlo(JSON.stringify(
        { params: P, uncertainty: uncertainty(P), runs, seed }))),
    }));
  }

  const api = { SPEC, defaults, normalize, uncertainty, loadCore };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.Model = api;
})(typeof window !== 'undefined' ? window : globalThis);
