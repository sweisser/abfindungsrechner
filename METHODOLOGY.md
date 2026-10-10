# How the calculator works

This page explains how the severance calculator gets its results: what it compares, which rules it
applies, what it assumes and what it leaves out. It is meant to help you judge how far to trust a
result. It does not replace the tooltips in the page, which describe each input in detail.

> **Disclaimer:** The calculator is a simplified model. We cannot guarantee that the calculations,
> tax rules or results are correct, complete or up to date. This is not tax, legal or financial advice.

## In short

The calculator follows your finances month by month in two scenarios over a horizon you choose
(default 10 years):

- **A – Stay:** you keep your job and your salary rises every January.
- **B – Take the severance:** you keep your salary until you leave, receive the severance, may draw
  unemployment benefit (ALG I) while you search, and then start a new job.

In both scenarios you spend **the same amount** each year. Whatever is left after tax and social
security is invested, and any shortfall is taken from your investments. At the end of the horizon
everything is sold, the tax on the gains is paid, and the result is converted into today's money.
That gives the **final wealth** of each scenario. Optionally, the value of the statutory pension
rights earned within the horizon is added. **The difference B − A answers the question:**
*"Am I better or worse off by taking the severance?"*

The calculation runs in two ways:

- **Expected:** one run with your main inputs, taken as the most likely values.
- **Simulated (Monte Carlo):** 2,000 runs. Each run draws the uncertain inputs (job search, new
  salary, raises, market returns, inflation) at random. This gives a range of outcomes and the
  chance that the severance wins.

## 1. Timeline

- The simulation starts **today** (the *current month* input) and runs for the horizon in whole years.
- It is split into **calendar years**, because income tax is assessed per calendar year. The first
  and last year are partial unless today is in January. Each row of the year-by-year table is one
  calendar year.
- The months of the current year **before today** are counted at your current salary in both
  scenarios. They count towards this year's income tax (which matters for the tax on a severance
  paid this year), but the money from them is treated as already spent.
- Salaries rise **every January** by the growth rate of the respective job.
- In scenario B:
  1. You keep your current salary until the exit month.
  2. The severance is paid *pay delay* months after leaving. It is taxed in the calendar year of
     the payment, so moving it into January of the next year can lower the tax.
  3. You then search for a job for *search months*. During the first of them (up to your ALG I
     entitlement) you receive ALG I; the rest have no income.
  4. The new job starts after the search, at the new salary.

## 2. Income, social security and tax

Each calendar year is taxed as a whole, as in the annual tax assessment. Monthly payroll
withholding is not modelled.

**Social security** (employee share): pension 9.3 %, unemployment 1.3 %, health 7.3 % plus half the
Zusatzbeitrag, and your care insurance rate. Each is capped month by month at the 2026 contribution
ceilings (Beitragsbemessungsgrenzen). The severance is free of social security contributions.

**Taxable income** = wages − Werbungskosten-Pauschale − social security contributions −
Sonderausgaben-Pauschbetrag − your extra deductions. With **joint assessment**, your partner's
taxable income is added and the splitting tariff applies. Only the *extra* tax caused by your own
income is counted as yours, i.e. the household tax minus the tax your partner would pay alone.

**Income tax** uses the 2026 tariff (§32a EStG), plus:

- **Solidarity surcharge** with its exemption threshold and phase-in zone, and **church tax** if you
  enter a rate.
- **Progressionsvorbehalt:** ALG I itself is tax-free, but it raises the tax rate on your other income.
- **Fünftelregelung (§34 EStG)**, if switched on: the severance is taxed as if spread over five years.
  The extra tax caused by the severance is shown separately.

**ALG I** per month = ALG I rate (60 % or 67 %) × your previous net monthly salary, with the salary
capped at the pension contribution ceiling. The net salary is computed with the same tax and social
security rules as above. This is an approximation of the official calculation.

## 3. Investments

Each scenario has one investment portfolio. It starts with your existing investments (the same in
both scenarios).

- **Return:** the gross return minus the fund costs (TER). Savings and withdrawals during the year
  are assumed to happen in the middle of the year on average. The severance is invested from the
  month it is paid.
- **Tax on gains** uses the flat capital-gains tax (25 % plus Soli and church tax). Only the
  non-exempt part of fund gains is taxed (Teilfreistellung, e.g. 30 % tax-free for equity funds), and
  the yearly Sparer-Pauschbetrag is tax-free.
  - **Accumulating fund** (default): gains are taxed only when you sell. The yearly Vorabpauschale
    is ignored.
  - Otherwise gains are taxed every year, as with interest.
- **Withdrawals:** if your net income does not cover your spending, fund units are sold. The
  calculator sells enough that, after the tax on the gains contained in them, the shortfall is covered.
- **Negative wealth:** if the portfolio runs out, the remaining shortfall is treated as a loan at the
  interest rate you enter.
- **End of the horizon:** everything is sold and the tax on the remaining gains is paid. Final wealth
  is always shown **after** this tax, so both scenarios are compared on the same footing.

## 4. Statutory pension

Both scenarios earn **Entgeltpunkte** (pension points) in the statutory pension insurance during
the horizon:

- **Salary:** points per year = gross salary (up to the pension contribution ceiling) ÷ average
  earnings (Durchschnittsentgelt). Average earnings, the ceiling and the Rentenwert all grow with
  the *wage and pension growth* input.
- **ALG I:** the employment agency pays pension contributions on 80 % of your previous gross salary,
  so ALG I months earn points too.
- **Search months without ALG I** and the **severance** earn no points.
- Points earned **after** the horizon are not counted in either scenario.

**Value of a point** at the end of the horizon: the pension it pays every month from the retirement
age for life, minus retirees' health and care contributions and income tax on the taxable share of
the pension (fixed by the year the pension starts, §22 EStG). Each payment is weighted by the chance
of being alive then and discounted to the end of the horizon at the *discount rate for pensions*.
The chance of being alive comes from a Gompertz mortality curve (mortality doubles about every 7
years of age) fitted to your life expectancy at retirement.

If **"Count pension rights in final wealth"** is on, the value of the points earned in each scenario
is added to its final wealth. If it is off, the pension difference is only shown separately.

## 5. Results

- **Final wealth** of A and B, nominal and in **today's money** (divided by the price level at the
  end of the horizon). The verdict and the main figures use today's money.
- **Year-by-year table:** income, social security, tax, spending and wealth per calendar year.
- **Break-evens:** the new salary, and the severance, at which B exactly catches up with A, all else
  being equal. They are found by searching for the value where the difference B − A is zero.
- **Sensitivity grid:** the difference B − A for new salaries of 60 %–120 % of your input and job
  searches of 0–24 months.

## 6. Monte Carlo simulation

The expected run treats every input as certain. The simulation shows how much the result depends
on luck. It runs **2,000 simulated futures** with a fixed random seed, so the same inputs always
give the same results.

In each simulated future:

- **Personal uncertainty:** the months until the new job, the new salary and the salary growth in
  each job are drawn once from a **PERT distribution**: a smooth curve between the best and worst
  case you enter under *Uncertainty*, peaking at your main input. Search months are rounded to
  whole months, and the drawn growth rates apply to the whole horizon.
- **Market returns:** each calendar year's return is drawn independently from a **lognormal
  distribution** whose mean is your expected return and whose standard deviation is the *return
  volatility*. A partial year gets a proportionally smaller variance.
- **Inflation** fluctuates around your expected inflation and is **persistent**: each year keeps
  70 % of the previous year's deviation and adds a random shock (an AR(1) process). The first year
  uses the expected inflation.
- **Both scenarios see the same market.** Within one simulated future, A and B get the same returns
  and the same inflation. The difference between them therefore reflects your decision, not market
  luck.

The simulated results show:

- the **chance that the severance wins** (the share of futures in which B ends with more final
  wealth than A);
- the **median and percentile ranges** (5 %, 25 %, 75 %, 95 %) of final wealth and of the difference;
- **wealth over time** as percentile bands, plus a few single simulated futures. These charts show
  wealth before the final sale tax.

The pension inputs (life expectancy, discount rate, pension growth) are not varied. The number of
pension points does vary, because it depends on the salaries and search months drawn.

## 7. Checking the figures yourself: the Excel download

The **Excel** button above the year-by-year table downloads a workbook that recomputes the expected
run of both scenarios with live formulas. Click a cell to see how it is computed; each row explains
its method in the column on the right.

- **Inputs:** every input of the calculator as a named cell (e.g. `salary`), plus the constants of
  the code (tax tariff 2026, contribution rates, …). Edit them and the workbook recomputes. The
  horizon and the current month are fixed, because they set the year columns; the level of the
  mortality curve is fixed, because it is fitted iteratively (the Pension sheet flags it when it no
  longer matches). Export again from the calculator to change them.
- **Derived, Calendar, A Stay, B Severance, Pension:** the calculation step by step, one column per
  calendar year, in the same order as the calculator's code: months, salaries, social security,
  income tax (with Progressionsvorbehalt, Fünftelregelung, Soli, church tax and splitting), cash,
  investments with their taxation, pension points and the value of a point.
- **Result:** the final wealth of both scenarios and B − A, each next to the value the calculator
  showed, with the difference.
- **Analysis:** break-evens, the sensitivity grid and the Monte Carlo results, as plain values: they
  need many runs of the whole calculation, which spreadsheet formulas cannot reproduce.

The calculator's tests recompute the workbook's formulas for many input combinations and check that
every figure equals the calculator's own result.

## 8. What the calculator does not model

These simplifications can matter for your decision. Adjust the inputs by hand where you can, and
check the important points with an advisor.

**Unemployment benefit**

- **Sperrzeit (§159 SGB III):** a termination agreement usually triggers a 12-week block on ALG I
  and shortens the entitlement by at least a quarter. The calculator starts ALG I right after you
  leave. If this applies to you, reduce *Max. months of ALG I* accordingly.
- **Ruhen (§158 SGB III):** if the notice period is shortened, part of the severance is set off
  against ALG I. Not modelled.
- **Health insurance without ALG I:** during search months without ALG I you have to pay for health
  insurance yourself. This cost is not included.

**Tax**

- The **Fünftelregelung** is applied whenever it is switched on. Whether you actually qualify (the
  severance and that year's other income must exceed what you would have earned by staying) is not
  checked.
- Tax brackets, allowances and contribution ceilings are **frozen at 2026** values, while salaries
  grow. This overstates taxes in later years in both scenarios.
- All social security contributions are deducted in full from taxable income; in reality the
  deduction for some of them is capped.
- The **Vorabpauschale** on accumulating funds is ignored.
- Your partner's taxable income stays constant over the horizon.

**The decision itself**

- **Staying is treated as safe.** Scenario A assumes you keep your job with steady raises. In reality
  a severance offer often comes during restructuring, and staying may carry the risk of a later
  layoff with a smaller package or none.
- **Nothing beyond the horizon is counted**, apart from the pension rights earned within it. If the
  new salary is lower than the old one, the gap continues after the horizon but is not included.
  Results can therefore depend strongly on the horizon you choose.
- **Company pensions (bAV)** and other employer benefits you may lose are not modelled.
- **Spending is the same in both scenarios** and does not react to good or bad outcomes.

**Simulation**

- Job search, new salary and raises are drawn **independently of the market**. In reality long job
  searches and market crashes tend to happen together, so the downside of B is probably understated.
- Returns and inflation are independent of each other.
- The PERT distribution never goes beyond the worst case you enter. Very long periods without a job
  (ALG I running out, then Bürgergeld) are only included if your worst case covers them.
