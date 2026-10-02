//! Server-generated colour schemes.
//!
//! The brief was "crazy", so the generator deliberately picks clashing hues at
//! full chroma rather than the safe analogous ramp a design system would want.
//! Hue steps use the golden angle (137.508 degrees) so successive colours land
//! as far apart on the wheel as possible without ever repeating.

use crate::matrix::Lcg;

const GOLDEN_ANGLE: f64 = 137.507_764_05;

pub struct Palette {
    pub seed: u64,
    pub name: &'static str,
    /// Role-ordered: ink, void, and then six accents.
    pub stops: Vec<Stop>,
}

pub struct Stop {
    pub role: String,
    pub hex: String,
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

const NAMES: &[&str] = &[
    "Ultraviolet Catastrophe",
    "Riemann Fever Dream",
    "Nonabelian Neon",
    "Cauchy Overdrive",
    "Hilbert Space Rave",
    "Tensor Meltdown",
    "Galois Acid",
    "Spectral Sequence",
    "Borel Hypercolour",
    "Lebesgue Maximalism",
    "Zorn's Lemonade",
    "Banach Tarski Split",
];

/// Roles consumed as CSS custom properties by the front end.
const ROLES: &[&str] = &["a", "b", "c", "d", "e", "f"];

fn hsl_to_hex(h: f64, s: f64, l: f64) -> String {
    let h = h.rem_euclid(360.0);
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to_byte = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", to_byte(r1), to_byte(g1), to_byte(b1))
}

impl Palette {
    pub fn generate(seed: u64) -> Palette {
        let mut rng = Lcg::new(seed);
        let base = rng.range(0, 359) as f64;
        let name = NAMES[(rng.next_u32() as usize) % NAMES.len()];

        let mut stops = Vec::with_capacity(ROLES.len() + 2);

        // Near-black ground and near-white ink, both tinted by the base hue so
        // the whole page reads as one scheme instead of accents on grey.
        stops.push(Stop {
            role: "void".to_string(),
            hex: hsl_to_hex(base, 0.72, 0.045),
            h: base,
            s: 0.72,
            l: 0.045,
        });
        stops.push(Stop {
            role: "ink".to_string(),
            hex: hsl_to_hex(base + 180.0, 0.95, 0.94),
            h: base + 180.0,
            s: 0.95,
            l: 0.94,
        });

        for (i, role) in ROLES.iter().enumerate() {
            let h = base + GOLDEN_ANGLE * (i as f64 + 1.0);
            // Alternate light and mid accents so adjacent swatches always
            // differ in luminance as well as hue.
            let l = if i % 2 == 0 { 0.60 } else { 0.47 };
            let s = 0.92 + 0.08 * ((i % 3) as f64 / 2.0);
            stops.push(Stop {
                role: role.to_string(),
                hex: hsl_to_hex(h, s, l),
                h: h.rem_euclid(360.0),
                s,
                l,
            });
        }

        Palette { seed, name, stops }
    }

    /// LaTeX summary of the generated scheme, so even the colours arrive as math.
    pub fn to_latex(&self) -> String {
        let rows: Vec<String> = self
            .stops
            .iter()
            .map(|s| {
                // Round explicitly rather than leaning on `{:.0}`: that
                // rounds half-to-even, so 4.5% would print as 4 here while
                // the browser's Math.round shows 5 on the same swatch.
                format!(
                    "\\texttt{{{}}} &\\mapsto& \\mathrm{{hsl}}({}^\\circ, {}\\%, {}\\%)",
                    s.role,
                    s.h.round() as i64,
                    (s.s * 100.0).round() as i64,
                    (s.l * 100.0).round() as i64
                )
            })
            .collect();
        format!(
            "\\Phi_{{{}}} : \\quad \\begin{{array}}{{rcl}} {} \\end{{array}}",
            self.seed,
            rows.join(" \\\\ ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_conversion_endpoints() {
        assert_eq!(hsl_to_hex(0.0, 1.0, 0.5), "#ff0000");
        assert_eq!(hsl_to_hex(120.0, 1.0, 0.5), "#00ff00");
        assert_eq!(hsl_to_hex(240.0, 1.0, 0.5), "#0000ff");
        assert_eq!(hsl_to_hex(0.0, 0.0, 1.0), "#ffffff");
        assert_eq!(hsl_to_hex(0.0, 0.0, 0.0), "#000000");
        // Wrapping hue is equivalent.
        assert_eq!(hsl_to_hex(360.0, 1.0, 0.5), hsl_to_hex(0.0, 1.0, 0.5));
        assert_eq!(hsl_to_hex(-120.0, 1.0, 0.5), hsl_to_hex(240.0, 1.0, 0.5));
    }

    #[test]
    fn palette_shape_and_determinism() {
        let p = Palette::generate(42);
        assert_eq!(p.stops.len(), 8);
        assert_eq!(p.stops[0].role, "void");
        assert_eq!(p.stops[1].role, "ink");
        for s in &p.stops {
            assert_eq!(s.hex.len(), 7, "{}", s.hex);
            assert!(s.hex.starts_with('#'));
            assert!(s.hex[1..].chars().all(|c| c.is_ascii_hexdigit()));
        }
        let q = Palette::generate(42);
        assert_eq!(p.stops[3].hex, q.stops[3].hex);
    }

    #[test]
    fn accent_hues_are_well_separated() {
        let p = Palette::generate(9);
        let hues: Vec<f64> = p.stops[2..].iter().map(|s| s.h).collect();
        for (i, a) in hues.iter().enumerate() {
            for b in &hues[i + 1..] {
                let d = (a - b).abs().min(360.0 - (a - b).abs());
                assert!(d > 20.0, "hues {a} and {b} too close");
            }
        }
    }

    #[test]
    fn different_seeds_give_different_schemes() {
        let a = Palette::generate(1);
        let b = Palette::generate(2);
        assert_ne!(a.stops[2].hex, b.stops[2].hex);
    }

    #[test]
    fn latex_is_balanced() {
        let l = Palette::generate(3).to_latex();
        assert_eq!(l.matches('{').count(), l.matches('}').count());
    }
}
