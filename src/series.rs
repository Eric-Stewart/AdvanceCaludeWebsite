//! Truncated power series over the rationals.
//!
//! Everything the site displays as a Taylor expansion is computed here and
//! rendered straight to LaTeX, so `tan x` really is `sin x / cos x` evaluated
//! by series division rather than a table of Bernoulli numbers copied in.

use crate::rational::Rat;

/// Coefficients `c[0] + c[1] x + c[2] x^2 + ...`, truncated at `len()`.
#[derive(Clone, Debug)]
pub struct Series(Vec<Rat>);

impl Series {
    pub fn from_fn(n: usize, f: impl Fn(usize) -> Rat) -> Series {
        Series((0..n).map(f).collect())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn coeff(&self, i: usize) -> Rat {
        self.0.get(i).copied().unwrap_or(Rat::ZERO)
    }

    /// Cauchy product, truncated to the shorter of the two operands.
    /// No endpoint needs it yet; it is the inverse that pins down `div`, and
    /// the round-trip test is what proves the division recurrence correct.
    #[allow(dead_code)]
    pub fn mul(&self, other: &Series) -> Series {
        let n = self.len().min(other.len());
        let mut out = vec![Rat::ZERO; n];
        for i in 0..n {
            for j in 0..=i {
                out[i] = out[i] + self.coeff(j) * other.coeff(i - j);
            }
        }
        Series(out)
    }

    /// `self / other`, requiring `other[0] != 0`. Standard recurrence:
    /// `q[i] = (a[i] - sum_{k<i} q[k] b[i-k]) / b[0]`.
    pub fn div(&self, other: &Series) -> Series {
        assert!(!other.coeff(0).is_zero(), "series division needs a unit constant term");
        let n = self.len().min(other.len());
        let b0 = other.coeff(0);
        let mut q = vec![Rat::ZERO; n];
        for i in 0..n {
            let mut acc = self.coeff(i);
            for k in 0..i {
                acc = acc - q[k] * other.coeff(i - k);
            }
            q[i] = acc / b0;
        }
        Series(q)
    }

    /// Render as `term + term - term + \cdots`, skipping zero coefficients.
    /// `var` is the variable name; a trailing `\cdots` marks the truncation.
    pub fn to_latex(&self, var: &str, ellipsis: bool) -> String {
        let mut out = String::new();
        for (i, c) in self.0.iter().enumerate() {
            if c.is_zero() {
                continue;
            }
            let body = match i {
                0 => String::new(),
                1 => var.to_string(),
                _ => format!("{}^{{{}}}", var, i),
            };
            if out.is_empty() {
                if c.is_negative() {
                    out.push('-');
                }
            } else {
                out.push_str(if c.is_negative() { " - " } else { " + " });
            }
            out.push_str(&c.to_latex_times(&body));
        }
        if out.is_empty() {
            out.push('0');
        }
        if ellipsis {
            out.push_str(" + \\cdots");
        }
        out
    }
}

fn factorial(n: usize) -> i128 {
    (1..=n as i128).product::<i128>().max(1)
}

/// One expandable function: an identifier, its LaTeX name, and its series.
pub struct Expansion {
    pub id: &'static str,
    pub name: &'static str,
    pub lhs: String,
    pub series: Series,
    pub note: &'static str,
}

/// How many terms we can emit before `i128` factorials overflow.
/// 33! already exceeds `i128::MAX`, and denominators in `tan`/`sec` grow
/// faster still, so we clamp well short of that.
pub const MAX_TERMS: usize = 18;

pub fn expand(id: &str, terms: usize) -> Option<Expansion> {
    let n = terms.clamp(2, MAX_TERMS);

    // Building blocks, all to the requested length.
    let exp = Series::from_fn(n, |k| Rat::new(1, factorial(k)));
    let sin = Series::from_fn(n, |k| {
        if k % 2 == 1 {
            let sign = if (k / 2) % 2 == 0 { 1 } else { -1 };
            Rat::new(sign, factorial(k))
        } else {
            Rat::ZERO
        }
    });
    let cos = Series::from_fn(n, |k| {
        if k % 2 == 0 {
            let sign = if (k / 2) % 2 == 0 { 1 } else { -1 };
            Rat::new(sign, factorial(k))
        } else {
            Rat::ZERO
        }
    });
    let one = Series::from_fn(n, |k| if k == 0 { Rat::ONE } else { Rat::ZERO });

    let (name, lhs, series, note) = match id {
        "exp" => ("exp", "e^{x}".to_string(), exp, "\\text{entire; } R = \\infty"),
        "sin" => ("sin", "\\sin x".to_string(), sin, "\\text{odd: } f(-x) = -f(x)"),
        "cos" => ("cos", "\\cos x".to_string(), cos, "\\text{even: } f(-x) = f(x)"),
        "tan" => (
            "tan",
            "\\tan x".to_string(),
            sin.div(&cos),
            "\\text{by series division: } \\tan x = \\sin x / \\cos x",
        ),
        "sec" => (
            "sec",
            "\\sec x".to_string(),
            one.div(&cos),
            "\\text{poles at } x = \\tfrac{\\pi}{2} + k\\pi",
        ),
        "sinh" => (
            "sinh",
            "\\sinh x".to_string(),
            Series::from_fn(n, |k| {
                if k % 2 == 1 {
                    Rat::new(1, factorial(k))
                } else {
                    Rat::ZERO
                }
            }),
            "\\text{odd: } f(-x) = -f(x)",
        ),
        "cosh" => (
            "cosh",
            "\\cosh x".to_string(),
            Series::from_fn(n, |k| {
                if k % 2 == 0 {
                    Rat::new(1, factorial(k))
                } else {
                    Rat::ZERO
                }
            }),
            "\\text{even: } f(-x) = f(x)",
        ),
        "tanh" => {
            let sinh = Series::from_fn(n, |k| {
                if k % 2 == 1 {
                    Rat::new(1, factorial(k))
                } else {
                    Rat::ZERO
                }
            });
            let cosh = Series::from_fn(n, |k| {
                if k % 2 == 0 {
                    Rat::new(1, factorial(k))
                } else {
                    Rat::ZERO
                }
            });
            (
                "tanh",
                "\\tanh x".to_string(),
                sinh.div(&cosh),
                "\\text{by series division: } \\tanh x = \\sinh x / \\cosh x",
            )
        }
        "log1p" => (
            "log1p",
            "\\ln(1+x)".to_string(),
            Series::from_fn(n, |k| {
                if k == 0 {
                    Rat::ZERO
                } else {
                    Rat::new(if k % 2 == 1 { 1 } else { -1 }, k as i128)
                }
            }),
            "R = 1\\text{; branch point at } x = -1",
        ),
        "atan" => (
            "atan",
            "\\arctan x".to_string(),
            Series::from_fn(n, |k| {
                if k % 2 == 1 {
                    Rat::new(if (k / 2) % 2 == 0 { 1 } else { -1 }, k as i128)
                } else {
                    Rat::ZERO
                }
            }),
            "\\text{at } x = 1\\text{: } \\tfrac{\\pi}{4} = 1 - \\tfrac13 + \\tfrac15 - \\cdots",
        ),
        "geom" => (
            "geom",
            "\\frac{1}{1-x}".to_string(),
            Series::from_fn(n, |_| Rat::ONE),
            "\\text{generating function of } (1,1,1,\\ldots)",
        ),
        "bessel0" => (
            "bessel0",
            "J_{0}(x)".to_string(),
            Series::from_fn(n, |k| {
                if k % 2 == 0 {
                    let m = k / 2;
                    let f = factorial(m);
                    Rat::new(if m % 2 == 0 { 1 } else { -1 }, f * f * (1i128 << (2 * m)))
                } else {
                    Rat::ZERO
                }
            }),
            "\\text{Bessel, first kind, order } 0",
        ),
        "lambert" => {
            // W(x) = sum_{k>=1} (-k)^{k-1} / k! x^k
            (
                "lambert",
                "W_{0}(x)".to_string(),
                Series::from_fn(n.min(12), |k| {
                    if k == 0 {
                        Rat::ZERO
                    } else {
                        let kk = k as i128;
                        Rat::new((-kk).pow(k as u32 - 1), factorial(k))
                    }
                }),
                "\\text{inverse of } x e^{x}",
            )
        }
        _ => return None,
    };

    Some(Expansion {
        id: name,
        name,
        lhs,
        series,
        note,
    })
}

/// Everything the UI is allowed to ask for, in menu order.
pub const CATALOG: &[(&str, &str)] = &[
    ("exp", "e^{x}"),
    ("sin", "\\sin x"),
    ("cos", "\\cos x"),
    ("tan", "\\tan x"),
    ("sec", "\\sec x"),
    ("sinh", "\\sinh x"),
    ("cosh", "\\cosh x"),
    ("tanh", "\\tanh x"),
    ("log1p", "\\ln(1+x)"),
    ("atan", "\\arctan x"),
    ("geom", "\\frac{1}{1-x}"),
    ("bessel0", "J_{0}(x)"),
    ("lambert", "W_{0}(x)"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tan_matches_known_coefficients() {
        let t = expand("tan", 10).unwrap().series;
        assert_eq!(t.coeff(1), Rat::ONE);
        assert_eq!(t.coeff(3), Rat::new(1, 3));
        assert_eq!(t.coeff(5), Rat::new(2, 15));
        assert_eq!(t.coeff(7), Rat::new(17, 315));
        assert_eq!(t.coeff(9), Rat::new(62, 2835));
        assert!(t.coeff(2).is_zero());
    }

    #[test]
    fn sec_matches_known_coefficients() {
        let s = expand("sec", 8).unwrap().series;
        assert_eq!(s.coeff(0), Rat::ONE);
        assert_eq!(s.coeff(2), Rat::new(1, 2));
        assert_eq!(s.coeff(4), Rat::new(5, 24));
        assert_eq!(s.coeff(6), Rat::new(61, 720));
    }

    #[test]
    fn tanh_matches_known_coefficients() {
        let t = expand("tanh", 8).unwrap().series;
        assert_eq!(t.coeff(1), Rat::ONE);
        assert_eq!(t.coeff(3), Rat::new(-1, 3));
        assert_eq!(t.coeff(5), Rat::new(2, 15));
        assert_eq!(t.coeff(7), Rat::new(-17, 315));
    }

    #[test]
    fn division_is_inverse_of_multiplication() {
        let e = expand("exp", 12).unwrap().series;
        let g = expand("geom", 12).unwrap().series;
        let back = e.mul(&g).div(&g);
        for k in 0..12 {
            assert_eq!(back.coeff(k), e.coeff(k), "coefficient {k}");
        }
    }

    #[test]
    fn latex_has_no_leading_plus() {
        let l = expand("sin", 6).unwrap().series.to_latex("x", true);
        assert!(l.starts_with('x'), "{l}");
        assert!(l.contains("\\frac{x^{3}}{6}"), "{l}");
        assert!(l.ends_with("\\cdots"));
    }

    #[test]
    fn unknown_id_is_none() {
        assert!(expand("nope", 8).is_none());
    }

    #[test]
    fn every_catalog_entry_expands() {
        for (id, _) in CATALOG {
            let e = expand(id, MAX_TERMS).expect(id);
            assert!(!e.series.to_latex("x", false).is_empty());
        }
    }
}
