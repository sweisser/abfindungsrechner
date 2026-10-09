# Actuarial review of the severance calculator

Scope: the Rust core (`tax.rs`, `income.rs`, `portfolio.rs`, `simulation.rs`, `analysis.rs`, `monte_carlo/`) and the parameter help texts in `params.js`.

## Overall verdict

The engineering is good. The tax and payroll mechanics are careful for a tool of this size. The decision model around them has gaps that can change the answer. Rough grade: **implementation B+, methodology C**. The tool computes "liquid wealth after N years" precisely, but liquid wealth after N years is the wrong measure for this decision.

## Fatal (each one can flip the recommendation)

### 1. Value beyond the horizon and pension rights are left out

The result is liquid wealth at year N. Two large items are missing:

- **The salary gap after the horizon.** If the new salary is lower than the old one, the gap continues after year 10, and the model treats it as zero. Because of that, the answer depends heavily on the horizon you pick. An actuary would value both income streams up to retirement, or add a terminal value of human capital at the horizon.
- **Statutory pension (GRV) and company pension (bAV).** Social security is counted as a pure cost (`core/src/income.rs:15`). In scenario A, 9.3 % of salary buys Entgeltpunkte that are worth more than they cost, and the model credits nothing for them. That biases the result towards B. Search months without ALG I and a lower new salary mean fewer Entgeltpunkte, and ALG I itself is credited on 80 % of the previous gross. Lost or unvested bAV is often the largest single item in a severance negotiation, and there is no input for it.

### 2. "Stay" is modelled as risk-free

Scenario A assumes guaranteed employment with steady raises. In practice severance is usually offered during restructuring, and the realistic alternative includes a chance of being laid off later with a smaller package or none. Without a yearly layoff probability in A, the comparison is biased towards staying. This goes in the opposite direction from point 1, and the two do not cancel out reliably.

### 3. Simulation years are not calendar years

Year k covers months 12(k−1) to 12k counted from today, and each of those blocks is taxed as one tax year (`core/src/simulation.rs:76`). That is only correct if today is 1 January. The help text for `payDelay` tells users that moving the payment into January lowers the tax. On 9 October, "payment in January" means month 3, which the model taxes together with the full year of salary. The timing lever the tool advertises is computed wrongly. It needs a start-month input.

### 4. The standard ALG I traps are not modelled

- **Sperrzeit (§159 SGB III).** A termination agreement normally triggers 12 weeks without ALG I, and the entitlement is cut by at least a quarter. The model starts ALG I in the exit month.
- **Ruhen (§158 SGB III).** If the notice period is shortened, part of the severance is offset against ALG I.
- **Health insurance gap.** During search months without ALG I, the person has to pay for voluntary health insurance (roughly €200–1,000 per month). This costs nothing in the model.

These are the main reasons people lose money on a severance package. Most users won't know to adjust `algMonths` for them.

### 5. Fünftelregelung eligibility is not checked

§34 EStG requires a "Zusammenballung": the severance plus that year's other income has to exceed what the person would have earned by staying. The code applies the rule whenever the checkbox is on (`core/src/income.rs:39`). With a mid-year exit and a small severance it isn't allowed, and the tool still gives the tax advantage.

## Significant

- **The Monte Carlo ignores correlation.** Search duration and new salary are drawn independently of the market. In reality equity crashes and long job searches happen together, so B's downside is understated. Return and inflation are also independent of each other.
- **The PERT distribution has a bounded worst case.** The real risk in B is the long tail: ALG I runs out, then Bürgergeld with an asset test. A bounded worst-case search duration cuts that tail off. A heavier-tailed distribution, or an explicit "no job within X months" probability, would be better.
- **Tax and contribution parameters are frozen at 2026.** Salaries grow but the tariff and the contribution ceilings (BBG) do not, so bracket creep over 10 years overstates taxes. It affects both scenarios, but not equally when the salaries differ. The usual fix is to index them with wage growth.
- **The Vorabpauschale is ignored** in deferred mode. With today's Basiszins this is not negligible over 10 years.
- **Pension contributions are deducted too generously.** All employee contributions are deducted from taxable income, but unemployment insurance (AV) is in practice not deductible because the §10 cap is already used up by health and care insurance. Small effect.
- **There is no independent validation.** `default_case_matches_reference` only checks against the earlier JS version, which is a regression test, not a validation. The Fünftel and Progressionsvorbehalt cases should be checked against the BMF calculator (or another external calculator).

## What is good

- **Same market for A and B** in every Monte Carlo run (common random numbers). This is the right design: it removes market noise from the difference and makes `probPositive` meaningful.
- **Holding spending equal in both scenarios** isolates the decision cleanly and avoids having to choose a utility function.
- **Household tax handled carefully:** the user's tax is the marginal tax against the partner alone; Progressionsvorbehalt for ALG I; Fünftel computed on the base including ALG I; Soli exemption threshold and phase-in zone; church tax deductible within the flat capital-gains tax rate. The 2026 tariff coefficients and the 2026 contribution ceilings and rates look correct.
- **Social security is capped month by month** against the ceilings, so part-year income is handled correctly.
- **Portfolio tax:** separate cost basis, Teilfreistellung, Sparer-Pauschbetrag, gross-up when selling, and liquidation at the horizon so both scenarios are compared after tax. Mid-year timing of cash flows and an exact payment month for the severance.
- **Lognormal returns with the correct mean, and AR(1) inflation.** These are standard and adequate choices, and they are documented in the tooltips.
- **Break-even values and the sensitivity grid** answer the questions users actually ask. The code is clean, tested and deterministic for a given seed.

## Improvements, in priority order

1. Add a **start month** so simulation years line up with calendar years (cheap, and fixes fatal issue 3).
2. Add **Sperrzeit / Ruhen** toggles, a **monthly health insurance cost** during uncovered months, and a **Zusammenballung check** (fatal issues 4 and 5).
3. Add a **layoff hazard in A**, for example p % per year, then search months plus an optional smaller severance (fatal issue 2).
4. Add **pension points** and a **bAV input**, and either set the horizon to retirement or add a **terminal value** of the remaining salary difference (fatal issue 1).
5. **Correlate** search months and new salary with a bad market year, and make the search duration **heavy-tailed**.
6. **Index** the tariff and ceilings over time; add the **Vorabpauschale**.
7. **Validate the tax** against an independent calculator for several reference cases and pin those results in tests.

Items 1–2 are small changes. Items 3–4 change what the tool actually measures and would make the recommendation trustworthy.
