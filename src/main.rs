use chrono;
use futures::{stream, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use std::error::Error;
use std::fs::{self, DirBuilder};
use std::time::SystemTime;
use tokio;

const SITE_BASE_URL: &str = "http://translatedby.com";
const TAG: &str = "GURPS";
const TAG_URL: &str = "http://translatedby.com/you/tags/GURPS/";
const CONCURRENT_REQUESTS: usize = 2;

async fn get_pages_urls(url: &str, client: &Client) -> Result<Vec<String>, Box<dyn Error>> {
    let mut pages_urls: Vec<String> = vec![url.to_owned()];

    let text = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&text);
    let selector = Selector::parse(r#"div.spager a"#)?;

    pages_urls.extend(
        document
            .select(&selector)
            .map(|a| {
                let url = a.value().attr("href").ok_or("No href attribute")?;
                Ok(format!("{}{}", SITE_BASE_URL, url))
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
    );

    Ok(pages_urls)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "Starting dumping {} at {}",
        TAG,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let start = SystemTime::now();

    let timestamp: String = format!("{}", chrono::Local::now().format("%Y-%m-%d"));
    let dump_dir_name: String = format!("{}_{}_rust", TAG, timestamp);
    DirBuilder::new().create(&dump_dir_name)?;

    let client = Client::new();

    let pages_urls = get_pages_urls(TAG_URL, &client).await?;

    let requests = stream::iter(pages_urls)
        .map(|url| {
            let client = &client;
            let dump_dir_name = &dump_dir_name;
            async move { parse_page(&url, &dump_dir_name, &client).await }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    requests
        .for_each(|b| async {
            match b {
                Ok(_) => println!("Ok",),
                Err(e) => eprintln!("Got an error: {}", e),
            }
        })
        .await;

    println!("Duration: {:?}", SystemTime::now().duration_since(start)?);
    println!(
        "End {} at {}",
        TAG,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    Ok(())
}

async fn parse_page(url: &str, dump_dir_name: &str, client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Page: {}", url);

    let text = client.get(url).send().await?.text().await?;

    let document = Html::parse_document(&text);
    let selector = Selector::parse(r#"dl.translations-list dt a"#)?;

    let mut book_names = Vec::new();
    let mut book_urls = Vec::new();

    document
        .select(&selector)
        .map(|a| {
            if let Some(url) = a.value().attr("href") {
                book_names.push(a.text().collect::<Vec<_>>().join("").trim().to_owned());
                book_urls.push(format!("{}{}", &SITE_BASE_URL, url.replace("/trans/", "/")));
            };
        })
        .for_each(drop);
    println!("{:#?}", book_names);
    println!("{:#?}", book_urls);

    let requests = stream::iter(book_names.iter().zip(book_urls.iter()))
        .map(|(name, url)| async move { parse_book(name, url, &dump_dir_name, &client).await })
        .buffer_unordered(CONCURRENT_REQUESTS);

    requests
        .for_each(|b| async {
            match b {
                Ok(_) => println!("Ok",),
                Err(e) => eprintln!("Got an error: {}", e),
            }
        })
        .await;

    Ok(())
}

async fn parse_book(
    name: &str,
    url: &str,
    dump_dir_name: &str,
    client: &Client,
) -> Result<(), Box<dyn Error>> {
    println!("Book: {}", name);
    println!("Book url: {}", url);

    let about_page_url = format!("{}stats/", url);
    let book_file_url = format!("{}.txt", url);

    let book_dir_name = format!("{}/{}", dump_dir_name, name);
    DirBuilder::new().create(&book_dir_name)?;

    let text = client.get(&about_page_url).send().await?.text().await?;

    let document = Html::parse_document(&text);
    let selector = Selector::parse(r#"#about-translation blockquote"#)?;
    let blockquote = match document.select(&selector).next() {
        Some(a) => a.text().collect::<Vec<_>>().join("").trim().to_owned(),
        None => String::new(),
    };

    fs::write(
        format!("{}/about.txt", &book_dir_name),
        format!("URL - {}\n{}", url, blockquote).as_bytes(),
    )?;

    let text = client.get(&book_file_url).send().await?.text().await?;

    fs::write(format!("{}/book.txt", &book_dir_name), text.as_bytes())?;
    Ok(())
}
