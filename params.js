// Parameter spec / defaults for the UI plus the loader for the Rust/WASM core.
// Usable in browser (window.Model) and node (require).
(function (root) {
  'use strict';

  // ---- Parameter spec (single source of truth for defaults + UI) ----------
  // pct: entered as percent in the UI, converted to fraction in normalize().
  // help: text of the field's help tooltip.
  // of: (Uncertainty only) the main input whose Monte Carlo range or volatility this sets.
  const SPEC = [
    { g: 'Current job', k: 'salary', label: 'Gross salary p.a.', def: 90000, step: 1000, unit: '€',
      help: 'Your current annual gross salary, including bonuses and any 13th month. Scenario A (stay) keeps this salary and raises it every January by the growth rate below. It is also taken as your salary in the months of this year before today, which count for this year’s income tax.' },
    { g: 'Current job', k: 'stayGrowth', label: 'Salary growth p.a. (if you stay)', def: 2, step: 0.1, unit: '%', pct: 1,
      help: 'Average raise you expect each January if you stay. Enter the most likely value. The Monte Carlo simulation varies it within the range under Uncertainty.' },
    { g: 'Severance', k: 'severance', label: 'Severance (gross)', def: 80000, step: 5000, unit: '€',
      help: 'Gross severance (Abfindung) from your termination or settlement agreement. It is subject to income tax but not to social security contributions.' },
    { g: 'Severance', k: 'startMonth', label: 'Current month (1 = January)', def: new Date().getMonth() + 1, step: 1, unit: 'mo',
      help: 'The calendar month you are in now; all other months count from here. Income tax is assessed per calendar year, so this decides which year the severance falls into. Each table row is one calendar year; the first and last can be partial.' },
    { g: 'Severance', k: 'exitMonth', label: 'Months until you leave (salary still paid)', def: 0, step: 1, unit: 'mo',
      help: 'Months from today until your employment ends. Until then you keep your current salary in both scenarios. 0 = you leave right away.' },
    { g: 'Severance', k: 'payDelay', label: 'Severance paid X months after leaving', def: 0, step: 1, unit: 'mo',
      help: 'Months between your last working day and the severance payment. Moving the payment into the next calendar year (e.g. January) can lower the tax, because that year has less other income. Which month is January follows from the current month.' },
    { g: 'Severance', k: 'fuenftel', label: 'Use Fünftelregelung (§34 EStG)', def: true,
      help: 'Taxes the severance as if it were spread over five years, which softens the progressive tax rate. Since 2025 employers no longer apply it in payroll; you claim it in your tax return.' },
    { g: 'After leaving', k: 'searchMonths', label: 'Months until new job starts', def: 6, step: 1, unit: 'mo',
      help: 'Months between leaving and starting your new job, during which you may receive ALG I. Rounded to whole months. Enter the most likely value. The Monte Carlo simulation varies it within the range under Uncertainty.' },
    { g: 'After leaving', k: 'algMonths', label: 'Max. months of unemployment benefit (ALG I)', def: 12, step: 1, unit: 'mo',
      help: 'How long you are entitled to Arbeitslosengeld I. It depends on age and contribution history: usually 12 months under 50, up to 24 months from 58. 0 = no entitlement.' },
    { g: 'After leaving', k: 'algRate', label: 'ALG I rate', def: 60, step: 1, unit: '%', pct: 1,
      help: 'ALG I as a share of your previous net salary: 60 %, or 67 % if you or your partner have a child.' },
    { g: 'After leaving', k: 'newSalary', label: 'New job gross salary p.a.', def: 80000, step: 1000, unit: '€',
      help: 'Annual gross salary in your new job. Enter the most likely value. The Monte Carlo simulation varies it within the range under Uncertainty. 0 = you do not work again within the horizon.' },
    { g: 'After leaving', k: 'newGrowth', label: 'New job salary growth p.a.', def: 2, step: 0.1, unit: '%', pct: 1,
      help: 'Average raise you expect each January in the new job. Enter the most likely value. The Monte Carlo simulation varies it within the range under Uncertainty.' },
    { g: 'Market & life', k: 'horizon', label: 'Horizon (years)', def: 10, step: 1, unit: 'y',
      help: 'How many years to compare, counted from today. All results are your wealth at the end of this period.' },
    { g: 'Market & life', k: 'spending', label: 'Net spending p.a. (today’s money)', def: 45000, step: 1000, unit: '€',
      help: 'What you spend per year after tax, in today’s money; it rises with inflation. It is the same in both scenarios: any surplus is invested, any shortfall is taken from your investments.' },
    { g: 'Market & life', k: 'inflation', label: 'Inflation p.a.', def: 2, step: 0.1, unit: '%', pct: 1,
      help: 'Expected average inflation. It raises your spending and converts results into today’s money. The Monte Carlo simulation lets it fluctuate around this value (Inflation volatility under Uncertainty).' },
    { g: 'Market & life', k: 'startWealth', label: 'Existing investments (both scenarios)', def: 0, step: 5000, unit: '€',
      help: 'Investments you already hold today, in both scenarios. They cushion months without income.' },
    { g: 'Market & life', k: 'ret', label: 'Gross return p.a.', def: 6, step: 0.1, unit: '%', pct: 1,
      help: 'Expected average yearly return of your investments, before costs and taxes. The Monte Carlo simulation lets it fluctuate around this value (Return volatility under Uncertainty).' },
    { g: 'Market & life', k: 'ter', label: 'Costs (TER) p.a.', def: 0.2, step: 0.05, unit: '%', pct: 1,
      help: 'Yearly fund costs (total expense ratio), deducted from the return.' },
    { g: 'Market & life', k: 'debtRate', label: 'Interest if wealth goes negative', def: 6, step: 0.1, unit: '%', pct: 1,
      help: 'If spending uses up all your investments, the shortfall is treated as a loan at this interest rate.' },
    { g: 'Pension', k: 'pensionInWealth', label: 'Count pension rights in final wealth', def: true,
      help: 'Both scenarios earn statutory pension points (Entgeltpunkte) within the horizon: from salary up to the contribution ceiling, and from ALG I on 80 % of your old salary; search months without ALG I and the severance earn none. On: the value of these points at the end of the horizon (see the inputs below) is added to the final wealth of each scenario, so the verdict, simulation and break-evens include it. Off: final wealth is your investments only; the pension difference is still shown. Points earned after the horizon are not counted in either scenario.' },
    { g: 'Pension', k: 'age', label: 'Your age today', def: 45, step: 1, unit: 'y',
      help: 'Sets how many years lie between the end of the horizon and your retirement, and the year your pension starts, which fixes its taxable share.' },
    { g: 'Pension', k: 'retirementAge', label: 'Retirement age', def: 67, step: 1, unit: 'y',
      help: 'Age at which your statutory pension starts: 67 for everyone born from 1964, without deductions. If the horizon ends later, the value counts payments from the end of the horizon only.' },
    { g: 'Pension', k: 'lifeExpectancy', label: 'Life expectancy at retirement', def: 20, step: 0.5, unit: 'y',
      help: 'Remaining years of life expected at the retirement age. Destatis projections for people retiring around 2045–2055 give very roughly 18–19 years for men and 21–22 for women. The value uses a Gompertz mortality curve (mortality doubling every 7 years of age) fitted to this number, so it also counts the chance of dying before retirement.' },
    { g: 'Pension', k: 'pensionGrowth', label: 'Wage and pension growth p.a.', def: 2.5, step: 0.1, unit: '%', pct: 1,
      help: 'Yearly growth of average earnings in Germany. Pensions and the contribution ceiling follow it. A salary that grows faster than this earns more points each year, one that grows slower earns fewer. Also raises the value of each point until and during retirement.' },
    { g: 'Pension', k: 'pensionDiscount', label: 'Discount rate for pensions p.a.', def: 3, step: 0.1, unit: '%', pct: 1,
      help: 'Rate at which future pension payments are converted into an amount at the end of the horizon. A pension is a safe, wage-indexed income, so use a safe rate (long-term government bonds), not the expected return of your investments, which would make it look less valuable than it is.' },
    { g: 'Pension', k: 'pensionTax', label: 'Tax rate in retirement', def: 25, step: 1, unit: '%', pct: 1,
      help: 'Your marginal income tax rate in retirement, incl. Soli and church tax. It applies to the taxable share of the pension: 84 % for pensions starting in 2026, rising by 0.5 points a year to 100 % from 2058 (§22 EStG). Health and care contributions of retirees (7.3 % + half the Zusatzbeitrag, plus the full care rate) are deducted first; they come from the inputs under Social security.' },
    { g: 'Taxes', k: 'joint', label: 'Joint assessment (Splitting)', def: false,
      help: 'Zusammenveranlagung for married or registered partners: income tax uses the splitting tariff with your partner’s income below. Also doubles the Sparer-Pauschbetrag and the Soli threshold.' },
    { g: 'Taxes', k: 'partnerZve', label: 'Partner taxable income p.a. (joint only)', def: 0, step: 1000, unit: '€',
      help: 'Your partner’s taxable income (zu versteuerndes Einkommen) per year, kept constant. Only used with joint assessment.' },
    { g: 'Taxes', k: 'church', label: 'Church tax rate (0 / 8 / 9)', def: 0, step: 1, unit: '%', pct: 1,
      help: 'Church tax as a share of income tax: 8 % in Bavaria and Baden-Württemberg, 9 % in the other states, 0 if you are not a church member.' },
    { g: 'Taxes', k: 'extraDeduct', label: 'Extra deductions p.a. (Werbungskosten etc.)', def: 0, step: 100, unit: '€',
      help: 'Deductions per year on top of the Werbungskosten-Pauschale and social security, e.g. further work expenses or Sonderausgaben.' },
    { g: 'Taxes', k: 'tf', label: 'Teilfreistellung on fund gains', def: 30, step: 1, unit: '%', pct: 1,
      help: 'Tax-free share of fund gains: 30 % for equity funds, 15 % for mixed funds, 0 % for bond funds and interest.' },
    { g: 'Taxes', k: 'deferred', label: 'Accumulating fund (tax at sale, ignores Vorabpauschale)', def: true,
      help: 'On: gains are taxed only when you sell (the yearly Vorabpauschale is ignored). Off: gains are taxed every year, as with interest.' },
    { g: 'Social security', k: 'zusatz', label: 'KV Zusatzbeitrag (total)', def: 2.9, step: 0.1, unit: '%', pct: 1,
      help: 'Your health insurer’s full Zusatzbeitrag (average 2.9 % in 2026). Enter the total rate; you pay half of it.' },
    { g: 'Social security', k: 'pvRate', label: 'Pflegeversicherung (employee share)', def: 2.4, step: 0.1, unit: '%', pct: 1,
      help: 'Your share of long-term care insurance: 1.8 %, plus 0.6 % if you are childless, less with two or more children under 25.' },
    { g: 'Uncertainty', k: 'searchMin', of: 'searchMonths', label: 'Months until new job: at best', def: 2, step: 1, unit: 'mo',
      help: 'Best case for the months until your new job starts. Each simulated future draws a value from a PERT distribution: a smooth curve from worst to best case that peaks at your main input, rounded to whole months.' },
    { g: 'Uncertainty', k: 'searchMax', of: 'searchMonths', label: 'Months until new job: at worst', def: 18, step: 1, unit: 'mo',
      help: 'Worst case for the months until your new job starts. Each simulated future draws a value from a PERT distribution: a smooth curve from worst to best case that peaks at your main input, rounded to whole months.' },
    { g: 'Uncertainty', k: 'newSalaryMin', of: 'newSalary', label: 'New job salary: at worst', def: 65000, step: 1000, unit: '€',
      help: 'Lowest new salary you consider realistic. Each simulated future draws a salary from a PERT distribution: a smooth curve from worst to best case that peaks at your main input.' },
    { g: 'Uncertainty', k: 'newSalaryMax', of: 'newSalary', label: 'New job salary: at best', def: 90000, step: 1000, unit: '€',
      help: 'Highest new salary you consider realistic. Each simulated future draws a salary from a PERT distribution: a smooth curve from worst to best case that peaks at your main input.' },
    { g: 'Uncertainty', k: 'stayGrowthMin', of: 'stayGrowth', label: 'Salary growth if you stay: lowest', def: 0, step: 0.1, unit: '%', pct: 1,
      help: 'Lowest yearly raise you consider realistic if you stay. Each simulated future draws one growth rate from a PERT distribution: a smooth curve from worst to best case that peaks at your main input, and keeps it for the whole horizon.' },
    { g: 'Uncertainty', k: 'stayGrowthMax', of: 'stayGrowth', label: 'Salary growth if you stay: highest', def: 3, step: 0.1, unit: '%', pct: 1,
      help: 'Highest yearly raise you consider realistic if you stay. Each simulated future draws one growth rate from a PERT distribution: a smooth curve from worst to best case that peaks at your main input, and keeps it for the whole horizon.' },
    { g: 'Uncertainty', k: 'newGrowthMin', of: 'newGrowth', label: 'New job salary growth: lowest', def: 0, step: 0.1, unit: '%', pct: 1,
      help: 'Lowest yearly raise you consider realistic in the new job. Each simulated future draws one growth rate from a PERT distribution: a smooth curve from worst to best case that peaks at your main input, and keeps it for the whole horizon.' },
    { g: 'Uncertainty', k: 'newGrowthMax', of: 'newGrowth', label: 'New job salary growth: highest', def: 4, step: 0.1, unit: '%', pct: 1,
      help: 'Highest yearly raise you consider realistic in the new job. Each simulated future draws one growth rate from a PERT distribution: a smooth curve from worst to best case that peaks at your main input, and keeps it for the whole horizon.' },
    { g: 'Uncertainty', k: 'returnVol', of: 'ret', label: 'Return volatility p.a.', def: 16, step: 1, unit: '%', pct: 1,
      help: 'How much yearly returns swing around the expected return (standard deviation). Each simulated year draws its return from a lognormal distribution with your expected return as mean, independent of other years. About 15–18 % for a global equity fund; 0 = the expected return every year. Both scenarios see the same market.' },
    { g: 'Uncertainty', k: 'inflationVol', of: 'inflation', label: 'Inflation volatility p.a.', def: 1, step: 0.1, unit: '%', pct: 1,
      help: 'How much yearly inflation swings around the expected inflation (long-run standard deviation). Inflation is persistent: each year keeps 70 % of last year’s deviation from the expected value, plus a random shock. 0 = the expected inflation every year.' },
    { g: 'Constants 2026', k: 'bbgRV', label: 'Beitragsbemessungsgrenze RV/AV', def: 101400, step: 100, unit: '€',
      help: 'Income ceiling for pension and unemployment insurance contributions; salary above it is contribution-free.' },
    { g: 'Constants 2026', k: 'bbgKV', label: 'Beitragsbemessungsgrenze KV/PV', def: 69750, step: 100, unit: '€',
      help: 'Income ceiling for health and long-term care insurance contributions; salary above it is contribution-free.' },
    { g: 'Constants 2026', k: 'rentenwert', label: 'Rentenwert (per point and month)', def: 42.52, step: 0.01, unit: '€',
      help: 'Monthly gross pension per Entgeltpunkt, since 1 July 2026. Grows with the wage and pension growth above.' },
    { g: 'Constants 2026', k: 'avgEarnings', label: 'Durchschnittsentgelt (provisional)', def: 51944, step: 1, unit: '€',
      help: 'Average gross earnings of all insured in 2026. A year at this salary earns one Entgeltpunkt; it grows with the wage and pension growth above.' },
    { g: 'Constants 2026', k: 'wk', label: 'Werbungskosten-Pauschale', def: 1230, step: 10, unit: '€',
      help: 'Flat deduction for work expenses, subtracted from wages before income tax.' },
    { g: 'Constants 2026', k: 'sparer', label: 'Sparer-Pauschbetrag (single)', def: 1000, step: 10, unit: '€',
      help: 'Capital gains up to this amount per year are tax-free; doubled with joint assessment.' },
    { g: 'Constants 2026', k: 'soliFree', label: 'Soli Freigrenze (single)', def: 20350, step: 50, unit: '€',
      help: 'No solidarity surcharge is due while income tax stays below this amount; doubled with joint assessment.' },
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

  // Main inputs the Monte Carlo varies: those that an Uncertainty input points to.
  function varied() {
    return [...new Set(SPEC.filter(s => s.of).map(s => s.of))];
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

  // How the last "Wealth (end)" of the year-by-year table becomes the final wealth figures:
  // selling everything at the end taxes the unrealized gains, the pension rights are added
  // (if counted, else 0), then inflation is taken out.
  function finalWealth(sim) {
    const priceLevel = sim.priceLevels[sim.priceLevels.length - 1];
    const one = (rows, liquid, nominal, real) => {
      const wealthEnd = rows[rows.length - 1].wealth;
      return { wealthEnd, saleTax: wealthEnd - liquid, pension: nominal - liquid, nominal, priceLevel, real };
    };
    return { a: one(sim.a, sim.liquidA, sim.finalA, sim.realA), b: one(sim.b, sim.liquidB, sim.finalB, sim.realB) };
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

  const api = { SPEC, defaults, normalize, varied, uncertainty, finalWealth, loadCore };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.Model = api;
})(typeof window !== 'undefined' ? window : globalThis);
