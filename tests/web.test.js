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
