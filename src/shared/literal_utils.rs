use crate::shared::constants::{FALSE_LITERAL, NULL_LITERAL, TRUE_LITERAL};

#[must_use]
pub fn is_boolean_or_null_literal(value: &str) -> bool {
    matches!(value, TRUE_LITERAL | FALSE_LITERAL | NULL_LITERAL)
}

#[must_use]
pub fn is_numeric_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let mut i = 0usize;

    if bytes[0] == b'-' {
        i += 1;
        if i >= bytes.len() {
            return false;
        }
    }

    let mut digit_count = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        digit_count += 1;
        i += 1;
    }

    if digit_count == 0 {
        return false;
    }

    // Leading zeros need no shortcut: the rest of the scan accepts them (`007`, `-05`, `007.5`
    // match spec §7.2's /^-?\d+(\.\d+)?(e[+-]?\d+)?$/i). A shortcut here used to quote any
    // string that merely STARTS with `0` and a digit (`007abc`, `00:`), which no reader takes
    // for a number.

    let mut saw_dot = false;
    if i < bytes.len() && bytes[i] == b'.' {
        saw_dot = true;
        i += 1;
        let mut frac_digits = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            frac_digits += 1;
            i += 1;
        }
        if frac_digits == 0 {
            return false;
        }
    }

    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut exp_digits = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            exp_digits += 1;
            i += 1;
        }
        if exp_digits == 0 {
            return false;
        }
    }

    if i != bytes.len() {
        return false;
    }

    if saw_dot {
        return true;
    }

    // Integer-like with no dot or exponent.
    true
}

#[must_use]
pub fn is_numeric_literal(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    if bytes[0] == b'-' {
        i += 1;
        if i >= bytes.len() {
            return false;
        }
    }

    if bytes[i] == b'0' {
        i += 1;
        if i < bytes.len() && bytes[i].is_ascii_digit() {
            return false;
        }
    } else if bytes[i].is_ascii_digit() {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return false;
    }

    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let mut frac_digits = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            frac_digits += 1;
            i += 1;
        }
        if frac_digits == 0 {
            return false;
        }
    }

    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut exp_digits = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            exp_digits += 1;
            i += 1;
        }
        if exp_digits == 0 {
            return false;
        }
    }

    if i != bytes.len() {
        return false;
    }

    trimmed.parse::<f64>().is_ok_and(f64::is_finite)
}
