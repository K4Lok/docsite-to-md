use std::fs;
use std::time::{Duration, Instant};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use docsite_to_md::Framework;
use docsite_to_md::normalize::normalize_markdown;
use docsite_to_md::{
    BrowserOptions, BundleOptions, CrawlOptions, ExportOptions, bundle_site, crawl_site,
    detect_site, export_site,
};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(path: &str) -> String {
    fs::read_to_string(format!(
        "{}/tests/fixtures/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture should exist")
}

fn bench_detection(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should create");
    let server = runtime.block_on(serve_routes(&[(
        "/",
        "text/html",
        fixture("docusaurus/root.html"),
    )]));
    let url = server.uri();

    c.bench_function("detect_docusaurus_fixture", |bench| {
        bench.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                runtime
                    .block_on(detect_site(&url))
                    .expect("docusaurus should detect");
                elapsed += start.elapsed();
                runtime.block_on(async {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                });
            }
            elapsed
        });
    });
}

fn bench_crawl(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should create");
    let server = runtime.block_on(serve_routes(&generic_routes()));
    let url = server.uri();

    c.bench_function("crawl_generic_fixture", |bench| {
        bench.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                runtime
                    .block_on(crawl_site(
                        &url,
                        CrawlOptions {
                            excludes: vec!["blog".to_string()],
                            ..CrawlOptions::default()
                        },
                    ))
                    .expect("crawl should succeed");
                elapsed += start.elapsed();
                runtime.block_on(async {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                });
            }
            elapsed
        });
    });
}

fn bench_export(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should create");
    let server = runtime.block_on(serve_routes(&modern_gitbook_routes()));
    let url = server.uri();

    c.bench_function("export_modern_gitbook_fixture", |bench| {
        bench.iter_batched(
            || tempdir().expect("tempdir should create"),
            |output| {
                runtime
                    .block_on(export_site(
                        &url,
                        ExportOptions {
                            output_dir: output.path().join("export"),
                            crawl: CrawlOptions::default(),
                            resume: false,
                            bundle_output: None,
                            browser: BrowserOptions::default(),
                        },
                    ))
                    .expect("export should succeed")
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_framework_exports(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should create");
    let mut group = c.benchmark_group("export_framework_fixtures");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(700));
    group.sample_size(10);

    for case in framework_cases() {
        group.throughput(Throughput::Elements(case.pages as u64));
        group.bench_function(BenchmarkId::new("framework", case.name), |bench| {
            bench.iter_custom(|iters| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iters {
                    let server = runtime.block_on(serve_routes(&case.routes));
                    let url = server.uri();
                    let output = tempdir().expect("tempdir should create");
                    let start = Instant::now();
                    let result = runtime
                        .block_on(export_site(
                            &url,
                            ExportOptions {
                                output_dir: output.path().join("export"),
                                crawl: case.crawl.clone(),
                                resume: false,
                                bundle_output: None,
                                browser: case.browser.clone(),
                            },
                        ))
                        .expect("export should succeed");
                    elapsed += start.elapsed();
                    assert_eq!(result.pages.len(), case.pages);
                    runtime.block_on(async {
                        tokio::time::sleep(Duration::from_millis(2)).await;
                    });
                }
                elapsed
            });
        });
    }

    group.finish();
}

fn bench_normalization(c: &mut Criterion) {
    let markdown = fs::read_to_string(format!(
        "{}/tests/fixtures/modern_gitbook/getting-started.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture should exist");

    c.bench_function("normalize_gitbook_markdown_fixture", |bench| {
        bench.iter(|| normalize_markdown(&markdown, &Framework::GitBookModern));
    });
}

fn bench_bundle(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should create");
    let server = runtime.block_on(serve_routes(&generic_routes()));
    let url = server.uri();

    c.bench_function("bundle_generic_fixture", |bench| {
        bench.iter_batched(
            || tempdir().expect("tempdir should create"),
            |output| {
                runtime
                    .block_on(bundle_site(
                        &url,
                        BundleOptions {
                            crawl: CrawlOptions {
                                excludes: vec!["blog".to_string()],
                                ..CrawlOptions::default()
                            },
                            output_file: output.path().join("bundle.md"),
                            browser: BrowserOptions::default(),
                        },
                    ))
                    .expect("bundle should succeed")
            },
            BatchSize::SmallInput,
        );
    });
}

fn generic_routes() -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("/", "text/html", fixture("generic_docs/root.html")),
        ("/guide", "text/html", fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", fixture("generic_docs/blog.html")),
    ]
}

fn modern_gitbook_routes() -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("/", "text/html", fixture("modern_gitbook/root.html")),
        (
            "/getting-started",
            "text/html",
            fixture("modern_gitbook/getting-started.html"),
        ),
        (
            "/api/reference",
            "text/html",
            fixture("modern_gitbook/api-reference.html"),
        ),
        ("/.md", "text/markdown", fixture("modern_gitbook/root.md")),
        (
            "/getting-started.md",
            "text/markdown",
            fixture("modern_gitbook/getting-started.md"),
        ),
        (
            "/api/reference.md",
            "text/markdown",
            fixture("modern_gitbook/api-reference.md"),
        ),
    ]
}

struct FrameworkCase {
    name: &'static str,
    pages: usize,
    routes: Vec<(&'static str, &'static str, String)>,
    crawl: CrawlOptions,
    browser: BrowserOptions,
}

fn framework_cases() -> Vec<FrameworkCase> {
    vec![
        FrameworkCase {
            name: "gitbook_modern",
            pages: 3,
            routes: modern_gitbook_routes(),
            crawl: CrawlOptions::default(),
            browser: BrowserOptions::default(),
        },
        FrameworkCase {
            name: "gitbook_classic",
            pages: 2,
            routes: vec![
                ("/", "text/html", fixture("classic_gitbook/root.html")),
                (
                    "/chapter-1",
                    "text/html",
                    fixture("classic_gitbook/chapter-1.html"),
                ),
                (
                    "/README.md",
                    "text/markdown",
                    fixture("classic_gitbook/README.md"),
                ),
                (
                    "/chapter-1.md",
                    "text/markdown",
                    fixture("classic_gitbook/chapter-1.md"),
                ),
            ],
            crawl: CrawlOptions::default(),
            browser: BrowserOptions::default(),
        },
        FrameworkCase {
            name: "docusaurus",
            pages: 3,
            routes: vec![
                ("/", "text/html", fixture("docusaurus/root.html")),
                ("/docs/intro", "text/html", fixture("docusaurus/intro.html")),
                (
                    "/docs/tutorial",
                    "text/html",
                    fixture("docusaurus/tutorial.html"),
                ),
            ],
            crawl: CrawlOptions::default(),
            browser: BrowserOptions::default(),
        },
        FrameworkCase {
            name: "mkdocs_material",
            pages: 3,
            routes: vec![
                ("/", "text/html", fixture("mkdocs/root.html")),
                (
                    "/getting-started",
                    "text/html",
                    fixture("mkdocs/getting-started.html"),
                ),
                ("/reference", "text/html", fixture("mkdocs/reference.html")),
            ],
            crawl: CrawlOptions::default(),
            browser: BrowserOptions::default(),
        },
        FrameworkCase {
            name: "vitepress",
            pages: 3,
            routes: vec![
                ("/", "text/html", fixture("vitepress/root.html")),
                ("/guide/start", "text/html", fixture("vitepress/start.html")),
                (
                    "/guide/config",
                    "text/html",
                    fixture("vitepress/config.html"),
                ),
            ],
            crawl: CrawlOptions::default(),
            browser: BrowserOptions::default(),
        },
        FrameworkCase {
            name: "nextra",
            pages: 3,
            routes: vec![
                ("/", "text/html", fixture("nextra/root.html")),
                ("/docs/intro", "text/html", fixture("nextra/intro.html")),
                (
                    "/docs/advanced",
                    "text/html",
                    fixture("nextra/advanced.html"),
                ),
            ],
            crawl: CrawlOptions::default(),
            browser: BrowserOptions {
                enabled: true,
                webdriver_url: None,
            },
        },
        FrameworkCase {
            name: "generic_fallback",
            pages: 4,
            routes: generic_routes(),
            crawl: CrawlOptions::default(),
            browser: BrowserOptions::default(),
        },
    ]
}

async fn serve_routes(routes: &[(&str, &str, String)]) -> MockServer {
    let server = MockServer::start().await;

    for (route_path, content_type, body) in routes {
        Mock::given(method("GET"))
            .and(path(*route_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", *content_type)
                    .set_body_string(body.clone()),
            )
            .mount(&server)
            .await;
    }

    server
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(10);
    targets = bench_detection, bench_crawl, bench_export, bench_framework_exports, bench_normalization, bench_bundle
}
criterion_main!(benches);
