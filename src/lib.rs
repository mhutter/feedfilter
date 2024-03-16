use std::time::Duration;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::Form;
use rss::Channel;

/// Name & Version of this application
pub const APP: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const APP_REPO: &str = env!("CARGO_PKG_REPOSITORY");

pub type HttpClient = reqwest::Client;

/// Build a HTTP client
///
/// Client will be configured with
/// - request timeout
/// - user-agent header
pub fn build_http_client() -> HttpClient {
    reqwest::Client::builder()
        .user_agent(format!("{APP} ({APP_REPO})"))
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build HTTP client")
}

/// Build the application router (sans state)
pub fn app() -> Router<HttpClient> {
    Router::new().route("/feed", get(feed))
}

/// Query parameters for the feed endpoint
#[derive(Debug, serde::Deserialize)]
pub struct FeedQuery {
    url: String,
    #[serde(default = "Default::default")]
    filter: Vec<String>,
}

/// GET /feed
pub async fn feed(
    State(http_client): State<reqwest::Client>,
    Form(query): Form<FeedQuery>,
) -> Result<Response, FeedError> {
    // Fetch upstream
    let req = http_client
        .get(query.url)
        .send()
        .await
        .map_err(FeedError::Fetch)?
        .error_for_status()
        .map_err(FeedError::Fetch)?;

    // extract upstream content type
    let content_type = req
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .map(|h| h.to_str().unwrap())
        .unwrap_or("application/rss+xml; charset=UTF-8")
        .to_owned();

    // read & parse body
    let body = req.bytes().await.map_err(FeedError::Read)?;
    let mut channel = Channel::read_from(&body[..]).map_err(FeedError::Parse)?;

    // filter items according to filter terms
    let items = channel
        .items
        .into_iter()
        .filter(|item| {
            !item
                .title
                .as_ref()
                .map(|title| query.filter.iter().any(|fp| title.contains(fp)))
                .unwrap_or_default()
        })
        .collect();

    // update parsed channel info
    channel.items = items;

    // Render back as RSS
    Ok((
        [(header::SERVER, APP), (header::CONTENT_TYPE, &content_type)],
        channel.to_string(),
    )
        .into_response())
}

/// Errors that might occur on the feed endpoint
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("Failed to fetch upstream feed: {0}")]
    Fetch(reqwest::Error),

    #[error("Failed to read upstream body: {0}")]
    Read(reqwest::Error),

    #[error("Failed to parse upstream body: {0}")]
    Parse(rss::Error),
}

impl IntoResponse for FeedError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_GATEWAY, self).into_response()
    }
}
