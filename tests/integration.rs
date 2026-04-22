mod support;

use std::fs;

use docsite_to_md::{
    BrowserOptions, BundleOptions, CrawlOptions, ExportOptions, Framework, bundle_site, crawl_site,
    detect_site, export_site,
};
use tempfile::tempdir;

fn fixture(path: &str) -> String {
    fs::read_to_string(format!(
        "/Users/k4lok/Development/OpenSources/docsite-to-md/tests/fixtures/{path}"
    ))
    .expect("fixture should exist")
}

#[tokio::test]
async fn detects_modern_gitbook() {
    let server =
        support::serve_routes(&[("/", "text/html", &fixture("modern_gitbook/root.html"))]).await;

    let profile = detect_site(&server.uri())
        .await
        .expect("site should detect");
    assert_eq!(profile.framework, Framework::GitBookModern);
    assert!(profile.supports_markdown_endpoints);
}

#[tokio::test]
async fn crawl_manifest_respects_scope_and_excludes() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("generic_docs/root.html")),
        ("/guide", "text/html", &fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            &fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", &fixture("generic_docs/blog.html")),
    ])
    .await;

    let manifest = crawl_site(
        &server.uri(),
        CrawlOptions {
            excludes: vec!["blog".to_string()],
            ..CrawlOptions::default()
        },
    )
    .await
    .expect("crawl should succeed");

    assert!(manifest.pages.iter().any(|page| page.url == server.uri()));
    assert!(
        manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/guide"))
    );
    assert!(
        !manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/blog"))
    );
}

#[tokio::test]
async fn export_creates_mirror_tree_and_manifest() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("modern_gitbook/root.html")),
        (
            "/getting-started",
            "text/html",
            &fixture("modern_gitbook/getting-started.html"),
        ),
        (
            "/api/reference",
            "text/html",
            &fixture("modern_gitbook/api-reference.html"),
        ),
        ("/.md", "text/markdown", &fixture("modern_gitbook/root.md")),
        (
            "/getting-started.md",
            "text/markdown",
            &fixture("modern_gitbook/getting-started.md"),
        ),
        (
            "/api/reference.md",
            "text/markdown",
            &fixture("modern_gitbook/api-reference.md"),
        ),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("export"),
            crawl: CrawlOptions::default(),
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    assert!(result.output_dir.join("index.md").exists());
    assert!(result.output_dir.join("getting-started.md").exists());
    assert!(result.output_dir.join("api/reference.md").exists());
    assert!(result.manifest_path.exists());
    assert!(result.links_path.exists());

    let root_md = fs::read_to_string(result.output_dir.join("index.md")).expect("root markdown");
    assert!(root_md.contains("[!DANGER]"));
}

#[tokio::test]
async fn export_deduplicates_same_content() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("generic_docs/root.html")),
        ("/guide", "text/html", &fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            &fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", &fixture("generic_docs/blog.html")),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("export"),
            crawl: CrawlOptions {
                excludes: vec!["blog".to_string()],
                ..CrawlOptions::default()
            },
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    let duplicate = result
        .pages
        .iter()
        .find(|page| page.page.url.ends_with("/guide-duplicate"))
        .expect("duplicate page should exist");
    assert!(duplicate.skipped || duplicate.duplicate_of.is_some());
}

#[tokio::test]
async fn retries_flaky_route() {
    let server = support::serve_with_flaky_route(
        &[
            ("/", "text/html", &fixture("generic_docs/root.html")),
            ("/guide", "text/html", &fixture("generic_docs/guide.html")),
            (
                "/guide-duplicate",
                "text/html",
                &fixture("generic_docs/guide-duplicate.html"),
            ),
            ("/blog", "text/html", &fixture("generic_docs/blog.html")),
        ],
        "/guide",
        500,
        200,
        &fixture("generic_docs/guide.html"),
    )
    .await;

    let manifest = crawl_site(
        &server.uri(),
        CrawlOptions {
            retry_attempts: 2,
            ..CrawlOptions::default()
        },
    )
    .await
    .expect("crawl should succeed after retry");

    assert!(
        manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/guide"))
    );
}

#[tokio::test]
async fn bundle_command_writes_single_file() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("generic_docs/root.html")),
        ("/guide", "text/html", &fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            &fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", &fixture("generic_docs/blog.html")),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let bundle_path = output.path().join("bundle.md");
    let path = bundle_site(
        &server.uri(),
        BundleOptions {
            crawl: CrawlOptions {
                excludes: vec!["blog".to_string()],
                ..CrawlOptions::default()
            },
            output_file: bundle_path.clone(),
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("bundle should succeed");

    let content = fs::read_to_string(path).expect("bundle file");
    assert!(content.contains("# Documentation Bundle"));
    assert!(content.contains("## Guide"));
}

#[tokio::test]
async fn classic_gitbook_detects_and_exports_markdown() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("classic_gitbook/root.html")),
        (
            "/chapter-1",
            "text/html",
            &fixture("classic_gitbook/chapter-1.html"),
        ),
        (
            "/README.md",
            "text/markdown",
            &fixture("classic_gitbook/README.md"),
        ),
        (
            "/chapter-1.md",
            "text/markdown",
            &fixture("classic_gitbook/chapter-1.md"),
        ),
    ])
    .await;

    let profile = detect_site(&server.uri()).await.expect("detect classic");
    assert_eq!(profile.framework, Framework::GitBookClassic);
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_mycactus() {
    let profile = detect_site("https://apidoc.mycactus.com")
        .await
        .expect("mycactus should detect");
    assert!(matches!(
        profile.framework,
        Framework::GitBookModern | Framework::GitBookClassic
    ));
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_gitbook() {
    let profile = detect_site("https://docs.gitbook.com")
        .await
        .expect("docs.gitbook.com should detect");
    assert!(matches!(
        profile.framework,
        Framework::GitBookModern | Framework::GitBookClassic
    ));
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_non_gitbook() {
    let profile = detect_site("https://www.rust-lang.org/learn")
        .await
        .expect("rust-lang should detect");
    assert_eq!(profile.framework, Framework::GenericDocsFallback);
}
