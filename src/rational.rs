//! Exact rational arithmetic, used so that every Taylor coefficient we ship to
//! the browser is an honest fraction rather than a rounded float.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rat {
    num: i128,
    den: i128, // invariant: den > 0, gcd(|num|, den) == 1
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    if a == 0 {
        1
    } else {
        a
    }
}

impl Rat {
    pub const ZERO: Rat = Rat { num: 0, den: 1 };
    pub const ONE: Rat = Rat { num: 1, den: 1 };

    pub fn new(num: i128, den: i128) -> Rat {
        assert!(den != 0, "rational with zero denominator");
        let sign = if den < 0 { -1 } else { 1 };
        let g = gcd(num, den);
        Rat {
            num: sign * num / g,
            den: sign * den / g,
        }
    }

    pub fn int(n: i128) -> Rat {
        Rat { num: n, den: 1 }
    }

    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    pub fn is_negative(&self) -> bool {
        self.num < 0
    }

    pub fn abs(&self) -> Rat {
        Rat {
            num: self.num.abs(),
            den: self.den,
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// `Some(n)` when this rational is exactly the integer `n`.
    pub fn to_integer(&self) -> Option<i128> {
        if self.den == 1 {
            Some(self.num)
        } else {
            None
        }
    }

    /// LaTeX for the bare magnitude: `3`, `\frac{1}{120}`, ...
    pub fn to_latex(&self) -> String {
        if self.den == 1 {
            format!("{}", self.num)
        } else if self.num < 0 {
            format!("-\\frac{{{}}}{{{}}}", -self.num, self.den)
        } else {
            format!("\\frac{{{}}}{{{}}}", self.num, self.den)
        }
    }

    /// LaTeX for a coefficient multiplying `body`, e.g. `\frac{x^5}{120}`.
    /// Folding the variable into the numerator reads far better than
    /// `\frac{1}{120} x^5`, which is what a naive renderer would emit.
    pub fn to_latex_times(&self, body: &str) -> String {
        let mag = self.abs();
        if body.is_empty() {
            return mag.to_latex();
        }
        if mag.den == 1 {
            if mag.num == 1 {
                body.to_string()
            } else {
                format!("{}{}", mag.num, body)
            }
        } else if mag.num == 1 {
            format!("\\frac{{{}}}{{{}}}", body, mag.den)
        } else {
            format!("\\frac{{{}{}}}{{{}}}", mag.num, body, mag.den)
        }
    }
}

impl Add for Rat {
    type Output = Rat;
    fn add(self, o: Rat) -> Rat {
        let g = gcd(self.den, o.den);
        Rat::new(self.num * (o.den / g) + o.num * (self.den / g), self.den / g * o.den)
    }
}

impl Sub for Rat {
    type Output = Rat;
    fn sub(self, o: Rat) -> Rat {
        self + (-o)
    }
}

impl Neg for Rat {
    type Output = Rat;
    fn neg(self) -> Rat {
        Rat {
            num: -self.num,
            den: self.den,
        }
    }
}

impl Mul for Rat {
    type Output = Rat;
    fn mul(self, o: Rat) -> Rat {
        let g1 = gcd(self.num, o.den);
        let g2 = gcd(o.num, self.den);
        Rat::new((self.num / g1) * (o.num / g2), (self.den / g2) * (o.den / g1))
    }
}

impl Div for Rat {
    type Output = Rat;
    fn div(self, o: Rat) -> Rat {
        assert!(!o.is_zero(), "division by zero rational");
        self * Rat::new(o.den, o.num)
    }
}

impl fmt::Display for Rat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_and_normalises_sign() {
        assert_eq!(Rat::new(6, -4), Rat::new(-3, 2));
        assert_eq!(Rat::new(0, 7), Rat::ZERO);
    }

    #[test]
    fn arithmetic() {
        assert_eq!(Rat::new(1, 2) + Rat::new(1, 3), Rat::new(5, 6));
        assert_eq!(Rat::new(2, 3) * Rat::new(3, 2), Rat::ONE);
        assert_eq!(Rat::new(1, 2) - Rat::new(1, 2), Rat::ZERO);
        assert_eq!(Rat::int(7) / Rat::int(2), Rat::new(7, 2));
    }

    #[test]
    fn latex_folds_variable_into_numerator() {
        assert_eq!(Rat::new(1, 120).to_latex_times("x^5"), "\\frac{x^5}{120}");
        assert_eq!(Rat::new(-2, 15).to_latex_times("x^3"), "\\frac{2x^3}{15}");
        assert_eq!(Rat::int(1).to_latex_times("x"), "x");
        assert_eq!(Rat::int(3).to_latex_times("x^2"), "3x^2");
    }
}
