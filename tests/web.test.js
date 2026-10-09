// End-to-end check of the built web bundle: core.js + core.wasm.js + params.js, as the browser loads them.
// Run with `just test-web` (after `just build`).
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.join(__dirname, '..');
const M = require(path.join(root, 'params.js'));

// Classic browser scripts: evaluate them in the global scope like <script> tags do.
function loadScript(file) {
  vm.runInThisContext(fs.readFileSync(path.join(root, file), 'utf8'), { filename: file });
}
globalThis.window = globalThis;
loadScript('core.js');
loadScript('core.wasm.js');
const core = M.loadCore(vm.runInThisContext('wasm_bindgen'), window.CORE_WASM_B64);

test('analyze returns the deterministic simulation', async () => {
  const res = (await core).analyze(M.normalize(M.defaults()));
  assert.strictEqual(typeof res.sim.realDiff, 'number');
  assert.strictEqual(res.sim.a.length, 10);
});

test('invalid input throws a JS error', async () => {
  const c = await core;
  assert.throws(() => c.analyze({}), /missing field/);
});

test('monteCarlo summarizes the requested number of runs reproducibly', async () => {
  const c = await core;
  const P = M.normalize(M.defaults());
  const r = c.monteCarlo(P, 500, 1);
  assert.strictEqual(r.runs, 500);
  assert.ok(r.realDiff.probPositive >= 0 && r.realDiff.probPositive <= 1);
  assert.ok(r.realDiff.percentiles.p5 < r.realDiff.percentiles.p95);
  assert.deepStrictEqual(c.monteCarlo(P, 500, 1), r);
});

test('monteCarlo returns wealth bands and sample paths per year', async () => {
  const c = await core;
  const r = c.monteCarlo(M.normalize(M.defaults()), 200, 1);
  for (const w of [r.wealthA, r.wealthB]) {
    for (const s of [w.nominal, w.real]) {
      assert.strictEqual(s.percentiles.length, 11);
      assert.ok(s.percentiles[10].p5 < s.percentiles[10].p95);
      assert.strictEqual(s.samples.length, 40);
      assert.strictEqual(s.samples[0].length, 11);
    }
  }
});

test('uncertainty ranges widen to include the most likely value', () => {
  const P = M.normalize(M.defaults());
  assert.deepStrictEqual(M.uncertainty(P).newSalary, { min: 65000, mode: 80000, max: 90000 });
  const high = M.uncertainty(Object.assign({}, P, { newSalary: 100000 })).newSalary;
  assert.deepStrictEqual(high, { min: 65000, mode: 100000, max: 100000 });
});

test('every input has help text', () => {
  const missing = M.SPEC.filter(s => typeof s.help !== 'string' || s.help.length < 20).map(s => s.k);
  assert.deepStrictEqual(missing, []);
});

test('uncertainty inputs point to the main input they vary', () => {
  const byKey = Object.fromEntries(M.SPEC.map(s => [s.k, s]));
  const unc = M.SPEC.filter(s => s.g === 'Uncertainty');
  for (const s of unc) {
    assert.ok(byKey[s.of], `${s.k}.of = ${s.of}`);
    assert.notStrictEqual(byKey[s.of].g, 'Uncertainty', `${s.k}.of`);
  }
  assert.ok(M.SPEC.filter(s => s.g !== 'Uncertainty').every(s => s.of === undefined));
});

test('the main inputs varied by the Monte Carlo are exactly those with uncertainty inputs', () => {
  const varied = [...new Set(M.SPEC.filter(s => s.of).map(s => s.of))].sort();
  assert.deepStrictEqual(M.varied().sort(), varied);
  assert.deepStrictEqual(varied, ['inflation', 'newGrowth', 'newSalary', 'ret', 'searchMonths', 'stayGrowth']);
});

test('final wealth breakdown leads from the table to the final wealth figures', async () => {
  const c = await core;
  for (const deferred of [true, false]) {
    const P = Object.assign(M.normalize(M.defaults()), { horizon: 14, deferred });
    const r = c.analyze(P).sim;
    for (const [key, rows, final, real] of [['a', r.a, r.finalA, r.realA], ['b', r.b, r.finalB, r.realB]]) {
      const f = M.finalWealth(r)[key];
      assert.strictEqual(f.wealthEnd, rows[rows.length - 1].wealth, key);
      assert.ok(Math.abs(f.wealthEnd - f.saleTax - final) < 1e-6, key);
      assert.strictEqual(f.nominal, final);
      assert.strictEqual(f.priceLevel, r.priceLevels[r.priceLevels.length - 1]);
      assert.ok(Math.abs(f.nominal / f.priceLevel - real) < 1e-6, key);
      assert.strictEqual(f.real, real);
    }
  }
  const P = M.normalize(M.defaults());
  assert.ok(M.finalWealth(c.analyze(P).sim).a.saleTax > 0, 'accumulating fund: gains taxed at sale');
});
