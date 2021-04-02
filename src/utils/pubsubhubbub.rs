use reqwest::{Client, Response, Result};

async fn ping_hub(url: &str) -> Result<Response> {
    let params = [
        ("hub.mode", "publish"),
        ("hub.url", "https://docs.rs/releases/feed"),
    ];

    Client::new().post(url).form(&params).send().await
}

/// Ping the two predefined hubs. Return either the number of successfully
/// pinged hubs, or the first error.
pub async fn ping_hubs() -> Result<usize> {
    tokio::try_join![
        ping_hub("https://pubsubhubbub.appspot.com"),
        ping_hub("https://pubsubhubbub.superfeedr.com"),
    ]?;
    Ok(2)
}
