use anyhow::Result;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// A single news item/headline
#[derive(Debug, Clone)]
pub struct NewsItem {
    pub title: String,
    pub link: String,
    pub pub_date: Option<DateTime<Utc>>,
    pub source: String,
}

/// Collection of news items
#[derive(Debug, Clone, Default)]
pub struct NewsFeed {
    pub items: Vec<NewsItem>,
    pub last_updated: Option<DateTime<Utc>>,
    pub is_loading: bool,
    pub error: Option<String>,
}

/// News feed poller
pub struct NewsPoller;

impl NewsPoller {
    /// Start background polling for news
    pub fn start(poll_interval: Duration) -> watch::Receiver<NewsFeed> {
        let (tx, rx) = watch::channel(NewsFeed {
            is_loading: true,
            ..Default::default()
        });

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent("control-system/1.0")
                .build()
                .unwrap_or_default();

            let mut interval = tokio::time::interval(poll_interval);

            loop {
                interval.tick().await;
                
                let mut feed = NewsFeed {
                    is_loading: true,
                    ..Default::default()
                };
                let _ = tx.send(feed.clone());

                // Fetch from multiple Malaysian news sources
                let mut all_items = Vec::new();

                // The Star Malaysia - Nation news
                match fetch_rss(&client, "https://www.thestar.com.my/rss/News/Nation", "Star").await {
                    Ok(items) => {
                        debug!("Fetched {} items from The Star", items.len());
                        all_items.extend(items);
                    }
                    Err(e) => warn!("Failed to fetch The Star: {}", e),
                }

                // Malay Mail
                match fetch_rss(&client, "https://www.malaymail.com/feed/rss/malaysia", "MM").await {
                    Ok(items) => {
                        debug!("Fetched {} items from Malay Mail", items.len());
                        all_items.extend(items);
                    }
                    Err(e) => warn!("Failed to fetch Malay Mail: {}", e),
                }

                // Free Malaysia Today
                match fetch_rss(&client, "https://www.freemalaysiatoday.com/feed/", "FMT").await {
                    Ok(items) => {
                        debug!("Fetched {} items from FMT", items.len());
                        all_items.extend(items);
                    }
                    Err(e) => warn!("Failed to fetch FMT: {}", e),
                }

                // Sort by date (newest first) and take top 10
                all_items.sort_by(|a, b| {
                    b.pub_date.cmp(&a.pub_date)
                });
                all_items.truncate(10);

                feed = NewsFeed {
                    items: all_items,
                    last_updated: Some(Utc::now()),
                    is_loading: false,
                    error: None,
                };

                if feed.items.is_empty() {
                    feed.error = Some("No news available".to_string());
                } else {
                    info!("News feed updated: {} headlines", feed.items.len());
                }

                if tx.send(feed).is_err() {
                    break;
                }
            }
        });

        rx
    }
}

/// Fetch and parse an RSS feed
async fn fetch_rss(client: &reqwest::Client, url: &str, source: &str) -> Result<Vec<NewsItem>> {
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    
    parse_rss(&text, source)
}

/// Parse RSS XML into news items
fn parse_rss(xml: &str, source: &str) -> Result<Vec<NewsItem>> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut items = Vec::new();
    let mut current_item: Option<NewsItemBuilder> = None;
    let mut current_tag = String::new();
    let mut in_item = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_tag = tag_name.clone();
                
                if tag_name == "item" || tag_name == "entry" {
                    in_item = true;
                    current_item = Some(NewsItemBuilder::new(source));
                }
            }
            Ok(Event::End(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                if (tag_name == "item" || tag_name == "entry") && in_item {
                    if let Some(builder) = current_item.take() {
                        if let Some(item) = builder.build() {
                            items.push(item);
                        }
                    }
                    in_item = false;
                }
                current_tag.clear();
            }
            Ok(Event::Text(e)) => {
                if in_item {
                    if let Some(ref mut builder) = current_item {
                        let text = e.unescape().unwrap_or_default().to_string();
                        if !text.is_empty() {
                            match current_tag.as_str() {
                                "title" => builder.title = Some(text),
                                "link" => {
                                    if builder.link.is_none() {
                                        builder.link = Some(text);
                                    }
                                }
                                "pubDate" | "published" => builder.pub_date_str = Some(text),
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Event::CData(e)) => {
                // CDATA sections contain the actual content for many RSS feeds
                if in_item {
                    if let Some(ref mut builder) = current_item {
                        let text = String::from_utf8_lossy(&e).to_string();
                        if !text.is_empty() {
                            match current_tag.as_str() {
                                "title" => builder.title = Some(text),
                                "link" => {
                                    if builder.link.is_none() {
                                        builder.link = Some(text);
                                    }
                                }
                                "pubDate" | "published" => builder.pub_date_str = Some(text),
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                error!("RSS parse error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(items)
}

/// Builder for constructing NewsItem
struct NewsItemBuilder {
    title: Option<String>,
    link: Option<String>,
    pub_date_str: Option<String>,
    source: String,
}

impl NewsItemBuilder {
    fn new(source: &str) -> Self {
        Self {
            title: None,
            link: None,
            pub_date_str: None,
            source: source.to_string(),
        }
    }

    fn build(self) -> Option<NewsItem> {
        let title = self.title?;
        
        // Clean up the title - remove HTML entities and extra whitespace
        let title = html_decode(&title)
            .lines()
            .next()
            .unwrap_or(&title)
            .trim()
            .to_string();

        if title.is_empty() {
            return None;
        }

        let link = self.link.unwrap_or_default();
        
        // Parse the publication date
        let pub_date = self.pub_date_str.and_then(|s| {
            // Try common RSS date formats
            chrono::DateTime::parse_from_rfc2822(&s)
                .or_else(|_| chrono::DateTime::parse_from_rfc3339(&s))
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        Some(NewsItem {
            title,
            link,
            pub_date,
            source: self.source,
        })
    }
}

/// Basic HTML entity decoding
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}
