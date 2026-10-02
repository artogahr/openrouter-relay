use axum::{
    Router,
    body::Body,
    http::{HeaderMap, HeaderName, Method, Request, Response, StatusCode},
    response::IntoResponse,
    routing::any,
};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use url::Url;

#[derive(Clone)]
struct State {
    client: reqwest::Client,
    token: String,
    upstream: Url,
}

fn allowed(method: &Method, path: &str) -> bool {
    matches!(
        (method.as_str(), path),
        ("GET", "/api/v1/models")
            | ("POST", "/api/v1/chat/completions")
            | ("POST", "/api/v1/responses")
            | ("POST", "/api/v1/messages")
    )
}

fn hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "proxy-connection"
    )
}

async fn handle(state: Arc<State>, request: Request<Body>) -> Response<Body> {
    if request.method() == Method::GET && request.uri().path() == "/" {
        return Response::new(Body::from("OK"));
    }
    if !allowed(request.method(), request.uri().path()) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let mut url = state.upstream.clone();
    url.set_path(request.uri().path());
    url.set_query(request.uri().query());
    let query: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key != "token")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }

    let (parts, body) = request.into_parts();
    let mut headers = HeaderMap::new();
    for (name, value) in parts.headers {
        if let Some(name) = name
            && !hop_header(&name)
            && name != "host"
            && name != "authorization"
            && name != "x-api-key"
            && name != "cookie"
            && name != "content-length"
            && name != "forwarded"
            && name != "via"
            && !name.as_str().starts_with("x-forwarded-")
            && !name.as_str().starts_with("x-apify-")
        {
            headers.append(name, value);
        }
    }
    headers.insert(
        "authorization",
        format!("Bearer {}", state.token)
            .parse()
            .expect("valid token"),
    );

    let mut builder = state
        .client
        .request(parts.method.clone(), url)
        .headers(headers);
    if parts.method == Method::POST {
        builder = builder.body(reqwest::Body::wrap_stream(body.into_data_stream()));
    }

    match builder.send().await {
        Ok(upstream) => {
            let mut response = Response::builder().status(upstream.status());
            for (name, value) in upstream.headers() {
                if !hop_header(name) && name != "content-length" {
                    response = response.header(name, value);
                }
            }
            response
                .body(Body::from_stream(upstream.bytes_stream()))
                .expect("valid upstream response")
        }
        Err(error) => {
            eprintln!("upstream request failed: {:?}", error.status());
            (StatusCode::BAD_GATEWAY, "OpenRouter relay request failed").into_response()
        }
    }
}

async fn check_standard_run(
    client: &reqwest::Client,
    token: &str,
    upstream: &Url,
    dataset_id: &str,
    api_base: &Url,
) -> Result<(), Box<dyn std::error::Error>> {
    let models = client
        .get(upstream.join("/api/v1/models")?)
        .bearer_auth(token)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    if !models.status().is_success() {
        return Err(format!("OpenRouter model catalog returned {}", models.status()).into());
    }
    let dataset_url = api_base.join(&format!("/v2/datasets/{dataset_id}/items"))?;
    let stored = client
        .post(dataset_url)
        .bearer_auth(token)
        .header("content-type", "application/json")
        .body(r#"{"status":"ok","check":"OpenRouter model catalog"}"#)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    if !stored.status().is_success() {
        return Err(format!("Could not store diagnostic: {}", stored.status()).into());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("APIFY_TOKEN")?;
    let upstream = env::var("OPENROUTER_UPSTREAM")
        .unwrap_or_else(|_| "https://openrouter.apify.actor".to_owned())
        .parse()?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    if env::var("APIFY_META_ORIGIN").as_deref() != Ok("STANDBY")
        && env::var_os("ACTOR_DEFAULT_DATASET_ID").is_some()
    {
        let dataset_id = env::var("ACTOR_DEFAULT_DATASET_ID")?;
        let api_base = env::var("APIFY_API_PUBLIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.apify.com".to_owned());
        let api_base = if api_base.contains("://") {
            api_base
        } else {
            format!("https://{api_base}")
        }
        .parse()?;
        return check_standard_run(&client, &token, &upstream, &dataset_id, &api_base).await;
    }
    let state = Arc::new(State {
        client,
        token,
        upstream,
    });
    let app = Router::new().fallback(any(move |request: Request<Body>| {
        let state = Arc::clone(&state);
        async move { handle(state, request).await }
    }));
    let port = env::var("ACTOR_WEB_SERVER_PORT")
        .unwrap_or_else(|_| "4321".to_owned())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::to_bytes,
        routing::{get, post},
    };
    use futures_util::stream;
    use std::{convert::Infallible, time::Instant};

    async fn start(app: Router) -> Url {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}").parse().unwrap()
    }

    #[tokio::test]
    async fn streams_chat_and_replaces_caller_token() {
        let upstream = start(Router::new().route(
            "/api/v1/chat/completions",
            post(|request: Request<Body>| async move {
                assert_eq!(request.headers()["authorization"], "Bearer run-token");
                assert!(!request.headers().contains_key("x-api-key"));
                assert!(!request.headers().contains_key("x-forwarded-for"));
                assert!(request.uri().query().is_none());
                let body = to_bytes(request.into_body(), 1024).await.unwrap();
                assert!(
                    std::str::from_utf8(&body)
                        .unwrap()
                        .contains("deepseek/deepseek-chat")
                );
                let chunks = stream::unfold(0, |step| async move {
                    match step {
                        0 => Some((Ok::<_, Infallible>("data: first\n\n"), 1)),
                        1 => {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            Some((Ok("data: [DONE]\n\n"), 2))
                        }
                        _ => None,
                    }
                });
                Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(Body::from_stream(chunks))
                    .unwrap()
            }),
        ))
        .await;
        let state = Arc::new(State {
            client: reqwest::Client::new(),
            token: "run-token".into(),
            upstream,
        });
        let relay = start(Router::new().fallback(any(move |request: Request<Body>| {
            let state = Arc::clone(&state);
            async move { handle(state, request).await }
        })))
        .await;

        let start_time = Instant::now();
        let mut response = reqwest::Client::new()
            .post(
                relay
                    .join("/api/v1/chat/completions?token=caller-token")
                    .unwrap(),
            )
            .header("authorization", "Bearer caller-token")
            .header("x-api-key", "caller-token")
            .header("x-forwarded-for", "192.0.2.1")
            .header("content-type", "application/json")
            .body("{\"model\":\"deepseek/deepseek-chat\"}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        assert_eq!(response.chunk().await.unwrap().unwrap(), "data: first\n\n");
        assert!(start_time.elapsed() < Duration::from_millis(300));
        assert_eq!(response.text().await.unwrap(), "data: [DONE]\n\n");
    }

    #[tokio::test]
    async fn standard_run_checks_models_and_stores_result() {
        let service = start(
            Router::new()
                .route(
                    "/api/v1/models",
                    get(|request: Request<Body>| async move {
                        assert_eq!(request.headers()["authorization"], "Bearer run-token");
                        StatusCode::OK
                    }),
                )
                .route(
                    "/v2/datasets/test-dataset/items",
                    post(|request: Request<Body>| async move {
                        assert_eq!(request.headers()["authorization"], "Bearer run-token");
                        let body = to_bytes(request.into_body(), 1024).await.unwrap();
                        assert_eq!(
                            body,
                            r#"{"status":"ok","check":"OpenRouter model catalog"}"#
                        );
                        StatusCode::CREATED
                    }),
                ),
        )
        .await;
        check_standard_run(
            &reqwest::Client::new(),
            "run-token",
            &service,
            "test-dataset",
            &service,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn rejects_unlisted_routes() {
        let state = Arc::new(State {
            client: reqwest::Client::new(),
            token: "run-token".into(),
            upstream: "http://localhost:1".parse().unwrap(),
        });
        let response = handle(
            Arc::clone(&state),
            Request::builder()
                .uri("/api/v1/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = handle(
            state,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
