use std::collections::HashMap;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub async fn serve_routes(routes: &[(&str, &str, &str)]) -> MockServer {
    let server = MockServer::start().await;

    for (route_path, content_type, body) in routes {
        Mock::given(method("GET"))
            .and(path(*route_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", *content_type)
                    .set_body_string((*body).to_string()),
            )
            .mount(&server)
            .await;
    }

    server
}

pub async fn serve_with_flaky_route(
    routes: &[(&str, &str, &str)],
    flaky_path: &str,
    first_status: u16,
    second_status: u16,
    second_body: &str,
) -> MockServer {
    let server = MockServer::start().await;
    let mut routes_map = HashMap::new();
    for (route_path, content_type, body) in routes {
        routes_map.insert(*route_path, (*content_type, *body));
    }

    for (route_path, (content_type, body)) in routes_map {
        if route_path == flaky_path {
            Mock::given(method("GET"))
                .and(path(route_path))
                .respond_with(ResponseTemplate::new(first_status))
                .up_to_n_times(1)
                .mount(&server)
                .await;

            Mock::given(method("GET"))
                .and(path(route_path))
                .respond_with(
                    ResponseTemplate::new(second_status)
                        .insert_header("content-type", content_type)
                        .set_body_string(second_body.to_string()),
                )
                .mount(&server)
                .await;
        } else {
            Mock::given(method("GET"))
                .and(path(route_path))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", content_type)
                        .set_body_string(body.to_string()),
                )
                .mount(&server)
                .await;
        }
    }

    server
}
