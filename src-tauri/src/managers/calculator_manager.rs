 use std::collections::HashMap;

/// 計算機管理器：提供數學運算、單位換算、進位轉換。
pub struct CalculatorManager {
    history: Vec<CalculationEntry>,
}

#[derive(Debug, Clone)]
pub struct CalculationEntry {
    pub expr: String,
    pub result: String,
}

impl CalculatorManager {
    pub fn new() -> Self {
        Self { history: Vec::new() }
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

        // 單位換算：如 "5 km to m"
        if let Some(result) = self.try_unit_conversion(trimmed) {
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
        ("m", 1.0), ("km", 1000.0), ("cm", 0.01), ("mm", 0.001),
        ("mi", 1609.344), ("ft", 0.3048), ("in", 0.0254), ("yd", 0.9144),
        // Weight (kg)
        ("kg", 1.0), ("g", 0.001), ("mg", 0.000001), ("lb", 0.453592), ("oz", 0.0283495),
        // Temperature — handled separately
        // Time (seconds)
        ("s", 1.0), ("ms", 0.001), ("min", 60.0), ("h", 3600.0), ("hr", 3600.0),
        ("day", 86400.0), ("week", 604800.0),
        // Area (m²)
        ("m2", 1.0), ("km2", 1e6), ("cm2", 0.0001), ("ha", 10000.0), ("acre", 4046.86),
        // Speed (m/s)
        ("m/s", 1.0), ("km/h", 1.0 / 3.6), ("mph", 0.44704),
        // Data (bytes)
        ("b", 1.0), ("kb", 1024.0), ("mb", 1048576.0), ("gb", 1073741824.0),
        ("tb", 1099511627776.0),
    ].iter().cloned().collect();

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
                Some('+') => { self.consume(); left += self.parse_term()?; }
                Some('-') => { self.consume(); left -= self.parse_term()?; }
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
                Some('*') => { self.consume(); let r = self.parse_power()?; left *= r; }
                Some('/') => {
                    self.consume();
                    let r = self.parse_power()?;
                    if r == 0.0 { return Err("division by zero".into()); }
                    left /= r;
                }
                Some('%') => {
                    self.consume();
                    let r = self.parse_power()?;
                    if r == 0.0 { return Err("modulo by zero".into()); }
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
                if self.peek() == Some(')') { self.consume(); } else { return Err("expected ')'".into()); }
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            Some(c) if c.is_ascii_alphabetic() => self.parse_name(),
            other => Err(format!("unexpected {:?}", other)),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E') {
            self.consume();
        }
        let s: String = self.src[start..self.pos].iter().collect();
        s.parse::<f64>().map_err(|e| e.to_string())
    }

    fn parse_name(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
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
            if self.peek() == Some(')') { self.consume(); } else { return Err("expected ')'".into()); }
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
}