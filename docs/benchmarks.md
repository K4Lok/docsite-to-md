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

Latest local framework export baseline:

| Framework | Fixture pages | Export time | Throughput |
| --- | ---: | ---: | ---: |
| `GitBookModern` | 3 | 3.50 ms | 858 pages/s |
| `GitBookClassic` | 2 | 2.78 ms | 720 pages/s |
| `Docusaurus` | 3 | 4.17 ms | 719 pages/s |
| `MkDocsMaterial` | 3 | 3.39 ms | 885 pages/s |
| `VitePress` | 3 | 3.87 ms | 776 pages/s |
| `Nextra` | 3 | 3.27 ms | 918 pages/s |
| `GenericDocsFallback` | 4 | 3.86 ms | 1,037 pages/s |

Run only this table's benchmark group with:

```bash
cargo bench --bench export export_framework_fixtures
```

These are local fixture timings, not live-site guarantees. Real exports depend on network latency, page size, rate limits, and site markup.

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
