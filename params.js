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

  // ---- Rust/WASM core ------------------------------------------------------
  // The computation lives in core/src/lib.rs; this loads core.wasm.js (built by build.sh)
  // and calls `analyze` (JSON in, JSON out).
  function instantiate(bytes) {
    return WebAssembly.instantiate(bytes, {}).then(r => r.instance.exports);
  }
  function loadCore(b64) {
    const bin = typeof atob === 'function' ? Uint8Array.from(atob(b64), c => c.charCodeAt(0))
                                           : Uint8Array.from(Buffer.from(b64, 'base64'));
    return instantiate(bin);
  }
  function makeAnalyze(x) {
    const enc = new TextEncoder(), dec = new TextDecoder();
    return P => {
      const input = enc.encode(JSON.stringify(P));
      const ptr = x.alloc(input.length);
      new Uint8Array(x.memory.buffer, ptr, input.length).set(input);
      const out = x.analyze(ptr, input.length);
      const res = JSON.parse(dec.decode(new Uint8Array(x.memory.buffer, out, x.out_len())));
      x.dealloc(ptr, input.length);
      if (res.error) throw new Error(res.error);
      return res;
    };
  }

  const api = { SPEC, defaults, normalize, loadCore, makeAnalyze };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.Model = api;
})(typeof window !== 'undefined' ? window : globalThis);
