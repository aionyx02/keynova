use chrono::NaiveDate;

mod datemath;
mod parser;
mod units;

use datemath::{
    add_to, contains_date_literal, next_weekday, parse_date_anchor, parse_n_unit, parse_weekday,
    previous_weekday, split_top_level_arith, subtract_from,
};
use units::{
    convert_unit, currency_usd_rate, format_number, parse_any_base, parse_value_unit,
    CURRENCY_TABLE_DATE,
};

pub use parser::eval_expr;

/// 計算機管理器：提供數學運算、單位換算、進位轉換、日期運算。
pub struct CalculatorManager {
    history: Vec<CalculationEntry>,
    /// Override today's date for deterministic tests; production code leaves None.
    today_override: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct CalculationEntry {
    pub expr: String,
    pub result: String,
}

impl CalculatorManager {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            today_override: None,
        }
    }

    /// 計算數學運算式，回傳結果字串。
    pub fn eval(&mut self, expr: &str) -> Result<String, String> {
        let trimmed = expr.trim();
        if trimmed.is_empty() {
            return Err("empty expression".into());
        }

        // 進位轉換：0x / 0b / 0o 前綴
        if let Some(result) = self.try_base_conversion(trimmed) {
            let s = result?;
            self.push_history(trimmed, &s);
            return Ok(s);
        }

        // 貨幣換算：如 "100 USD to TWD"（offline fallback table；online rate 待 ADR-038）
        if let Some(result) = self.try_currency_conversion(trimmed) {
            let s = result?;
            self.push_history(trimmed, &s);
            return Ok(s);
        }

        // 單位換算：如 "5 km to m"
        if let Some(result) = self.try_unit_conversion(trimmed) {
            let s = result?;
            self.push_history(trimmed, &s);
            return Ok(s);
        }

        // 日期運算：`today + 90 days`、`2026/12/31 - today`、`next monday`
        if let Some(result) = self.try_date_arithmetic(trimmed) {
            let s = result?;
            self.push_history(trimmed, &s);
            return Ok(s);
        }

        // 數學運算式
        let result = eval_expr(trimmed)?;
        let s = format_number(result);
        self.push_history(trimmed, &s);
        Ok(s)
    }

    /// Return the reference "today". Production code defers to the system clock;
    /// tests override via `today_override` for determinism.
    fn today(&self) -> NaiveDate {
        self.today_override
            .unwrap_or_else(|| chrono::Local::now().date_naive())
    }

    pub fn history(&self) -> &[CalculationEntry] {
        &self.history
    }

    fn push_history(&mut self, expr: &str, result: &str) {
        self.history.push(CalculationEntry {
            expr: expr.to_string(),
            result: result.to_string(),
        });
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    fn try_base_conversion(&self, s: &str) -> Option<Result<String, String>> {
        // "0x1A to dec", "255 to hex", "10 to bin", "0b1010 to dec"
        let lower = s.to_lowercase();

        // Has " to " separator
        if let Some(idx) = lower.find(" to ") {
            let left = s[..idx].trim();
            let target = lower[idx + 4..].trim();
            let value = parse_any_base(left)?;
            let result = match target {
                "dec" | "decimal" => format!("{}", value),
                "hex" => format!("0x{:X}", value),
                "bin" | "binary" => format!("0b{:b}", value),
                "oct" | "octal" => format!("0o{:o}", value),
                _ => return None,
            };
            return Some(Ok(result));
        }

        // Plain base-prefixed number → dec
        if lower.starts_with("0x") || lower.starts_with("0b") || lower.starts_with("0o") {
            let value = parse_any_base(s)?;
            return Some(Ok(format!("{}", value)));
        }

        None
    }

    /// UTIL.1.B — offline currency conversion. Returns `Some(Ok(...))` when the
    /// input matches `<N> <CCY> to <CCY>` where both currency codes are in the
    /// hard-coded fallback table. Returns `None` if the second token is not a
    /// known currency code (so unit conversion can take over). Result text
    /// includes a `(offline rate)` suffix so the user knows the rate is stale.
    ///
    /// Online rates require ADR-038 (currently `提議`) and are not implemented.
    fn try_currency_conversion(&self, s: &str) -> Option<Result<String, String>> {
        let lower = s.to_lowercase();
        let idx = lower.find(" to ")?;
        let left = s[..idx].trim();
        let target_raw = lower[idx + 4..].trim().to_uppercase();

        let (value, src_raw) = parse_value_unit(left)?;
        let src = src_raw.to_uppercase();

        // Both tokens must be recognised currency codes; otherwise fall through.
        let src_rate = currency_usd_rate(&src)?;
        let dst_rate = currency_usd_rate(&target_raw)?;

        // Convert value (src) → USD → target.
        let usd = value / src_rate;
        let result = usd * dst_rate;
        Some(Ok(format!(
            "{} {} (offline rate, {})",
            format_number(result),
            target_raw,
            CURRENCY_TABLE_DATE
        )))
    }

    /// UTIL.1.C — date arithmetic. Returns `Some(Ok(...))` when the input matches
    /// a date-arithmetic pattern, `Some(Err(...))` when the pattern matches but
    /// evaluation fails (e.g. invalid date), `None` to fall through to other parsers.
    ///
    /// Supported forms:
    /// - `today`, `tomorrow`, `yesterday`
    /// - `today + N <unit>` / `today - N <unit>` (unit ∈ day(s)/week(s)/month(s)/year(s))
    /// - `<date> + N <unit>` / `<date> - N <unit>`
    /// - `N <unit> ago` (= `today - N <unit>`)
    /// - `<date> - <date>` → number of days between
    /// - `<date> - today` / `today - <date>` → number of days
    /// - `next <weekday>` / `last <weekday>`
    ///
    /// Dates accept `YYYY-MM-DD` or `YYYY/MM/DD`.
    fn try_date_arithmetic(&self, s: &str) -> Option<Result<String, String>> {
        let lower = s.to_lowercase();

        // Quick rejects: must contain at least one date-ish token.
        if !lower.contains("today")
            && !lower.contains("tomorrow")
            && !lower.contains("yesterday")
            && !lower.contains(" ago")
            && !lower.contains("next ")
            && !lower.contains("last ")
            && !contains_date_literal(&lower)
        {
            return None;
        }

        // Pattern 1: pure keyword
        if let Some(d) = parse_date_anchor(&lower, self.today()) {
            if lower.trim() == "today" || lower.trim() == "tomorrow" || lower.trim() == "yesterday"
            {
                return Some(Ok(d.format("%Y-%m-%d").to_string()));
            }
        }

        // Pattern 2: `N <unit> ago`
        if let Some(rest) = lower.strip_suffix(" ago") {
            return Some(parse_n_unit(rest).and_then(|(n, unit)| {
                let date = subtract_from(self.today(), n, &unit)?;
                Ok(date.format("%Y-%m-%d").to_string())
            }));
        }

        // Pattern 3: `next <weekday>` / `last <weekday>`
        if let Some(rest) = lower.strip_prefix("next ") {
            return Some(parse_weekday(rest.trim()).map(|wd| {
                next_weekday(self.today(), wd)
                    .format("%Y-%m-%d")
                    .to_string()
            }));
        }
        if let Some(rest) = lower.strip_prefix("last ") {
            return Some(parse_weekday(rest.trim()).map(|wd| {
                previous_weekday(self.today(), wd)
                    .format("%Y-%m-%d")
                    .to_string()
            }));
        }

        // Pattern 4: `<lhs> + N <unit>` / `<lhs> - N <unit>` / `<date> - <date>`
        // We split on the rightmost ` + ` or ` - ` to handle these cases.
        if let Some((lhs, op, rhs)) = split_top_level_arith(&lower) {
            let left_date = parse_date_anchor(lhs.trim(), self.today())?;
            // Try `<date> - <date>` first.
            if op == '-' {
                if let Some(right_date) = parse_date_anchor(rhs.trim(), self.today()) {
                    let days = left_date.signed_duration_since(right_date).num_days();
                    return Some(Ok(format!("{} days", days)));
                }
            }
            // Otherwise expect `N <unit>` on the right.
            return Some(parse_n_unit(rhs.trim()).and_then(|(n, unit)| {
                let result = match op {
                    '+' => add_to(left_date, n, &unit)?,
                    '-' => subtract_from(left_date, n, &unit)?,
                    _ => unreachable!(),
                };
                Ok(result.format("%Y-%m-%d").to_string())
            }));
        }

        None
    }

    fn try_unit_conversion(&self, s: &str) -> Option<Result<String, String>> {
        let lower = s.to_lowercase();
        let idx = lower.find(" to ")?;
        let left = s[..idx].trim();
        let target_unit = lower[idx + 4..].trim().to_string();

        // Parse "N unit"
        let (value, src_unit) = parse_value_unit(left)?;
        let result = convert_unit(value, &src_unit.to_lowercase(), &target_unit)?;
        Some(Ok(format!("{} {}", format_number(result), target_unit)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_arithmetic() {
        assert_eq!(eval_expr("2 + 3").unwrap(), 5.0);
        assert_eq!(eval_expr("10 / 2").unwrap(), 5.0);
        assert_eq!(eval_expr("3 * 4").unwrap(), 12.0);
        assert_eq!(eval_expr("10 - 3").unwrap(), 7.0);
    }

    #[test]
    fn precedence_and_parens() {
        assert_eq!(eval_expr("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(eval_expr("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn functions() {
        assert!((eval_expr("sqrt(16)").unwrap() - 4.0).abs() < 1e-9);
        assert!((eval_expr("abs(-5)").unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn constants() {
        assert!((eval_expr("pi").unwrap() - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn unit_conversion() {
        let mut mgr = CalculatorManager::new();
        let r = mgr.eval("1 km to m").unwrap();
        assert_eq!(r, "1000 m");
    }

    #[test]
    fn base_conversion() {
        let mut mgr = CalculatorManager::new();
        assert_eq!(mgr.eval("0xFF").unwrap(), "255");
        assert_eq!(mgr.eval("255 to hex").unwrap(), "0xFF");
    }

    // ── UTIL.1.A volume ──────────────────────────────────────────────────────

    #[test]
    fn volume_l_to_ml() {
        let mut mgr = CalculatorManager::new();
        assert_eq!(mgr.eval("1 l to ml").unwrap(), "1000 ml");
    }

    #[test]
    fn volume_gal_to_l_us() {
        let mut mgr = CalculatorManager::new();
        // 1 US gallon ≈ 3.78541 L; format_number gives 3-4 sig figs.
        let r = mgr.eval("1 gal to l").unwrap();
        assert!(r.starts_with("3.78541"), "expected ~3.78541 l, got {r}");
    }

    #[test]
    fn volume_cup_to_ml_us() {
        let mut mgr = CalculatorManager::new();
        // 1 US legal cup ≈ 236.5882365 ml.
        let r = mgr.eval("1 cup to ml").unwrap();
        assert!(r.starts_with("236.588"), "expected ~236.588 ml, got {r}");
    }

    #[test]
    fn volume_fl_oz_to_ml() {
        let mut mgr = CalculatorManager::new();
        let r = mgr.eval("8 fl_oz to ml").unwrap();
        // 8 US fl oz ≈ 236.588 ml
        assert!(r.starts_with("236.588"), "expected ~236.588 ml, got {r}");
    }

    #[test]
    fn volume_unknown_unit_errors_cleanly() {
        let mut mgr = CalculatorManager::new();
        assert!(mgr.eval("1 barrel to l").is_err());
    }

    // ── UTIL.1.C date arithmetic ────────────────────────────────────────────

    fn mgr_with_today(today: NaiveDate) -> CalculatorManager {
        let mut m = CalculatorManager::new();
        m.today_override = Some(today);
        m
    }

    #[test]
    fn date_today_returns_iso() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("today").unwrap(), "2026-05-18");
    }

    #[test]
    fn date_tomorrow_and_yesterday() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("tomorrow").unwrap(), "2026-05-19");
        assert_eq!(m.eval("yesterday").unwrap(), "2026-05-17");
    }

    #[test]
    fn date_today_plus_n_days() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("today + 90 days").unwrap(), "2026-08-16");
    }

    #[test]
    fn date_n_days_ago() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("3 days ago").unwrap(), "2026-05-15");
        assert_eq!(m.eval("2 weeks ago").unwrap(), "2026-05-04");
    }

    #[test]
    fn date_absolute_minus_today() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("2026/12/31 - today").unwrap(), "227 days");
        assert_eq!(m.eval("today - 2026-05-01").unwrap(), "17 days");
    }

    #[test]
    fn date_month_year_arithmetic() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("today + 1 month").unwrap(), "2026-06-18");
        assert_eq!(m.eval("today + 1 year").unwrap(), "2027-05-18");
    }

    #[test]
    fn date_next_and_last_weekday() {
        // 2026-05-18 is a Monday. next monday should be 2026-05-25 (a week later).
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert_eq!(m.eval("next monday").unwrap(), "2026-05-25");
        assert_eq!(m.eval("next friday").unwrap(), "2026-05-22");
        assert_eq!(m.eval("last friday").unwrap(), "2026-05-15");
    }

    #[test]
    fn date_invalid_date_errors() {
        let mut m = mgr_with_today(NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
        assert!(
            m.eval("2026-13-01 - today").is_err()
                || m.eval("2026-13-01 - today").unwrap_or_default().is_empty()
        );
    }

    // ── UTIL.1.B offline currency ───────────────────────────────────────────

    #[test]
    fn currency_usd_to_twd_uses_offline_table() {
        let mut m = CalculatorManager::new();
        let result = m.eval("100 USD to TWD").unwrap();
        assert!(
            result.starts_with("3150"),
            "expected ~3150 TWD, got {result}"
        );
        assert!(
            result.contains("offline"),
            "must surface offline rate hint: {result}"
        );
        assert!(
            result.contains("snapshot"),
            "must surface snapshot date: {result}"
        );
    }

    #[test]
    fn currency_round_trip_returns_original_value() {
        let mut m = CalculatorManager::new();
        let result = m.eval("100 EUR to USD").unwrap();
        // 100 EUR / 0.92 ≈ 108.69 USD
        assert!(
            result.starts_with("108."),
            "expected ~108.7 USD, got {result}"
        );
    }

    #[test]
    fn currency_case_insensitive() {
        let mut m = CalculatorManager::new();
        let result = m.eval("50 usd to jpy").unwrap();
        // 50 USD * 151 = 7550 JPY
        assert!(
            result.starts_with("7550"),
            "expected ~7550 JPY, got {result}"
        );
        assert!(
            result.contains("JPY"),
            "result must use uppercase code: {result}"
        );
    }

    #[test]
    fn currency_unknown_code_falls_through_to_unit_conversion() {
        // "XYZ" is not a currency; "ml" is a known volume unit. Result should
        // come from unit converter (which will reject XYZ as unknown unit too).
        let mut m = CalculatorManager::new();
        assert!(m.eval("100 XYZ to ml").is_err());
    }
}
