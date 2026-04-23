# Benchmarks

`docsite-to-md` has two benchmark layers:

- Repeatable local benchmarks use checked-in HTML and Markdown fixtures behind a local mock server. These are stable enough for day-to-day development and CI.
- Ignored live benchmarks exercise real public documentation sites. These are useful before a release, but results can change when those sites change.

## Repeatable Benchmarks

Run deterministic fixture benchmarks with:

```bash
cargo bench
```

The Criterion suite measures:

- framework detection over a Docusaurus fixture
- crawl discovery over a generic docs fixture
- mirror-tree export over a modern GitBook fixture
- Markdown normalization over a modern GitBook fixture
- bundle generation over a generic docs fixture
- framework-by-framework fixture export speed

Latest live export benchmark:

| Framework | Live target | Max pages | Exported pages | Wall time | Throughput |
| --- | --- | ---: | ---: | ---: | ---: |
| `GitBookModern` | [GitBook docs](https://docs.gitbook.com/) | 100 | 100 | 43.69 s | 2.29 pages/s |
| `Docusaurus` | [React Native docs](https://reactnative.dev/docs/getting-started) | 100 | 79 | 33.75 s | 2.34 pages/s |
| `MkDocsMaterial` | [MkDocs Material](https://squidfunk.github.io/mkdocs-material/) | 100 | 94 | 58.74 s | 1.60 pages/s |
| `VitePress` | [VitePress docs](https://vitepress.dev/) | 100 | 100 | 19.84 s | 5.04 pages/s |
| `Nextra` | [Nextra docs](https://nextra.site/) | 100 | 77 | 55.61 s | 1.38 pages/s |

These numbers were captured with the release binary, `--max-pages 100`, `--concurrency 8`, and live network requests. They are a snapshot, not a guarantee: live sites vary with network latency, page size, rate limits, and site markup.

Run the live benchmark table with:

```bash
bash scripts/live-benchmark.sh
```

Run only the fixture framework benchmark group with:

```bash
cargo bench --bench export export_framework_fixtures
```

## Package Size

Run the package-size report with:

```bash
bash scripts/package-size.sh
```

To also measure the optional browser feature build:

```bash
bash scripts/package-size.sh --with-browser
```

Current local baseline:

| Metric | Command | Latest local value |
| --- | --- | ---: |
| Crate compressed size | `cargo package --no-verify` | 49.8 KiB |
| Release binary size | `cargo build --release --locked` | 9.4 MiB |
| Fixture export speed | `cargo bench` | Criterion report |
| Live docs quality | ignored live benchmark | run before release |

## Live Quality Benchmarks

Run the ignored live benchmark report with:

```bash
cargo test --test live_benchmark live_benchmark_report -- --ignored --nocapture
```

Use this before publishing when you want confidence against the curated public docs targets in `tests/live_targets.json`.
