use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut browser, mut handler) = Browser::launch(BrowserConfig::builder().build()?).await?;
    let _handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await { }
    });
    let page = browser.new_page("https://example.com").await?;
    let el = page.find_element("h1").await?;
    println!("Found element");
    Ok(())
}
