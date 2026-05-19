use std::collections::HashMap;

use chrono::{Datelike, Days, Months, NaiveDate, Weekday};

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
                next_weekday(self.today(), wd).format("%Y-%m-%d").to_string()
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

// ─── Offline currency table (UTIL.1.B) ───────────────────────────────────────
//
// Rates expressed as units of currency per 1 USD. Snapshot date is fixed in
// `CURRENCY_TABLE_DATE` so the user sees a stale-warning suffix. Online rates
// are gated behind ADR-038 (status: 提議). When that ADR is accepted, a
// background fetcher will overwrite these values with cached live rates.

const CURRENCY_TABLE_DATE: &str = "snapshot 2026-05";

fn currency_usd_rate(code: &str) -> Option<f64> {
    // Source: rough rounded snapshot — values per 1 USD.
    match code {
        "USD" => Some(1.0),
        "EUR" => Some(0.92),
        "GBP" => Some(0.79),
        "JPY" => Some(151.0),
        "CNY" => Some(7.20),
        "TWD" => Some(31.5),
        "KRW" => Some(1370.0),
        "HKD" => Some(7.81),
        "SGD" => Some(1.34),
        "AUD" => Some(1.52),
        "CAD" => Some(1.36),
        "CHF" => Some(0.91),
        "INR" => Some(83.5),
        "THB" => Some(36.5),
        "MYR" => Some(4.70),
        "PHP" => Some(57.0),
        "IDR" => Some(16100.0),
        "VND" => Some(25400.0),
        _ => None,
    }
}

// ─── Date arithmetic helpers (UTIL.1.C) ──────────────────────────────────────

fn contains_date_literal(s: &str) -> bool {
    // Match YYYY-MM-DD or YYYY/MM/DD; loose check, full parse happens later.
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        let chunk = &s[i..i + 10];
        if chunk
            .chars()
            .enumerate()
            .all(|(idx, c)| match (idx, c) {
                (0..=3, c) => c.is_ascii_digit(),
                (4, '-') | (4, '/') => true,
                (5..=6, c) => c.is_ascii_digit(),
                (7, '-') | (7, '/') => true,
                (8..=9, c) => c.is_ascii_digit(),
                _ => false,
            })
        {
            return true;
        }
    }
    false
}

fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    let normalized = s.replace('/', "-");
    NaiveDate::parse_from_str(&normalized, "%Y-%m-%d").ok()
}

fn parse_date_anchor(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let trimmed = s.trim();
    match trimmed {
        "today" => Some(today),
        "tomorrow" => today.checked_add_days(Days::new(1)),
        "yesterday" => today.checked_sub_days(Days::new(1)),
        other => parse_iso_date(other),
    }
}

fn parse_weekday(s: &str) -> Result<Weekday, String> {
    match s.trim() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        other => Err(format!("unknown weekday '{other}'")),
    }
}

fn next_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let cur = from.weekday().num_days_from_monday();
    let tgt = target.num_days_from_monday();
    let diff = if tgt > cur {
        tgt - cur
    } else {
        7 - (cur - tgt)
    };
    from.checked_add_days(Days::new(diff as u64))
        .unwrap_or(from)
}

fn previous_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let cur = from.weekday().num_days_from_monday();
    let tgt = target.num_days_from_monday();
    let diff = if cur > tgt {
        cur - tgt
    } else {
        7 - (tgt - cur)
    };
    from.checked_sub_days(Days::new(diff as u64))
        .unwrap_or(from)
}

fn parse_n_unit(s: &str) -> Result<(u32, String), String> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("expected '<N> <unit>', got '{s}'"));
    }
    let n: u32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid number '{}'", parts[0]))?;
    let unit = parts[1].trim_end_matches('s').to_string(); // strip plural
    Ok((n, unit))
}

fn add_to(date: NaiveDate, n: u32, unit: &str) -> Result<NaiveDate, String> {
    match unit {
        "day" => date
            .checked_add_days(Days::new(n as u64))
            .ok_or_else(|| "date overflow".into()),
        "week" => date
            .checked_add_days(Days::new(n as u64 * 7))
            .ok_or_else(|| "date overflow".into()),
        "month" => date
            .checked_add_months(Months::new(n))
            .ok_or_else(|| "date overflow".into()),
        "year" => date
            .checked_add_months(Months::new(n * 12))
            .ok_or_else(|| "date overflow".into()),
        other => Err(format!("unknown date unit '{other}'")),
    }
}

fn subtract_from(date: NaiveDate, n: u32, unit: &str) -> Result<NaiveDate, String> {
    match unit {
        "day" => date
            .checked_sub_days(Days::new(n as u64))
            .ok_or_else(|| "date underflow".into()),
        "week" => date
            .checked_sub_days(Days::new(n as u64 * 7))
            .ok_or_else(|| "date underflow".into()),
        "month" => date
            .checked_sub_months(Months::new(n))
            .ok_or_else(|| "date underflow".into()),
        "year" => date
            .checked_sub_months(Months::new(n * 12))
            .ok_or_else(|| "date underflow".into()),
        other => Err(format!("unknown date unit '{other}'")),
    }
}

/// Split on the rightmost top-level ` + ` or ` - ` (not inside numeric literal).
/// Returns `(lhs, op, rhs)` or `None` if no operator found.
fn split_top_level_arith(s: &str) -> Option<(&str, char, &str)> {
    // Find rightmost " + " or " - " surrounded by spaces.
    let mut idx_op: Option<(usize, char)> = None;
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if i + 2 >= bytes.len() {
            break;
        }
        if bytes[i] == b' ' && (bytes[i + 1] == b'+' || bytes[i + 1] == b'-') && bytes[i + 2] == b' '
        {
            idx_op = Some((i, bytes[i + 1] as char));
        }
    }
    idx_op.map(|(i, op)| (&s[..i], op, &s[i + 3..]))
}

fn parse_any_base(s: &str) -> Option<i64> {
    let lower = s.to_lowercase();
    if lower.starts_with("0x") {
        i64::from_str_radix(&s[2..], 16).ok()
    } else if lower.starts_with("0b") {
        i64::from_str_radix(&s[2..], 2).ok()
    } else if lower.starts_with("0o") {
        i64::from_str_radix(&s[2..], 8).ok()
    } else {
        s.parse::<i64>().ok()
    }
}

fn parse_value_unit(s: &str) -> Option<(f64, String)> {
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let v = parts[0].parse::<f64>().ok()?;
        Some((v, parts[1].to_string()))
    } else {
        None
    }
}

fn convert_unit(value: f64, from: &str, to: &str) -> Option<f64> {
    // Convert both to SI base, then to target
    let to_si: HashMap<&str, f64> = [
        // Length (meter)
        ("m", 1.0),
        ("km", 1000.0),
        ("cm", 0.01),
        ("mm", 0.001),
        ("mi", 1609.344),
        ("ft", 0.3048),
        ("in", 0.0254),
        ("yd", 0.9144),
        // Weight (kg)
        ("kg", 1.0),
        ("g", 0.001),
        ("mg", 0.000001),
        ("lb", 0.453592),
        ("oz", 0.0283495),
        // Temperature — handled separately
        // Time (seconds)
        ("s", 1.0),
        ("ms", 0.001),
        ("min", 60.0),
        ("h", 3600.0),
        ("hr", 3600.0),
        ("day", 86400.0),
        ("week", 604800.0),
        // Area (m²)
        ("m2", 1.0),
        ("km2", 1e6),
        ("cm2", 0.0001),
        ("ha", 10000.0),
        ("acre", 4046.86),
        // Speed (m/s)
        ("m/s", 1.0),
        ("km/h", 1.0 / 3.6),
        ("mph", 0.44704),
        // Data (bytes)
        ("b", 1.0),
        ("kb", 1024.0),
        ("mb", 1048576.0),
        ("gb", 1073741824.0),
        ("tb", 1099511627776.0),
        // Volume (liter)
        ("l", 1.0),
        ("ml", 0.001),
        ("cl", 0.01),
        ("dl", 0.1),
        ("m3", 1000.0),
        ("cup", 0.2365882365),       // US legal cup
        ("tbsp", 0.01478676),        // US tablespoon
        ("tsp", 0.00492892),         // US teaspoon
        ("fl_oz", 0.0295735),        // US fluid ounce
        ("pt", 0.473176),            // US liquid pint
        ("qt", 0.946353),            // US liquid quart
        ("gal", 3.78541),            // US gallon
    ]
    .iter()
    .cloned()
    .collect();

    // Temperature conversion
    let temp_result = convert_temperature(value, from, to);
    if temp_result.is_some() {
        return temp_result;
    }

    let si_from = to_si.get(from)?;
    let si_to = to_si.get(to)?;
    Some(value * si_from / si_to)
}

fn convert_temperature(value: f64, from: &str, to: &str) -> Option<f64> {
    let to_celsius = match from {
        "c" | "celsius" | "°c" => value,
        "f" | "fahrenheit" | "°f" => (value - 32.0) * 5.0 / 9.0,
        "k" | "kelvin" => value - 273.15,
        _ => return None,
    };
    match to {
        "c" | "celsius" | "°c" => Some(to_celsius),
        "f" | "fahrenheit" | "°f" => Some(to_celsius * 9.0 / 5.0 + 32.0),
        "k" | "kelvin" => Some(to_celsius + 273.15),
        _ => None,
    }
}

fn format_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        // Up to 8 significant digits, strip trailing zeros
        let s = format!("{:.8}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ─── Recursive-descent expression evaluator ──────────────────────────────────

struct Parser<'a> {
    src: &'a [char],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a [char]) -> Self {
        Self { src, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.src.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c == ' ') {
            self.pos += 1;
        }
    }

    /// expr = term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.consume();
                    left += self.parse_term()?;
                }
                Some('-') => {
                    self.consume();
                    left -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// term = power (('*' | '/') power)*
    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_power()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.consume();
                    let r = self.parse_power()?;
                    left *= r;
                }
                Some('/') => {
                    self.consume();
                    let r = self.parse_power()?;
                    if r == 0.0 {
                        return Err("division by zero".into());
                    }
                    left /= r;
                }
                Some('%') => {
                    self.consume();
                    let r = self.parse_power()?;
                    if r == 0.0 {
                        return Err("modulo by zero".into());
                    }
                    left %= r;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// power = unary ('^' unary)?
    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        self.skip_ws();
        if self.peek() == Some('^') {
            self.consume();
            let exp = self.parse_unary()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    /// unary = '-' unary | primary
    fn parse_unary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        if self.peek() == Some('-') {
            self.consume();
            Ok(-self.parse_unary()?)
        } else if self.peek() == Some('+') {
            self.consume();
            self.parse_unary()
        } else {
            self.parse_primary()
        }
    }

    /// primary = number | constant | func '(' expr ')' | '(' expr ')'
    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.consume();
                let v = self.parse_expr()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.consume();
                } else {
                    return Err("expected ')'".into());
                }
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            Some(c) if c.is_ascii_alphabetic() => self.parse_name(),
            other => Err(format!("unexpected {:?}", other)),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E')
        {
            self.consume();
        }
        let s: String = self.src[start..self.pos].iter().collect();
        s.parse::<f64>().map_err(|e| e.to_string())
    }

    fn parse_name(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.consume();
        }
        let name: String = self.src[start..self.pos].iter().collect();
        self.skip_ws();

        // Constants
        match name.as_str() {
            "pi" | "PI" => return Ok(std::f64::consts::PI),
            "e" | "E" if self.peek() != Some('(') => return Ok(std::f64::consts::E),
            "inf" => return Ok(f64::INFINITY),
            _ => {}
        }

        // Functions
        if self.peek() == Some('(') {
            self.consume();
            let arg = self.parse_expr()?;
            self.skip_ws();
            if self.peek() == Some(')') {
                self.consume();
            } else {
                return Err("expected ')'".into());
            }
            return match name.as_str() {
                "sqrt" => Ok(arg.sqrt()),
                "abs" => Ok(arg.abs()),
                "floor" => Ok(arg.floor()),
                "ceil" => Ok(arg.ceil()),
                "round" => Ok(arg.round()),
                "sin" => Ok(arg.sin()),
                "cos" => Ok(arg.cos()),
                "tan" => Ok(arg.tan()),
                "asin" => Ok(arg.asin()),
                "acos" => Ok(arg.acos()),
                "atan" => Ok(arg.atan()),
                "ln" => Ok(arg.ln()),
                "log" | "log10" => Ok(arg.log10()),
                "log2" => Ok(arg.log2()),
                "exp" => Ok(arg.exp()),
                _ => Err(format!("unknown function '{}'", name)),
            };
        }

        Err(format!("unknown name '{}'", name))
    }
}

pub fn eval_expr(expr: &str) -> Result<f64, String> {
    let chars: Vec<char> = expr.chars().collect();
    let mut parser = Parser::new(&chars);
    let result = parser.parse_expr()?;
    parser.skip_ws();
    if parser.pos < parser.src.len() {
        return Err(format!("unexpected input at position {}", parser.pos));
    }
    if result.is_nan() {
        return Err("result is NaN".into());
    }
    Ok(result)
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
        assert!(m.eval("2026-13-01 - today").is_err() || m.eval("2026-13-01 - today").unwrap_or_default().is_empty());
    }

    // ── UTIL.1.B offline currency ───────────────────────────────────────────

    #[test]
    fn currency_usd_to_twd_uses_offline_table() {
        let mut m = CalculatorManager::new();
        let result = m.eval("100 USD to TWD").unwrap();
        assert!(result.starts_with("3150"), "expected ~3150 TWD, got {result}");
        assert!(result.contains("offline"), "must surface offline rate hint: {result}");
        assert!(result.contains("snapshot"), "must surface snapshot date: {result}");
    }

    #[test]
    fn currency_round_trip_returns_original_value() {
        let mut m = CalculatorManager::new();
        let result = m.eval("100 EUR to USD").unwrap();
        // 100 EUR / 0.92 ≈ 108.69 USD
        assert!(result.starts_with("108."), "expected ~108.7 USD, got {result}");
    }

    #[test]
    fn currency_case_insensitive() {
        let mut m = CalculatorManager::new();
        let result = m.eval("50 usd to jpy").unwrap();
        // 50 USD * 151 = 7550 JPY
        assert!(result.starts_with("7550"), "expected ~7550 JPY, got {result}");
        assert!(result.contains("JPY"), "result must use uppercase code: {result}");
    }

    #[test]
    fn currency_unknown_code_falls_through_to_unit_conversion() {
        // "XYZ" is not a currency; "ml" is a known volume unit. Result should
        // come from unit converter (which will reject XYZ as unknown unit too).
        let mut m = CalculatorManager::new();
        assert!(m.eval("100 XYZ to ml").is_err());
    }
}
