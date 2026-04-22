# docsite-to-md

`docsite-to-md` is a Rust CLI and library for exporting documentation sites into Markdown.

It is designed for docs archiving, indexing, offline reading, and LLM preparation with a stronger focus on:

- extractor-based site detection
- GitBook support, including modern Next.js-powered GitBook sites
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
