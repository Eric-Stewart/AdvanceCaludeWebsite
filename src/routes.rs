//! The JSON API. Every endpoint returns LaTeX source; the browser only ever
//! typesets what Rust computed.

use crate::cfrac;
use crate::corpus;
use crate::http::{Request, Response};
use crate::json::Json;
use crate::matrix::{IntMatrix, MAX_DIM};
use crate::palette::Palette;
use crate::series::{self, MAX_TERMS};

/// Seeds the UI is allowed to reroll through, for both `/api/matrix` and
/// `/api/palette`: the valid range is `1..=SEED_POOL`.
///
/// The live server would happily accept any `u64`, but the static export has
/// to enumerate a finite set. Bounding both modes to the same pool keeps the
/// two deployments behaviourally identical instead of letting the hosted build
/// quietly diverge from the one developed against.
pub const SEED_POOL: u64 = 128;

/// Returns `None` for anything that is not an API route, letting the caller
/// fall through to static files.
pub fn route(req: &Request) -> Option<Response> {
    if !req.path.starts_with("/api/") {
        return None;
    }
    Some(match req.path.as_str() {
        "/api/health" => Response::json(
            Json::obj([
                ("ok", Json::Bool(true)),
                ("service", Json::str("chromatex")),
                ("backend", Json::str("rust (std only)")),
            ])
            .to_string(),
        ),
        "/api/corpus" => corpus_response(req),
        "/api/drift" => drift_response(),
        "/api/catalog" => catalog_response(),
        "/api/series" => series_response(req),
        "/api/constants" => constants_response(),
        "/api/cfrac" => cfrac_response(req),
        "/api/matrix" => matrix_response(req),
        "/api/palette" => palette_response(req),
        _ => Response::json(
            Json::obj([
                ("ok", Json::Bool(false)),
                ("error", Json::str("no such endpoint")),
                ("path", Json::str(req.path.clone())),
            ])
            .to_string(),
        )
        .header("X-Api-Status", "404"),
    })
}

fn err(message: &str) -> Response {
    Response::json(
        Json::obj([("ok", Json::Bool(false)), ("error", Json::str(message))]).to_string(),
    )
}

fn corpus_response(req: &Request) -> Response {
    // `?tag=physics` narrows the wall; absent, the whole corpus comes back.
    let selected: Vec<&corpus::Entry> = match req.param("tag") {
        Some(tag) if !tag.is_empty() => corpus::by_tag(tag),
        _ => corpus::ENTRIES.iter().collect(),
    };

    let mut tags: Vec<&str> = corpus::ENTRIES.iter().map(|e| e.tag).collect();
    tags.sort_unstable();
    tags.dedup();

    let items = selected.iter().map(|e| {
        Json::obj([
            ("title", Json::str(e.title)),
            ("tag", Json::str(e.tag)),
            ("latex", Json::str(e.latex)),
        ])
    });
    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            ("count", Json::Int(selected.len() as i128)),
            ("tags", Json::arr(tags.into_iter().map(Json::str))),
            ("entries", Json::arr(items)),
        ])
        .to_string(),
    )
}

fn drift_response() -> Response {
    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            (
                "fragments",
                Json::arr(corpus::DRIFT.iter().map(|d| Json::str(*d))),
            ),
        ])
        .to_string(),
    )
}

fn catalog_response() -> Response {
    let functions = series::CATALOG.iter().map(|(id, latex)| {
        Json::obj([("id", Json::str(*id)), ("latex", Json::str(*latex))])
    });
    let constants = cfrac::CONSTANTS.iter().map(|c| {
        Json::obj([
            ("id", Json::str(c.id)),
            ("latex", Json::str(c.latex)),
            ("blurb", Json::str(c.blurb)),
        ])
    });
    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            ("functions", Json::arr(functions)),
            ("constants", Json::arr(constants)),
            ("maxTerms", Json::Int(MAX_TERMS as i128)),
            ("maxDim", Json::Int(MAX_DIM as i128)),
            (
                "seedPool",
                Json::obj([
                    ("matrix", Json::Int(SEED_POOL as i128)),
                    ("palette", Json::Int(SEED_POOL as i128)),
                ]),
            ),
        ])
        .to_string(),
    )
}

fn series_response(req: &Request) -> Response {
    let id = req.param("f").unwrap_or("exp").to_string();
    let terms: usize = req.param_or("n", 8usize);

    let Some(e) = series::expand(&id, terms) else {
        return err("unknown function; see /api/catalog");
    };

    let body = e.series.to_latex("x", true);
    let coefficients = (0..e.series.len()).map(|k| {
        Json::obj([
            ("k", Json::Int(k as i128)),
            ("latex", Json::str(e.series.coeff(k).to_latex())),
            ("value", Json::Num(e.series.coeff(k).to_f64())),
        ])
    });

    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            ("id", Json::str(e.id)),
            ("name", Json::str(e.name)),
            ("lhs", Json::str(e.lhs.clone())),
            ("note", Json::str(e.note)),
            ("terms", Json::Int(e.series.len() as i128)),
            ("display", Json::str(format!("{} = {}", e.lhs, body))),
            ("rhs", Json::str(body)),
            ("coefficients", Json::arr(coefficients)),
        ])
        .to_string(),
    )
}

fn constants_response() -> Response {
    let items = cfrac::CONSTANTS.iter().map(|c| {
        Json::obj([
            ("id", Json::str(c.id)),
            ("latex", Json::str(c.latex)),
            ("value", Json::Num(c.value)),
            ("blurb", Json::str(c.blurb)),
        ])
    });
    Response::json(
        Json::obj([("ok", Json::Bool(true)), ("constants", Json::arr(items))]).to_string(),
    )
}

fn cfrac_response(req: &Request) -> Response {
    let id = req.param("c").unwrap_or("pi");
    let n: usize = req.param_or("n", 7usize);

    let Some(c) = cfrac::lookup(id) else {
        return err("unknown constant; see /api/constants");
    };
    let e = cfrac::expand(c.value, n);
    let best = e.convergents.last();

    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            ("id", Json::str(c.id)),
            ("symbol", Json::str(c.latex)),
            ("value", Json::Num(c.value)),
            ("blurb", Json::str(c.blurb)),
            ("cfrac", Json::str(e.to_cfrac_latex(c.latex))),
            ("bracket", Json::str(e.to_bracket_latex())),
            ("convergents", Json::str(e.to_convergents_latex())),
            (
                "terms",
                Json::arr(e.terms.iter().map(|t| Json::Int(*t as i128))),
            ),
            (
                "best",
                match best {
                    Some((p, q, errv)) => Json::obj([
                        ("latex", Json::str(format!("\\frac{{{p}}}{{{q}}}"))),
                        ("p", Json::Int(*p)),
                        ("q", Json::Int(*q)),
                        ("error", Json::Num(*errv)),
                        ("errorLatex", Json::str(cfrac::sci_latex(*errv))),
                    ]),
                    None => Json::Null,
                },
            ),
        ])
        .to_string(),
    )
}

fn matrix_response(req: &Request) -> Response {
    let n: usize = req.param_or("n", 4usize);
    let seed: u64 = req.param_or("seed", 1u64);
    let m = IntMatrix::random(n, seed);
    let det = m.det();

    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            ("n", Json::Int(m.n as i128)),
            ("seed", Json::Int(seed as i128)),
            ("pmatrix", Json::str(m.pmatrix_latex())),
            (
                "determinant",
                Json::str(format!("{} = {}", m.vmatrix_latex(), det)),
            ),
            ("det", Json::Int(det)),
            ("trace", Json::Int(m.trace())),
            (
                "traceLatex",
                Json::str(format!("\\operatorname{{tr}} A = {}", m.trace())),
            ),
            ("charpoly", Json::str(m.charpoly_latex())),
            (
                "invertible",
                Json::Bool(det != 0),
            ),
        ])
        .to_string(),
    )
}

fn palette_response(req: &Request) -> Response {
    let seed: u64 = req.param_or("seed", 7u64);
    let p = Palette::generate(seed);
    let stops = p.stops.iter().map(|s| {
        Json::obj([
            ("role", Json::str(s.role.clone())),
            ("hex", Json::str(s.hex.clone())),
            ("h", Json::Num((s.h * 10.0).round() / 10.0)),
            ("s", Json::Num((s.s * 1000.0).round() / 1000.0)),
            ("l", Json::Num((s.l * 1000.0).round() / 1000.0)),
        ])
    });
    Response::json(
        Json::obj([
            ("ok", Json::Bool(true)),
            ("seed", Json::Int(seed as i128)),
            ("name", Json::str(p.name)),
            ("latex", Json::str(p.to_latex())),
            ("stops", Json::arr(stops)),
        ])
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::parse_request_line;

    fn get(target: &str) -> Response {
        let req = parse_request_line(&format!("GET {target} HTTP/1.1")).unwrap();
        route(&req).expect("api route")
    }

    fn body(target: &str) -> String {
        String::from_utf8(get(target).body).unwrap()
    }

    #[test]
    fn non_api_paths_fall_through() {
        let req = parse_request_line("GET /index.html HTTP/1.1").unwrap();
        assert!(route(&req).is_none());
    }

    #[test]
    fn health_and_catalog() {
        assert!(body("/api/health").contains("\"ok\":true"));
        let c = body("/api/catalog");
        assert!(c.contains("\"maxTerms\""));
        assert!(c.contains("lambert"));
    }

    #[test]
    fn series_endpoint_returns_latex() {
        let b = body("/api/series?f=tan&n=8");
        assert!(b.contains("\"ok\":true"));
        // Backslashes must be JSON-escaped in the payload.
        assert!(b.contains(r"\\tan x"), "{b}");
        assert!(b.contains(r"\\frac{x^{3}}{3}"), "{b}");
    }

    #[test]
    fn series_rejects_unknown_function() {
        let b = body("/api/series?f=wat");
        assert!(b.contains("\"ok\":false"));
        assert!(b.contains("unknown function"));
    }

    #[test]
    fn series_clamps_term_count() {
        let b = body("/api/series?f=exp&n=9999");
        assert!(b.contains(&format!("\"terms\":{MAX_TERMS}")), "{b}");
        // Zero terms clamps up rather than producing an empty expansion.
        assert!(body("/api/series?f=exp&n=0").contains("\"terms\":2"));
    }

    #[test]
    fn cfrac_endpoint() {
        let b = body("/api/cfrac?c=pi&n=5");
        assert!(b.contains(r"\\cfrac"), "{b}");
        // 355/113 appears among the convergents; `best` is the deepest one.
        assert!(b.contains(r"\\frac{355}{113}"), "{b}");
        assert!(b.contains("\"p\":103993"), "{b}");
        assert!(body("/api/cfrac?c=nope").contains("unknown constant"));
    }

    #[test]
    fn matrix_endpoint_clamps_and_computes() {
        let b = body("/api/matrix?n=99&seed=5");
        assert!(b.contains(&format!("\"n\":{MAX_DIM}")), "{b}");
        assert!(b.contains("vmatrix"));
        assert!(b.contains("charpoly"));
    }

    #[test]
    fn corpus_filters_by_tag() {
        let all = body("/api/corpus");
        assert!(all.contains("\"tags\":["));
        assert!(all.contains("Euler's identity"));

        let physics = body("/api/corpus?tag=physics");
        assert!(physics.contains("Maxwell"), "{physics}");
        assert!(!physics.contains("Basel problem"), "{physics}");

        // An unknown tag yields an empty set, not an error.
        let none = body("/api/corpus?tag=zzz");
        assert!(none.contains("\"count\":0"), "{none}");
        assert!(none.contains("\"ok\":true"));
    }

    #[test]
    fn palette_endpoint() {
        let b = body("/api/palette?seed=3");
        assert!(b.contains("\"role\":\"void\""));
        assert!(b.contains('#'));
    }

    #[test]
    fn unknown_api_path_is_reported() {
        let r = get("/api/nope");
        assert!(r.extra_headers.iter().any(|(k, v)| k == "X-Api-Status" && v == "404"));
    }

    #[test]
    fn every_catalog_entry_serves() {
        for (id, _) in series::CATALOG {
            assert!(body(&format!("/api/series?f={id}")).contains("\"ok\":true"), "{id}");
        }
        for c in cfrac::CONSTANTS {
            assert!(body(&format!("/api/cfrac?c={}", c.id)).contains("\"ok\":true"), "{}", c.id);
        }
    }
}
