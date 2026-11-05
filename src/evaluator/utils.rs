use std::fmt::Write;

const MAX_SAFE_INT: f64 = ((1i64 << 53) - 1) as f64;

// TODO: eeeh, wtf
#[inline]
pub fn unparse_number(b: &mut String, v: f64) {
    if v == v.floor() {
        if v == 0.0 {
            if v.is_sign_negative() {
                b.push_str("-0");
            } else {
                b.push('0');
            }
            return;
        }
        if v.abs() <= MAX_SAFE_INT {
            b.push_str(itoa::Buffer::new().format(v as i64));
            return;
        }
        let _ = write!(b, "{v:.0}");
        return;
    }

    if v.is_sign_negative() {
        b.push('-');
    }

    // 1. Get the 17 significant digits `d` and decimal point position `dp`
    let (mut d, mut p) = fpfmt::fixed_width(v.abs(), 17);
    let dp = 17 + p;

    // 2. Strip trailing zeros (e.g. 15000000000000000 -> 15)
    while d % 10 == 0 {
        d /= 10;
        p += 1;
    }
    let mut ibuf = itoa::Buffer::new();
    let s = ibuf.format(d);

    // 3. Format into one of the 3 `%.17g` styles
    if !(-3..=17).contains(&dp) {
        // Case A: Scientific notation (e.g. "1e-14")
        let mut ebuf = [0u8; 32];
        let n = fpfmt::fmt_float(&mut ebuf, d, p, s.len() as i32);
        b.push_str(unsafe { std::str::from_utf8_unchecked(&ebuf[..n]) });
    } else if dp <= 0 {
        // Case B: Leading zeros after dot (e.g. "0.0001")
        b.push_str(&"0.0000"[..(2 - dp) as usize]);
        b.push_str(s);
    } else {
        // Case C: Normal decimal (e.g. "13.37" -> "13" + "." + "369999999999999")
        b.push_str(&s[..dp as usize]);
        b.push('.');
        b.push_str(&s[dp as usize..]);
    }
}

/*
use std::fmt::Write as _;

  /// 5^0 ..= 5^32. 2^53 · 5^32 < 2^128, so `m · 5^k` is exact in u128 for k ≤ 32.
  const POW5: [u128; 33] = {
      let mut t = [1u128; 33];
      let mut i = 1;
      while i < 33 {
          t[i] = t[i - 1] * 5;
          i += 1;
      }
      t
  };

  const TEN16: u64 = 10_000_000_000_000_000;
  const TEN17: u64 = 100_000_000_000_000_000;

  /// The 17 correctly rounded (half-to-even) significant digits of `v`, and `k`
  /// such that `v ≈ digits · 10^-k`. This is the digit source of Go's `%.17g`.
  /// `v` must be finite and positive.
  fn digits17(v: f64) -> (u64, i32) {
      if v < 1e-15 {
          return digits17_slow(v);
      }
      // v = m · 2^e2 exactly, with m ∈ [2^52, 2^53) (v ≥ 1e-15, so it is normal).
      let bits = v.to_bits();
      let m = (bits & ((1 << 52) - 1)) | (1 << 52);
      let e2 = ((bits >> 52) & 0x7ff) as i32 - 1075;
      // floor(log10 2^x) for x = e2 + 52 (ryu's constant); the true decimal
      // exponent of v is this or one more, which the loop below corrects.
      let mut k = 16 - (((e2 + 52) * 78913) >> 18);
      loop {
          // v · 10^k = m · 5^k / 2^shift, exact in u128.
          let shift = -e2 - k;
          debug_assert!((1..=32).contains(&k) && (0..128).contains(&shift));
          let prod = m as u128 * POW5[k as usize];
          let mut d = (prod >> shift) as u64;
          if shift > 0 {
              let rem = prod & ((1u128 << shift) - 1);
              let half = 1u128 << (shift - 1);
              if rem > half || (rem == half && d & 1 == 1) {
                  d += 1;
              }
          }
          if d >= TEN17 {
              k -= 1; // decimal exponent was one higher than estimated (or a carry)
          } else if d < TEN16 {
              k += 1; // not reachable with a floor estimate; kept for robustness
          } else {
              return (d, k);
          }
      }
  }

  /// Below 1e-15 `m · 5^k` no longer fits in u128; let std do the exact rounding.
  #[cold]
  #[inline(never)]
  fn digits17_slow(v: f64) -> (u64, i32) {
      let mut buf = [0u8; 32];
      let rest = {
          let mut w = &mut buf[..];
          std::io::Write::write_fmt(&mut w, format_args!("{v:.16e}")).unwrap();
          w.len()
      };
      let s = &buf[..32 - rest]; // "d.dddddddddddddddde-NNN"
      let d = std::iter::once(s[0])
          .chain(s[2..18].iter().copied())
          .fold(0u64, |acc, c| acc * 10 + u64::from(c - b'0'));
      let exp: i32 = std::str::from_utf8(&s[19..]).unwrap().parse().unwrap();
      (d, 16 - exp)
  }

  /// go-jsonnet's `unparseNumber`:
  ///   v == floor(v)  ->  fmt.Sprintf("%.0f", v)
  ///   otherwise      ->  fmt.Sprintf("%.17g", v)
  pub fn unparse_number(b: &mut String, v: f64) {
      debug_assert!(v.is_finite(), "jsonnet never manifests NaN or Inf");

      if v == v.floor() {
          if v == 0.0 {
              b.push_str(if v.is_sign_negative() { "-0" } else { "0" });
          } else if v.abs() < (1u64 << 63) as f64 {
              // `as i64` is exact for every integer-valued f64 in range.
              b.push_str(itoa::Buffer::new().format(v as i64));
          } else {
              // Full decimal expansion, same as Go's %.0f.
              let _ = write!(b, "{v:.0}");
          }
          return;
      }

      if v < 0.0 {
          b.push('-');
      }
      let (d, k) = digits17(v.abs());
      let mut ibuf = itoa::Buffer::new();
      let s = ibuf.format(d).trim_end_matches('0'); // 17 digits minus trailing zeros
      let dp = 17 - k; // digits before the decimal point: v = 0.ddd · 10^dp

      if dp < -3 {
          // %e layout, "d.ddde-05": the exponent dp-1 is ≤ -5, padded to 2 digits.
          b.push_str(&s[..1]);
          if s.len() > 1 {
              b.push('.');
              b.push_str(&s[1..]);
          }
          b.push_str("e-");
          let e = 1 - dp;
          if e < 10 {
              b.push('0');
          }
          b.push_str(itoa::Buffer::new().format(e));
      } else if dp <= 0 {
          // "0.000ddd"
          b.push_str(&"0.000"[..(2 - dp) as usize]);
          b.push_str(s);
      } else {
          // "dd.ddd". A non-integer always has more significant digits than
          // integer digits, so the dot never lands at the end.
          let dp = dp as usize;
          debug_assert!(dp < s.len());
          b.push_str(&s[..dp]);
          b.push('.');
          b.push_str(&s[dp..]);
      }
  }
*/
