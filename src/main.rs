use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "[::1]:4159".to_string());
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("bind to {addr}: {err}"));

    let http_client = feedfilter::build_http_client();
    let app = feedfilter::app().with_state(http_client);

    println!("---> Listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
