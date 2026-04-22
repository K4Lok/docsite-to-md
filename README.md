# docsite-to-md

`docsite-to-md` is a Rust CLI and library for exporting documentation sites into Markdown.

It is designed for docs archiving, indexing, offline reading, and LLM preparation with a stronger focus on:

- extractor-based site detection
- GitBook support, including modern Next.js-powered GitBook sites
- first-class support for Docusaurus, MkDocs Material, VitePress, and Nextra
- generic HTML fallback extraction for non-GitBook docs
- mirror-tree Markdown output
- structured manifests and resumable exports

## Features

- `detect <url>`: detect the site framework and extraction strategy
- `crawl <url>`: discover in-scope pages and emit a crawl manifest
- `export <url>`: export one Markdown file per page plus `manifest.json` and `links.txt`
- `bundle <url>`: produce a merged Markdown bundle
- retry/backoff, configurable concurrency, rate limiting, scope filtering, and duplicate detection
- optional browser fallback hook behind the `browser` feature

## Supported Frameworks

- `GitBookModern`
- `GitBookClassic`
- `Docusaurus`
- `MkDocsMaterial`
- `VitePress`
- `Nextra`
- `GenericDocsFallback`

Browser fallback remains optional and disabled by default. The exporter stays HTTP-first for normal installs, while the manifest records whether browser fallback was used for a page when it is enabled.

Live Markdown quality still varies by site. The tool is strongest on navigation discovery and mirror-tree export structure today; framework-specific cleanup continues to improve as we benchmark more public docs sites.

## Install

```bash
cargo install --path .
```

## Usage

```bash
docsite-to-md detect https://apidoc.mycactus.com
docsite-to-md crawl https://apidoc.mycactus.com --scope-prefix /api-reference
docsite-to-md export https://apidoc.mycactus.com --output-dir ./mycactus-docs
docsite-to-md bundle https://apidoc.mycactus.com --output ./mycactus.md
```

## Library

```rust
use docsite_to_md::{detect_site, crawl_site, export_site, CrawlOptions, ExportOptions, BrowserOptions};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let profile = detect_site("https://apidoc.mycactus.com").await?;
let manifest = crawl_site("https://apidoc.mycactus.com", CrawlOptions::default()).await?;
let result = export_site(
    "https://apidoc.mycactus.com",
    ExportOptions {
        output_dir: "output".into(),
        crawl: CrawlOptions::default(),
        resume: false,
        bundle_output: None,
        browser: BrowserOptions::default(),
    },
).await?;
# Ok(())
# }
```

## Notes

- Browser rendering is optional and disabled by default.
- GitBook normalization is conservative and preserves source meaning where possible.
- Support for additional frameworks can be added via new extractors.

## Live Benchmark

The repo includes a checked-in live benchmark target list for supported public docs sites in `tests/live_targets.json`.

Useful maintainer commands:

```bash
# Run the ignored live smoke checks for curated public targets
cargo test --test live_benchmark live_smoke_ -- --ignored --nocapture

# Run the full live benchmark report with smoke + quality grading
cargo test --test live_benchmark live_benchmark_report -- --ignored --nocapture
```

What to inspect in the report:

- detected framework and whether smoke validation passed
- whether at least one representative page exported correctly
- per-page quality grade: `pass`, `warn`, or `fail`
- remaining chrome leakage such as `Copy page`, `CTRL K`, edit-page links, feedback widgets, anchor glyph clutter, or raw bootstrap script output
- whether any exported page used browser fallback
