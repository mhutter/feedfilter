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

pub const APP: &str = concat!("feedfilter/", env!("CARGO_PKG_VERSION"));

pub type HttpClient = reqwest::Client;

pub fn build_http_client() -> HttpClient {
    reqwest::Client::builder()
        .user_agent(APP)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build HTTP client")
}

pub fn app() -> Router<HttpClient> {
    Router::new().route("/feed", get(feed))
}

#[derive(Debug, serde::Deserialize)]
pub struct FeedQuery {
    url: String,
    #[serde(default = "Default::default")]
    filter: Vec<String>,
}

pub async fn feed(
    State(http_client): State<reqwest::Client>,
    Form(query): Form<FeedQuery>,
) -> Result<Response, FeedError> {
    dbg!(&query);
    let req = http_client
        .get(query.url)
        .send()
        .await
        .map_err(FeedError::Fetch)?
        .error_for_status()
        .map_err(FeedError::Fetch)?;

    let content_type = req
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .map(|h| h.to_str().unwrap())
        .unwrap_or("application/rss+xml; charset=UTF-8")
        .to_owned();

    let body = req.bytes().await.map_err(FeedError::Read)?;

    let mut channel = Channel::read_from(&body[..]).map_err(FeedError::Parse)?;

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
    channel.items = items;

    Ok((
        [(header::SERVER, APP), (header::CONTENT_TYPE, &content_type)],
        channel.to_string(),
    )
        .into_response())
}

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
        let status = match self {
            Self::Fetch(_) | Self::Read(_) | Self::Parse(_) => StatusCode::BAD_GATEWAY,
        };
        (status, self).into_response()
    }
}
