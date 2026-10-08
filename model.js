// Severance-vs-stay simulation model (German tax rules, tariff 2026).
// Pure functions, usable in browser (window.Model) and node (require).
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

  // ---- Tax -----------------------------------------------------------------
  // §32a EStG, tariff 2026 (Grundtarif)
  function tarif(x) {
    x = Math.floor(Math.max(0, x));
    if (x <= 12348) return 0;
    if (x <= 17799) { const y = (x - 12348) / 1e4; return (914.51 * y + 1400) * y; }
    if (x <= 69878) { const z = (x - 17799) / 1e4; return (173.1 * z + 2397) * z + 1034.87; }
    if (x <= 277825) return 0.42 * x - 11135.63;
    return 0.45 * x - 19470.38;
  }
  const est = (x, joint) => (joint ? 2 * tarif(x / 2) : tarif(x));

  function soli(e, P) {
    const thr = P.soliFree * (P.joint ? 2 : 1);
    if (e <= thr) return 0;
    return Math.min(0.055 * e, 0.119 * (e - thr));
  }

  // Employee social security for segments [{m: monthly gross, n: months}]
  function socialSecurity(segs, P) {
    const rvav = 0.093 + 0.013;
    const kvpv = 0.073 + P.zusatz / 2 + P.pvRate;
    let sv = 0;
    segs.forEach(s => {
      sv += s.n * (rvav * Math.min(s.m, P.bbgRV / 12) + kvpv * Math.min(s.m, P.bbgKV / 12));
    });
    return sv;
  }

  // Total tax (ESt + Soli + KiSt) for the whole household given user items.
  function householdTax(P, { wage, sv, alg, sev }, userOn) {
    const w = userOn ? wage : 0, s = userOn ? sev : 0, a = userOn ? alg : 0, v = userOn ? sv : 0;
    const wk = w > 0 ? P.wk : 0;
    const sa = P.joint ? 72 : 36;
    const own = userOn ? Math.max(0, w - wk - v - sa - P.extraDeduct) : 0;
    const partner = P.joint ? P.partnerZve : 0;
    const zve = own + partner;
    const zveR = zve + a; // Progressionsvorbehalt base
    const base = a > 0 && zve > 0 && zveR > 0 ? (est(zveR, P.joint) / zveR) * zve : est(zve, P.joint);
    let sevTax = 0;
    if (s > 0) {
      sevTax = P.fuenftel
        ? 5 * (est(zveR + s / 5, P.joint) - est(zveR, P.joint))
        : est(zveR + s, P.joint) - est(zveR, P.joint);
    }
    const e = base + sevTax;
    return { est: e, total: e + soli(e, P) + P.church * e };
  }

  // Tax attributable to the user (marginal vs. partner alone).
  function userTax(P, items) {
    const t1 = householdTax(P, items, true).total;
    const t0 = P.joint ? householdTax(P, items, false).total : 0;
    return t1 - t0;
  }

  function yearCash(P, segs, alg, sev) {
    const wage = segs.reduce((a, s) => a + s.m * s.n, 0);
    const sv = socialSecurity(segs, P);
    const tax = userTax(P, { wage, sv, alg, sev });
    const taxNoSev = sev > 0 ? userTax(P, { wage, sv, alg, sev: 0 }) : tax;
    return { wage, sv, alg, sev, tax, taxOnSev: tax - taxNoSev, net: wage + alg + sev - sv - tax };
  }

  // ---- Portfolio -----------------------------------------------------------
  const capTaxRate = P => {
    const k = P.church;
    const kest = 0.25 / (1 + 0.25 * k);
    return kest * (1 + 0.055 + k);
  };

  function newPort(P) { return { W: P.startWealth, B: P.startWealth, pb: 0 }; }

  function growYear(S, P) {
    const r = P.ret - P.ter;
    const pbAll = P.sparer * (P.joint ? 2 : 1);
    S.pb = pbAll;
    if (S.W < 0) { S.W *= 1 + P.debtRate; S.B = S.W; return; }
    const gain = S.W * r;
    if (P.deferred || gain <= 0) { S.W += gain; if (!P.deferred) S.B = S.W; return; }
    const taxable = Math.max(0, gain * (1 - P.tf) - S.pb);
    S.pb = Math.max(0, S.pb - gain * (1 - P.tf));
    S.W += gain - taxable * capTaxRate(P);
    S.B = S.W;
  }

  // scaled = amount at end of year (incl. half-year growth), raw = cash actually moved
  function flow(S, P, scaled, raw) {
    if (scaled >= 0) {
      S.W += scaled; S.B += raw;
      if (S.W < 0) S.B = S.W;
      return;
    }
    const need = -scaled, t = capTaxRate(P);
    if (S.W <= 0) { S.W -= need; S.B = S.W; return; }
    const g = Math.max(0, 1 - S.B / S.W) * (1 - P.tf);
    let X = need;
    if (need * g > S.pb) X = (need - t * S.pb) / (1 - t * g);
    if (X >= S.W) {
      const taxable = Math.max(0, (S.W - S.B) * (1 - P.tf) - S.pb);
      const net = S.W - taxable * t;
      S.W = net >= need ? net - need : -(need - net);
      S.B = S.W;
      return;
    }
    S.pb = Math.max(0, S.pb - X * g);
    S.B *= 1 - X / S.W;
    S.W -= X;
  }

  function liquidate(S, P) {
    if (S.W <= 0) return S.W;
    const pbAll = P.sparer * (P.joint ? 2 : 1);
    const taxable = Math.max(0, (S.W - S.B) * (1 - P.tf) - pbAll);
    return S.W - taxable * capTaxRate(P);
  }

  // ---- Simulation ------------------------------------------------------------
  const ov = (a, b, c, d) => Math.max(0, Math.min(b, d) - Math.max(a, c));

  function algMonthly(P) {
    const m = Math.min(P.salary / 12, P.bbgRV / 12);
    const c = yearCash(P, [{ m, n: 12 }], 0, 0);
    return P.algRate * (c.net / 12);
  }

  function simulate(P) {
    const N = Math.round(P.horizon);
    const e = Math.round(P.exitMonth), j = Math.round(P.searchMonths);
    const a = Math.min(Math.round(P.algMonths), j);
    const pay = e + Math.round(P.payDelay);
    const ky = Math.floor(pay / 12) + 1;
    const payFrac = (pay % 12) / 12;
    const algM = algMonthly(P);
    const r = P.ret - P.ter;
    const out = { A: [], B: [], algMonthly: algM };
    const SA = newPort(P), SB = newPort(P);
    const rows = { A: [], B: [] };

    for (let k = 1; k <= N; k++) {
      const y0 = 12 * (k - 1), y1 = 12 * k;
      const infl = Math.pow(1 + P.inflation, k - 1);
      const spend = P.spending * infl;
      const stayM = P.salary * Math.pow(1 + P.stayGrowth, k - 1) / 12;
      const newM = P.newSalary * Math.pow(1 + P.newGrowth, k - 1) / 12;

      // A: stay
      const cA = yearCash(P, [{ m: stayM, n: 12 }], 0, 0);
      growYear(SA, P);
      flow(SA, P, (cA.net - spend) * Math.pow(1 + r, 0.5), cA.net - spend);
      rows.A.push(Object.assign({ year: k, spend, wealth: SA.W }, cA));

      // B: take severance
      const segs = [
        { m: stayM, n: ov(0, e, y0, y1) },
        { m: newM, n: P.newSalary > 0 ? ov(e + j, Infinity, y0, y1) : 0 },
      ];
      const algMo = ov(e, e + a, y0, y1);
      const sev = k === ky ? P.severance : 0;
      const cB = yearCash(P, segs, algMo * algM, sev);
      const sevNet = sev - cB.taxOnSev;
      const rest = cB.net - sev + cB.taxOnSev - spend;
      growYear(SB, P);
      flow(SB, P, sevNet * Math.pow(1 + r, 1 - payFrac), sevNet);
      flow(SB, P, rest * Math.pow(1 + r, 0.5), rest);
      rows.B.push(Object.assign({ year: k, spend, wealth: SB.W, sevNet }, cB));
    }
    const dfl = Math.pow(1 + P.inflation, N);
    const fa = liquidate(SA, P), fb = liquidate(SB, P);
    return {
      A: rows.A, B: rows.B, algMonthly: algM, N, deflator: dfl,
      finalA: fa, finalB: fb, diff: fb - fa,
      realA: fa / dfl, realB: fb / dfl, realDiff: (fb - fa) / dfl,
      sevYear: ky,
      sevTax: rows.B[Math.min(ky, N) - 1] ? rows.B[Math.min(ky, N) - 1].taxOnSev : 0,
    };
  }

  // monotone bisection on parameter `key` so that diff == 0
  function breakeven(P, key, lo, hi) {
    const f = v => simulate(Object.assign({}, P, { [key]: v })).diff;
    if (f(lo) >= 0) return { value: lo, note: 'already ≥ 0 at lower bound' };
    if (f(hi) < 0) return { value: null, note: 'not reachable' };
    for (let i = 0; i < 50; i++) {
      const mid = (lo + hi) / 2;
      if (f(mid) < 0) lo = mid; else hi = mid;
    }
    return { value: (lo + hi) / 2 };
  }

  const api = { SPEC, defaults, normalize, tarif, est, simulate, breakeven, yearCash, capTaxRate };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.Model = api;
})(typeof window !== 'undefined' ? window : globalThis);
