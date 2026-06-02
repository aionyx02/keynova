//! Currency table, base/value parsing, unit + temperature conversion, and
//! number formatting (UTIL.1.A / UTIL.1.B).
//!
//! Extracted from `calculator_manager.rs` (REF.9.B) as a pure structural move;
//! behavior unchanged.

use std::collections::HashMap;

// ─── Offline currency table (UTIL.1.B) ───────────────────────────────────────
//
// Rates expressed as units of currency per 1 USD. Snapshot date is fixed in
// `CURRENCY_TABLE_DATE` so the user sees a stale-warning suffix. Online rates
// are gated behind ADR-038 (status: 提議). When that ADR is accepted, a
// background fetcher will overwrite these values with cached live rates.

pub(super) const CURRENCY_TABLE_DATE: &str = "snapshot 2026-05";

pub(super) fn currency_usd_rate(code: &str) -> Option<f64> {
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

pub(super) fn parse_any_base(s: &str) -> Option<i64> {
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

pub(super) fn parse_value_unit(s: &str) -> Option<(f64, String)> {
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let v = parts[0].parse::<f64>().ok()?;
        Some((v, parts[1].to_string()))
    } else {
        None
    }
}

pub(super) fn convert_unit(value: f64, from: &str, to: &str) -> Option<f64> {
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
        ("cup", 0.2365882365), // US legal cup
        ("tbsp", 0.01478676),  // US tablespoon
        ("tsp", 0.00492892),   // US teaspoon
        ("fl_oz", 0.0295735),  // US fluid ounce
        ("pt", 0.473176),      // US liquid pint
        ("qt", 0.946353),      // US liquid quart
        ("gal", 3.78541),      // US gallon
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

pub(super) fn format_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        // Up to 8 significant digits, strip trailing zeros
        let s = format!("{:.8}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
