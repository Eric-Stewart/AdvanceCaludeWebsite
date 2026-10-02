//! Static-site export.
//!
//! GitHub Pages cannot run the backend, so this walks the full (finite)
//! parameter space of the API and writes each response to a file. It does
//! that by building real `Request` values and calling [`routes::route`] — the
//! same code path the live server uses — so an exported file is byte-identical
//! to what `GET`ting that URL would have returned. There is no second
//! implementation of the maths to drift out of sync.
//!
//! Files land under `data/` rather than `api/` so that the exported tree can
//! still be served by this very binary (whose router owns `/api/*`) during
//! testing.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::cfrac;
use crate::corpus;
use crate::http::parse_request_line;
use crate::matrix::MAX_DIM;
use crate::routes::{self, SEED_POOL};
use crate::series::{self, MAX_TERMS};

/// Slugify a corpus tag for use in a filename (`number theory` → `number-theory`).
/// Mirrored by `slug()` in `static/app.js`.
pub fn slug(tag: &str) -> String {
    tag.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// Every (logical API URL, exported file path) pair the site can request.
/// Mirrored by `staticPathFor()` in `static/app.js`.
pub fn manifest() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![
        ("/api/health".into(), "data/health.json".into()),
        ("/api/catalog".into(), "data/catalog.json".into()),
        ("/api/constants".into(), "data/constants.json".into()),
        ("/api/drift".into(), "data/drift.json".into()),
        ("/api/corpus".into(), "data/corpus.json".into()),
    ];

    let mut tags: Vec<&str> = corpus::ENTRIES.iter().map(|e| e.tag).collect();
    tags.sort_unstable();
    tags.dedup();
    for tag in tags {
        out.push((
            format!("/api/corpus?tag={tag}"),
            format!("data/corpus/{}.json", slug(tag)),
        ));
    }

    for (id, _) in series::CATALOG {
        for n in 2..=MAX_TERMS {
            out.push((
                format!("/api/series?f={id}&n={n}"),
                format!("data/series/{id}-{n}.json"),
            ));
        }
    }

    for c in cfrac::CONSTANTS {
        for n in 2..=cfrac::MAX_TERMS {
            out.push((
                format!("/api/cfrac?c={}&n={n}", c.id),
                format!("data/cfrac/{}-{n}.json", c.id),
            ));
        }
    }

    for n in 2..=MAX_DIM {
        for seed in 1..=SEED_POOL {
            out.push((
                format!("/api/matrix?n={n}&seed={seed}"),
                format!("data/matrix/{n}-{seed}.json"),
            ));
        }
    }

    for seed in 1..=SEED_POOL {
        out.push((
            format!("/api/palette?seed={seed}"),
            format!("data/palette/{seed}.json"),
        ));
    }

    out
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

/// Recursively copy `src` into `dst`, creating directories as needed.
fn copy_tree(src: &Path, dst: &Path) -> io::Result<usize> {
    let mut n = 0;
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            n += copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
            n += 1;
        }
    }
    Ok(n)
}

/// Build the complete deployable tree at `out`: the contents of `static_root`
/// plus the exported `data/` responses. Returns (static files, api files).
pub fn emit(out: &Path, static_root: &Path) -> io::Result<(usize, usize)> {
    if out.exists() {
        // Refuse to merge into an existing tree: a stale file from a previous
        // export with different constants would silently ship.
        fs::remove_dir_all(out)?;
    }
    let copied = copy_tree(static_root, out)?;

    let mut written = 0;
    for (url, rel) in manifest() {
        let line = format!("GET {url} HTTP/1.1");
        let req = parse_request_line(&line)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("bad url: {url}")))?;
        let response = routes::route(&req).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("not an api route: {url}"))
        })?;
        if response.status != 200 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{url} returned {}", response.status),
            ));
        }
        // An `{"ok":false}` body means a handler rejected its own generated
        // parameters — a bug in the manifest, not something to publish.
        if response.body.starts_with(br#"{"ok":false"#) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{url} returned an error payload"),
            ));
        }
        write_file(&PathBuf::from(out).join(&rel), &response.body)?;
        written += 1;
    }

    // A marker the front end can probe to learn it is running without a backend.
    write_file(
        &PathBuf::from(out).join("data/static.json"),
        br#"{"ok":true,"mode":"static","generator":"chromatex --emit-static"}"#,
    )?;

    Ok((copied, written + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_match_the_js_rule() {
        assert_eq!(slug("number theory"), "number-theory");
        assert_eq!(slug("physics"), "physics");
        assert_eq!(slug("Analysis"), "analysis");
    }

    #[test]
    fn manifest_covers_every_parameter_combination() {
        let m = manifest();
        let expected = 5
            + 8 // distinct corpus tags
            + series::CATALOG.len() * (MAX_TERMS - 1)
            + cfrac::CONSTANTS.len() * (cfrac::MAX_TERMS - 1)
            + (MAX_DIM - 1) * SEED_POOL as usize
            + SEED_POOL as usize;
        assert_eq!(m.len(), expected, "manifest size");
    }

    #[test]
    fn manifest_paths_are_unique_and_safe() {
        let mut paths: Vec<&str> = m_paths();
        paths.sort_unstable();
        let before = paths.len();
        paths.dedup();
        assert_eq!(before, paths.len(), "duplicate export paths");
        for p in m_paths() {
            assert!(p.starts_with("data/"), "{p}");
            assert!(!p.contains(".."), "{p}");
            assert!(p.ends_with(".json"), "{p}");
        }
    }

    fn m_paths() -> Vec<&'static str> {
        // Leak is fine in a test; keeps the borrow simple.
        Box::leak(manifest().into_boxed_slice())
            .iter()
            .map(|(_, p)| p.as_str())
            .collect()
    }

    #[test]
    fn every_manifest_url_is_a_live_route_returning_ok() {
        for (url, _) in manifest() {
            let req = parse_request_line(&format!("GET {url} HTTP/1.1")).expect(&url);
            let r = routes::route(&req).expect(&url);
            assert_eq!(r.status, 200, "{url}");
            assert!(
                r.body.starts_with(br#"{"ok":true"#),
                "{url} -> {}",
                String::from_utf8_lossy(&r.body[..40.min(r.body.len())])
            );
        }
    }
}
