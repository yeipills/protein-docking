//! Float formatting identical to Python's `repr(float)` / `str(float)`.
//!
//! The legacy text formats were produced with `str(x)`, so reproducing them byte for byte
//! requires the same notation rules: shortest round-trip digits, fixed notation for decimal
//! exponents in `[-4, 16)`, scientific notation otherwise, and a trailing `.0` for integers.

use std::fmt::Write;

pub fn py_repr(x: f64) -> String {
    let mut s = String::with_capacity(24);
    write_py_repr(&mut s, x);
    s
}

pub fn write_py_repr(out: &mut String, x: f64) {
    if x.is_nan() {
        out.push_str("nan");
        return;
    }
    if x.is_infinite() {
        out.push_str(if x > 0.0 { "inf" } else { "-inf" });
        return;
    }
    if x == 0.0 {
        out.push_str(if x.is_sign_negative() { "-0.0" } else { "0.0" });
        return;
    }

    // Rust's `{:e}` yields the shortest round-trip digits, e.g. "-4.1306666666666665e1".
    let mut buf = StackBuf::default();
    write!(buf, "{:e}", x).unwrap();
    let text = buf.as_str();
    let (mantissa, exp) = text.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let (neg, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => (true, m),
        None => (false, mantissa),
    };
    let mut digits_buf = [0u8; 24];
    let mut nd = 0usize;
    for b in mantissa.bytes().filter(|b| *b != b'.') {
        digits_buf[nd] = b;
        nd += 1;
    }
    // Only ASCII digits were copied.
    let digits = std::str::from_utf8(&digits_buf[..nd]).unwrap();
    let nd = nd as i32;

    if neg {
        out.push('-');
    }
    if (-4..16).contains(&exp) {
        if exp < 0 {
            out.push_str("0.");
            for _ in 0..(-exp - 1) {
                out.push('0');
            }
            out.push_str(digits);
        } else if nd <= exp + 1 {
            out.push_str(digits);
            for _ in 0..(exp + 1 - nd) {
                out.push('0');
            }
            out.push_str(".0");
        } else {
            let split = (exp + 1) as usize;
            out.push_str(&digits[..split]);
            out.push('.');
            out.push_str(&digits[split..]);
        }
    } else {
        out.push_str(&digits[..1]);
        if nd > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        write!(out, "e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs()).unwrap();
    }
}

/// Fixed-size formatting buffer (an `f64` in `{:e}` form needs at most 25 bytes).
#[derive(Default)]
struct StackBuf {
    data: [u8; 32],
    len: usize,
}

impl StackBuf {
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len]).unwrap()
    }
}

impl Write for StackBuf {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.data.len() {
            return Err(std::fmt::Error);
        }
        self.data[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::py_repr;

    #[test]
    fn matches_python() {
        let cases = [
            (3.0, "3.0"),
            (-41.306666666666665, "-41.306666666666665"),
            (0.1, "0.1"),
            (1e-5, "1e-05"),
            (0.0001, "0.0001"),
            (1.5e16, "1.5e+16"),
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (123456.789, "123456.789"),
            (-0.0, "-0.0"),
            (0.0, "0.0"),
            (2.0 * std::f64::consts::PI, "6.283185307179586"),
            (1.8369701987210297e-16, "1.8369701987210297e-16"),
            (100.0, "100.0"),
            (-58.012, "-58.012"),
        ];
        for (x, want) in cases {
            assert_eq!(py_repr(x), want, "for {x:e}");
        }
    }
}
