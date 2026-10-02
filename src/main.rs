//! chromatex — a LaTeX-saturated, aggressively animated single-page site
//! served by a dependency-free Rust backend.
//!
//!     cargo run --release -- --port 8080
//!
//! Every formula on the page is LaTeX; every non-static formula is computed in
//! Rust (exact rational series, Bareiss determinants, continued fractions) and
//! shipped to the browser as LaTeX source for KaTeX to typeset.

mod cfrac;
mod corpus;
mod export;
mod http;
mod json;
mod matrix;
mod palette;
mod rational;
mod routes;
mod series;

use std::path::PathBuf;

const DEFAULT_PORT: u16 = 8080;

struct Config {
    addr: String,
    static_root: PathBuf,
    workers: usize,
    /// When set, build the static site into this directory and exit instead
    /// of serving.
    emit_static: Option<PathBuf>,
}

fn parse_args() -> Result<Config, String> {
    let mut port = DEFAULT_PORT;
    let mut host = "127.0.0.1".to_string();
    let mut static_root: Option<PathBuf> = None;
    let mut emit_static: Option<PathBuf> = None;
    let mut workers = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8)
        .clamp(4, 32);

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let next = |i: usize| -> Result<String, String> {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("{} needs a value", args[i]))
        };
        match args[i].as_str() {
            "--port" | "-p" => {
                port = next(i)?.parse().map_err(|_| "--port must be a number".to_string())?;
                i += 2;
            }
            "--host" => {
                host = next(i)?;
                i += 2;
            }
            "--static" => {
                static_root = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--emit-static" => {
                emit_static = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--workers" => {
                workers = next(i)?
                    .parse::<usize>()
                    .map_err(|_| "--workers must be a number".to_string())?
                    .clamp(1, 256);
                i += 2;
            }
            "--help" | "-h" => {
                println!(
                    "chromatex\n\n\
                     USAGE:\n  \
                       chromatex [--host H] [--port P] [--static DIR] [--workers N]\n  \
                       chromatex --emit-static DIR     build the deployable static site and exit\n\n\
                     Defaults: host 127.0.0.1, port {DEFAULT_PORT}, static ./static"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    // Default to ./static relative to the crate, so `cargo run` works from
    // anywhere in the tree rather than only from the crate root.
    let static_root = static_root.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static"));

    Ok(Config {
        addr: format!("{host}:{port}"),
        static_root,
        workers,
        emit_static,
    })
}

fn main() {
    let config = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chromatex: {e}");
            eprintln!("try --help");
            std::process::exit(2);
        }
    };

    if !config.static_root.join("index.html").exists() {
        eprintln!(
            "chromatex: no index.html under {} — pass --static DIR",
            config.static_root.display()
        );
        std::process::exit(1);
    }

    println!("\n  \\chromatex");

    if let Some(out) = config.emit_static {
        match export::emit(&out, &config.static_root) {
            Ok((copied, written)) => {
                println!("  exported {copied} static files + {written} api responses");
                println!("  → {}", out.display());
                return;
            }
            Err(e) => {
                eprintln!("chromatex: export to {} failed: {e}", out.display());
                std::process::exit(1);
            }
        }
    }

    if let Err(e) = http::serve(&config.addr, config.static_root, config.workers, routes::route) {
        eprintln!("chromatex: could not serve on {}: {e}", config.addr);
        std::process::exit(1);
    }
}
