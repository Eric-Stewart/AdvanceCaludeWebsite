//! Small integer matrices with exact determinants and characteristic
//! polynomials, rendered as LaTeX `vmatrix`/`pmatrix` environments.

use crate::rational::Rat;

/// Deterministic generator so a given seed always produces the same matrix —
/// the browser can deep-link a seed and get the identical display.
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Lcg {
        // Avoid the fixed point at 0 and decorrelate nearby seeds.
        Lcg(seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u32() as u64 % span) as i64
    }
}

#[derive(Clone)]
pub struct IntMatrix {
    pub n: usize,
    data: Vec<i128>,
}

/// Largest dimension we expose. Bareiss elimination on 6x6 entries bounded by
/// 9 stays far inside `i128`; the LaTeX also stops being readable past this.
pub const MAX_DIM: usize = 6;

impl IntMatrix {
    pub fn random(n: usize, seed: u64) -> IntMatrix {
        let n = n.clamp(2, MAX_DIM);
        let mut rng = Lcg::new(seed);
        let data = (0..n * n).map(|_| rng.range(-9, 9) as i128).collect();
        IntMatrix { n, data }
    }

    pub fn at(&self, r: usize, c: usize) -> i128 {
        self.data[r * self.n + c]
    }

    pub fn trace(&self) -> i128 {
        (0..self.n).map(|i| self.at(i, i)).sum()
    }

    /// Fraction-free Bareiss elimination: stays in the integers the whole way,
    /// so the answer is exact with no rational bookkeeping.
    pub fn det(&self) -> i128 {
        let n = self.n;
        let mut m = self.data.clone();
        let mut sign = 1i128;
        let mut prev = 1i128;

        for k in 0..n.saturating_sub(1) {
            if m[k * n + k] == 0 {
                // Find a row below with a non-zero entry in this column.
                match (k + 1..n).find(|&r| m[r * n + k] != 0) {
                    Some(r) => {
                        for c in 0..n {
                            m.swap(k * n + c, r * n + c);
                        }
                        sign = -sign;
                    }
                    None => return 0, // singular: whole column is zero
                }
            }
            for i in k + 1..n {
                for j in k + 1..n {
                    let num = m[i * n + j] * m[k * n + k] - m[i * n + k] * m[k * n + j];
                    m[i * n + j] = num / prev; // exact by the Bareiss identity
                }
            }
            prev = m[k * n + k];
        }
        sign * m[(n - 1) * n + (n - 1)]
    }

    /// Characteristic polynomial coefficients of `det(xI - A)`, highest degree
    /// first, via Faddeev–LeVerrier. Integer matrices give integer coefficients.
    pub fn charpoly(&self) -> Vec<i128> {
        let n = self.n;
        let mut coeffs = vec![1i128];
        // M_1 = I, c_1 = -tr(A); M_{k+1} = A M_k + c_k I
        let mut m: Vec<Rat> = (0..n * n)
            .map(|i| if i % n == i / n { Rat::ONE } else { Rat::ZERO })
            .collect();

        for k in 1..=n {
            if k > 1 {
                // m <- A * m + c_{k-1} I
                let mut next = vec![Rat::ZERO; n * n];
                for i in 0..n {
                    for j in 0..n {
                        let mut acc = Rat::ZERO;
                        for t in 0..n {
                            acc = acc + Rat::int(self.at(i, t)) * m[t * n + j];
                        }
                        next[i * n + j] = acc;
                    }
                }
                let c = Rat::int(coeffs[k - 1]);
                for i in 0..n {
                    next[i * n + i] = next[i * n + i] + c;
                }
                m = next;
            }
            // c_k = -tr(A M_k) / k
            let mut tr = Rat::ZERO;
            for i in 0..n {
                for t in 0..n {
                    tr = tr + Rat::int(self.at(i, t)) * m[t * n + i];
                }
            }
            let c = -tr / Rat::int(k as i128);
            // Faddeev–LeVerrier on an integer matrix yields integer coefficients;
            // the division by k always comes out even.
            coeffs.push(c.to_integer().expect("characteristic polynomial coefficient is integral"));
        }
        coeffs
    }

    fn body_latex(&self) -> String {
        (0..self.n)
            .map(|r| {
                (0..self.n)
                    .map(|c| self.at(r, c).to_string())
                    .collect::<Vec<_>>()
                    .join(" & ")
            })
            .collect::<Vec<_>>()
            .join(" \\\\ ")
    }

    pub fn pmatrix_latex(&self) -> String {
        format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", self.body_latex())
    }

    pub fn vmatrix_latex(&self) -> String {
        format!("\\begin{{vmatrix}} {} \\end{{vmatrix}}", self.body_latex())
    }

    /// `det(xI - A) = x^n + ... ` as display LaTeX.
    pub fn charpoly_latex(&self) -> String {
        let coeffs = self.charpoly();
        let n = self.n;
        let mut out = String::new();
        for (k, c) in coeffs.iter().enumerate() {
            let deg = n - k;
            if *c == 0 {
                continue;
            }
            let body = match deg {
                0 => String::new(),
                1 => "\\lambda".to_string(),
                d => format!("\\lambda^{{{d}}}"),
            };
            let mag = c.abs();
            if out.is_empty() {
                if *c < 0 {
                    out.push('-');
                }
            } else {
                out.push_str(if *c < 0 { " - " } else { " + " });
            }
            if mag == 1 && deg > 0 {
                out.push_str(&body);
            } else {
                out.push_str(&format!("{mag}{body}"));
            }
        }
        if out.is_empty() {
            out.push('0');
        }
        format!("\\det(\\lambda I - A) = {out}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_rows(rows: &[&[i128]]) -> IntMatrix {
        IntMatrix {
            n: rows.len(),
            data: rows.iter().flat_map(|r| r.iter().copied()).collect(),
        }
    }

    #[test]
    fn det_of_known_matrices() {
        assert_eq!(from_rows(&[&[1, 2], &[3, 4]]).det(), -2);
        assert_eq!(from_rows(&[&[2, 0, 1], &[1, 3, 2], &[1, 1, 2]]).det(), 6);
        // Cofactor expansion by hand: 2(3·1−2·1) − 0 + 1(1·1−3·1) = 0.
        assert_eq!(from_rows(&[&[2, 0, 1], &[1, 3, 2], &[1, 1, 1]]).det(), 0);
        // Identity and a singular matrix.
        assert_eq!(from_rows(&[&[1, 0], &[0, 1]]).det(), 1);
        assert_eq!(from_rows(&[&[1, 2], &[2, 4]]).det(), 0);
    }

    #[test]
    fn det_handles_zero_pivot() {
        // Leading entry is zero, forcing a row swap.
        assert_eq!(from_rows(&[&[0, 1], &[1, 0]]).det(), -1);
        assert_eq!(from_rows(&[&[0, 0, 1], &[0, 1, 0], &[1, 0, 0]]).det(), -1);
    }

    #[test]
    fn charpoly_constant_term_matches_determinant() {
        for seed in 0..40u64 {
            for n in 2..=MAX_DIM {
                let m = IntMatrix::random(n, seed);
                let c = m.charpoly();
                // det(A) = (-1)^n * constant term of det(xI - A)
                let sign = if n % 2 == 0 { 1 } else { -1 };
                assert_eq!(sign * c[n], m.det(), "seed {seed} dim {n}");
                // Coefficient of x^{n-1} is -tr(A).
                assert_eq!(c[1], -m.trace(), "trace, seed {seed} dim {n}");
                assert_eq!(c[0], 1);
            }
        }
    }

    #[test]
    fn seed_is_deterministic() {
        let a = IntMatrix::random(4, 7).pmatrix_latex();
        let b = IntMatrix::random(4, 7).pmatrix_latex();
        assert_eq!(a, b);
        assert_ne!(a, IntMatrix::random(4, 8).pmatrix_latex());
    }

    #[test]
    fn latex_shape() {
        let m = from_rows(&[&[1, 2], &[3, 4]]);
        assert_eq!(m.pmatrix_latex(), "\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}");
        assert_eq!(m.charpoly_latex(), "\\det(\\lambda I - A) = \\lambda^{2} - 5\\lambda - 2");
    }
}
