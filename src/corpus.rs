//! The curated LaTeX corpus. These drive the drifting background equations,
//! the marquee, and the scroll-revealed theorem cards.

pub struct Entry {
    pub title: &'static str,
    pub latex: &'static str,
    pub tag: &'static str,
}

pub const ENTRIES: &[Entry] = &[
    Entry {
        title: "Euler's identity",
        tag: "analysis",
        latex: r"e^{i\pi} + 1 = 0",
    },
    Entry {
        title: "Gaussian integral",
        tag: "analysis",
        latex: r"\int_{-\infty}^{\infty} e^{-x^{2}}\,dx = \sqrt{\pi}",
    },
    Entry {
        title: "Basel problem",
        tag: "number theory",
        latex: r"\sum_{n=1}^{\infty} \frac{1}{n^{2}} = \frac{\pi^{2}}{6}",
    },
    Entry {
        title: "Euler product",
        tag: "number theory",
        latex: r"\zeta(s) = \prod_{p\ \mathrm{prime}} \frac{1}{1 - p^{-s}}",
    },
    Entry {
        title: "Functional equation of zeta",
        tag: "number theory",
        latex: r"\zeta(s) = 2^{s}\pi^{s-1}\sin\!\left(\frac{\pi s}{2}\right)\Gamma(1-s)\,\zeta(1-s)",
    },
    Entry {
        title: "Ramanujan's series",
        tag: "number theory",
        latex: r"\frac{1}{\pi} = \frac{2\sqrt{2}}{9801}\sum_{k=0}^{\infty}\frac{(4k)!\,(1103 + 26390k)}{(k!)^{4}\,396^{4k}}",
    },
    Entry {
        title: "Stokes' theorem",
        tag: "geometry",
        latex: r"\int_{\partial\Omega} \omega = \int_{\Omega} d\omega",
    },
    Entry {
        title: "Cauchy integral formula",
        tag: "analysis",
        latex: r"f(a) = \frac{1}{2\pi i}\oint_{\gamma} \frac{f(z)}{z-a}\,dz",
    },
    Entry {
        title: "Maxwell's equations",
        tag: "physics",
        latex: r"\begin{aligned} \nabla\cdot\mathbf{E} &= \frac{\rho}{\varepsilon_0} & \nabla\times\mathbf{E} &= -\frac{\partial\mathbf{B}}{\partial t} \\ \nabla\cdot\mathbf{B} &= 0 & \nabla\times\mathbf{B} &= \mu_0\mathbf{J} + \mu_0\varepsilon_0\frac{\partial\mathbf{E}}{\partial t} \end{aligned}",
    },
    Entry {
        title: "Schrödinger equation",
        tag: "physics",
        latex: r"i\hbar\frac{\partial}{\partial t}\Psi(\mathbf{r},t) = \left[-\frac{\hbar^{2}}{2m}\nabla^{2} + V(\mathbf{r},t)\right]\Psi(\mathbf{r},t)",
    },
    Entry {
        title: "Einstein field equations",
        tag: "physics",
        latex: r"R_{\mu\nu} - \tfrac{1}{2}R\,g_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^{4}}T_{\mu\nu}",
    },
    Entry {
        title: "Navier–Stokes",
        tag: "physics",
        latex: r"\rho\left(\frac{\partial\mathbf{u}}{\partial t} + \mathbf{u}\cdot\nabla\mathbf{u}\right) = -\nabla p + \mu\nabla^{2}\mathbf{u} + \mathbf{f}",
    },
    Entry {
        title: "Fourier transform",
        tag: "analysis",
        latex: r"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x\xi}\,dx",
    },
    Entry {
        title: "Gamma reflection",
        tag: "analysis",
        latex: r"\Gamma(z)\Gamma(1-z) = \frac{\pi}{\sin \pi z}",
    },
    Entry {
        title: "Stirling's approximation",
        tag: "asymptotics",
        latex: r"n! \sim \sqrt{2\pi n}\left(\frac{n}{e}\right)^{n}",
    },
    Entry {
        title: "Cauchy–Schwarz",
        tag: "algebra",
        latex: r"\left|\langle u, v\rangle\right|^{2} \le \langle u,u\rangle\cdot\langle v,v\rangle",
    },
    Entry {
        title: "Bayes' theorem",
        tag: "probability",
        latex: r"P(A\mid B) = \frac{P(B\mid A)\,P(A)}{P(B)}",
    },
    Entry {
        title: "Jacobi theta",
        tag: "number theory",
        latex: r"\vartheta(z;\tau) = \sum_{n=-\infty}^{\infty} e^{\pi i n^{2}\tau + 2\pi i n z}",
    },
    Entry {
        title: "Gauss–Bonnet",
        tag: "geometry",
        latex: r"\int_{M} K\,dA + \oint_{\partial M} k_{g}\,ds = 2\pi\chi(M)",
    },
    Entry {
        title: "Noether's theorem",
        tag: "physics",
        latex: r"\frac{d}{dt}\left(\frac{\partial L}{\partial \dot{q}}\delta q - L\,\delta t\right) = 0",
    },
    Entry {
        title: "Binomial theorem",
        tag: "algebra",
        latex: r"(x+y)^{n} = \sum_{k=0}^{n}\binom{n}{k}x^{k}y^{n-k}",
    },
    Entry {
        title: "Mandelbrot iteration",
        tag: "dynamics",
        latex: r"z_{n+1} = z_{n}^{2} + c,\qquad z_{0} = 0",
    },
    Entry {
        title: "Fundamental theorem of calculus",
        tag: "analysis",
        latex: r"\frac{d}{dx}\int_{a}^{x} f(t)\,dt = f(x)",
    },
    Entry {
        title: "Gauss sum",
        tag: "number theory",
        latex: r"\sum_{n=0}^{N-1} e^{2\pi i n^{2}/N} = \frac{1+i}{2}\left(1 + i^{-N}\right)\sqrt{N}",
    },
];

/// Short one-liners for the drifting background layer — anything with a
/// multi-line environment would be unreadable at that size.
pub const DRIFT: &[&str] = &[
    r"e^{i\pi}+1=0",
    r"\nabla\times\mathbf{B}=\mu_0\mathbf{J}",
    r"\zeta(2)=\frac{\pi^2}{6}",
    r"\oint_{\gamma}\frac{dz}{z}=2\pi i",
    r"\partial_\mu T^{\mu\nu}=0",
    r"\hat{H}\psi=E\psi",
    r"\aleph_0 < 2^{\aleph_0}",
    r"\sum\frac{1}{p}=\infty",
    r"\det(e^{A})=e^{\mathrm{tr}A}",
    r"dx\wedge dy",
    r"\pi_1(S^1)\cong\mathbb{Z}",
    r"\lim_{n\to\infty}\left(1+\tfrac1n\right)^n=e",
    r"\mathcal{F}\{f*g\}=\hat f\cdot\hat g",
    r"\Gamma(\tfrac12)=\sqrt\pi",
    r"\|x\|_2\le\|x\|_1",
    r"\mathbb{E}[X]=\int x\,dP",
    r"H^1(X,\mathcal{O}_X)",
    r"\sin^2\theta+\cos^2\theta=1",
    r"i\hbar\partial_t\Psi",
    r"\varphi=\frac{1+\sqrt5}{2}",
    r"\nabla^2\phi=0",
    r"x^n+y^n\ne z^n",
    r"\mathrm{SU}(3)\times\mathrm{SU}(2)\times\mathrm{U}(1)",
    r"\binom{2n}{n}\sim\frac{4^n}{\sqrt{\pi n}}",
];

pub fn by_tag(tag: &str) -> Vec<&'static Entry> {
    ENTRIES.iter().filter(|e| e.tag == tag).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced(s: &str) -> bool {
        let mut depth = 0i32;
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    chars.next(); // escaped char, including \{ and \}
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        depth == 0
    }

    #[test]
    fn all_latex_has_balanced_braces() {
        for e in ENTRIES {
            assert!(balanced(e.latex), "unbalanced: {}", e.title);
        }
        for d in DRIFT {
            assert!(balanced(d), "unbalanced drift: {d}");
        }
    }

    #[test]
    fn environments_are_closed() {
        for e in ENTRIES {
            let begins = e.latex.matches("\\begin{").count();
            let ends = e.latex.matches("\\end{").count();
            assert_eq!(begins, ends, "{}", e.title);
        }
    }

    #[test]
    fn corpus_is_populated_and_titles_unique() {
        assert!(ENTRIES.len() >= 20);
        assert!(DRIFT.len() >= 20);
        let mut titles: Vec<&str> = ENTRIES.iter().map(|e| e.title).collect();
        titles.sort_unstable();
        let before = titles.len();
        titles.dedup();
        assert_eq!(before, titles.len(), "duplicate titles");
    }

    #[test]
    fn tag_filter_works() {
        assert!(!by_tag("physics").is_empty());
        assert!(by_tag("nonexistent").is_empty());
    }
}
