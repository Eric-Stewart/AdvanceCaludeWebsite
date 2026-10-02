//! Continued-fraction expansions of famous constants, rendered as nested
//! `\cfrac` LaTeX together with their convergents.

pub struct Constant {
    pub id: &'static str,
    pub latex: &'static str,
    pub value: f64,
    pub blurb: &'static str,
}

pub const CONSTANTS: &[Constant] = &[
    Constant {
        id: "pi",
        latex: "\\pi",
        value: std::f64::consts::PI,
        blurb: "\\text{no pattern: } \\pi \\text{ is transcendental}",
    },
    Constant {
        id: "e",
        latex: "e",
        value: std::f64::consts::E,
        blurb: "[2; 1,2,1,1,4,1,1,6,\\ldots]\\text{ — a pattern Euler proved}",
    },
    Constant {
        id: "phi",
        latex: "\\varphi",
        value: 1.618_033_988_749_894_8,
        blurb: "\\text{all ones: the slowest convergence possible}",
    },
    Constant {
        id: "sqrt2",
        latex: "\\sqrt{2}",
        value: std::f64::consts::SQRT_2,
        blurb: "\\text{purely periodic: } [1; \\overline{2}]",
    },
    Constant {
        id: "sqrt3",
        latex: "\\sqrt{3}",
        value: 1.732_050_807_568_877_2,
        blurb: "\\text{period two: } [1; \\overline{1, 2}]",
    },
    Constant {
        id: "ln2",
        latex: "\\ln 2",
        value: std::f64::consts::LN_2,
        blurb: "\\sum_{k\\ge1} \\frac{(-1)^{k+1}}{k}\\text{, in disguise}",
    },
    Constant {
        id: "gamma",
        latex: "\\gamma",
        value: 0.577_215_664_901_532_9,
        blurb: "\\text{Euler–Mascheroni; irrationality unknown}",
    },
    Constant {
        id: "zeta3",
        latex: "\\zeta(3)",
        value: 1.202_056_903_159_594_3,
        blurb: "\\text{Apéry's constant, proved irrational in } 1978",
    },
];

pub fn lookup(id: &str) -> Option<&'static Constant> {
    CONSTANTS.iter().find(|c| c.id == id)
}

/// How many partial quotients we trust from an `f64` seed value.
pub const MAX_TERMS: usize = 10;

pub struct Expansion {
    pub terms: Vec<i64>,
    /// `(numerator, denominator, signed error)` for each convergent.
    pub convergents: Vec<(i128, i128, f64)>,
}

/// Classic Euclidean algorithm on the real value. We stop early once the
/// residual is too small to extract another reliable term from a float.
pub fn expand(value: f64, terms: usize) -> Expansion {
    let n = terms.clamp(1, MAX_TERMS);
    let mut quotients = Vec::new();
    let mut x = value;
    for _ in 0..n {
        let a = x.floor();
        quotients.push(a as i64);
        let frac = x - a;
        if frac < 1e-12 {
            break;
        }
        x = 1.0 / frac;
    }

    // Convergents by the standard recurrence h_k = a_k h_{k-1} + h_{k-2}.
    let mut convergents = Vec::with_capacity(quotients.len());
    let (mut hm1, mut hm2) = (1i128, 0i128);
    let (mut km1, mut km2) = (0i128, 1i128);
    for &a in &quotients {
        let h = a as i128 * hm1 + hm2;
        let k = a as i128 * km1 + km2;
        hm2 = hm1;
        hm1 = h;
        km2 = km1;
        km1 = k;
        let approx = h as f64 / k as f64;
        convergents.push((h, k, approx - value));
    }

    Expansion {
        terms: quotients,
        convergents,
    }
}

impl Expansion {
    /// Nested `\cfrac` tower: `a0 + 1/(a1 + 1/(a2 + ...))`.
    pub fn to_cfrac_latex(&self, lhs: &str) -> String {
        fn nest(terms: &[i64]) -> String {
            match terms {
                [] => "\\ddots".to_string(),
                [a] => format!("{a} + \\cfrac{{1}}{{\\ddots}}"),
                [a, rest @ ..] => format!("{a} + \\cfrac{{1}}{{{}}}", nest(rest)),
            }
        }
        match self.terms.as_slice() {
            [] => format!("{lhs} = 0"),
            [a] => format!("{lhs} = {a}"),
            [a, rest @ ..] => format!("{lhs} = {a} + \\cfrac{{1}}{{{}}}", nest(rest)),
        }
    }

    /// Compact bracket notation: `[3; 7, 15, 1, 292, \ldots]`.
    pub fn to_bracket_latex(&self) -> String {
        match self.terms.as_slice() {
            [] => "[\\,]".to_string(),
            [a] => format!("[{a}]"),
            [a, rest @ ..] => {
                let tail = rest.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
                format!("[{a}; {tail}, \\ldots]")
            }
        }
    }

    /// An aligned table of convergents and their errors.
    pub fn to_convergents_latex(&self) -> String {
        let rows: Vec<String> = self
            .convergents
            .iter()
            .enumerate()
            .map(|(i, (h, k, err))| {
                format!(
                    "p_{{{i}}}/q_{{{i}}} &= \\frac{{{h}}}{{{k}}} & \\varepsilon &= {}",
                    sci_latex(*err)
                )
            })
            .collect();
        // Explicit row spacing: with a \frac in every row the default
        // baseline skip lets consecutive rows collide.
        format!("\\begin{{aligned}} {} \\end{{aligned}}", rows.join(" \\\\[6pt] "))
    }
}

/// Render a float in LaTeX scientific notation, e.g. `-1.26 \times 10^{-7}`.
pub fn sci_latex(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return "0".to_string();
    }
    let exp = x.abs().log10().floor() as i32;
    let mant = x / 10f64.powi(exp);
    format!("{mant:.3} \\times 10^{{{exp}}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_expansion_is_classic() {
        let e = expand(std::f64::consts::PI, 5);
        assert_eq!(e.terms, vec![3, 7, 15, 1, 292]);
        // 22/7 and 355/113 must appear as convergents.
        assert_eq!((e.convergents[1].0, e.convergents[1].1), (22, 7));
        assert_eq!((e.convergents[3].0, e.convergents[3].1), (355, 113));
    }

    #[test]
    fn phi_is_all_ones() {
        let e = expand(1.618_033_988_749_894_8, 8);
        assert!(e.terms.iter().all(|&t| t == 1), "{:?}", e.terms);
        // Convergents are ratios of consecutive Fibonacci numbers.
        assert_eq!((e.convergents[6].0, e.convergents[6].1), (21, 13));
    }

    #[test]
    fn sqrt2_is_periodic() {
        let e = expand(std::f64::consts::SQRT_2, 6);
        assert_eq!(e.terms, vec![1, 2, 2, 2, 2, 2]);
    }

    #[test]
    fn e_expansion() {
        let e = expand(std::f64::consts::E, 9);
        assert_eq!(e.terms, vec![2, 1, 2, 1, 1, 4, 1, 1, 6]);
    }

    #[test]
    fn convergent_errors_shrink() {
        let e = expand(std::f64::consts::PI, 6);
        let errs: Vec<f64> = e.convergents.iter().map(|c| c.2.abs()).collect();
        for w in errs.windows(2) {
            assert!(w[1] < w[0], "errors not decreasing: {errs:?}");
        }
    }

    #[test]
    fn exact_integer_terminates() {
        let e = expand(4.0, 6);
        assert_eq!(e.terms, vec![4]);
        assert_eq!(e.to_cfrac_latex("n"), "n = 4");
    }

    #[test]
    fn latex_is_balanced() {
        let l = expand(std::f64::consts::PI, 5).to_cfrac_latex("\\pi");
        assert_eq!(l.matches('{').count(), l.matches('}').count());
        assert!(l.starts_with("\\pi = 3 + \\cfrac{1}{7"));
    }

    #[test]
    fn bracket_notation() {
        assert_eq!(
            expand(std::f64::consts::PI, 4).to_bracket_latex(),
            "[3; 7, 15, 1, \\ldots]"
        );
    }

    #[test]
    fn every_constant_expands() {
        for c in CONSTANTS {
            let e = expand(c.value, MAX_TERMS);
            assert!(!e.terms.is_empty(), "{}", c.id);
            assert!(lookup(c.id).is_some());
        }
    }
}
