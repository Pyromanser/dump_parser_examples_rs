use chrono;
use scraper::{Html, Selector};
use std::error::Error;
use std::fs::{self, DirBuilder};
use std::time::SystemTime;

const SITE_BASE_URL: &str = "http://translatedby.com";
const TAG: &str = "GURPS";
const TAG_URL: &str = "http://translatedby.com/you/tags/GURPS/";

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "Starting dumping {} at {}",
        TAG,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let start = SystemTime::now();

    let timestamp: String = format!("{}", chrono::Local::now().format("%Y-%m-%d"));
    let dump_dir_name: String = format!("{}_{}_rust", TAG, timestamp);
    DirBuilder::new().create(&dump_dir_name)?;

    let text = reqwest::blocking::get(TAG_URL)?.text()?;

    let document = Html::parse_document(&text);
    let selector = Selector::parse(r#"div.spager a"#)?;

    let mut pages_urls: Vec<String> = vec![TAG_URL.to_owned()];

    pages_urls.extend(
        document
            .select(&selector)
            .map(|a| {
                let url = a.value().attr("href").ok_or("No href attribute")?;
                Ok(format!("{}{}", SITE_BASE_URL, url))
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
    );

    pages_urls
        .iter()
        .map(|url| parse_page(url, &dump_dir_name))
        .for_each(drop);

    println!("Duration: {:?}", SystemTime::now().duration_since(start)?);
    println!(
        "End {} at {}",
        TAG,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    Ok(())
}

fn parse_page(url: &str, dump_dir_name: &str) {
    println!("Page: {}", url);

    let resp = reqwest::blocking::get(url).expect("Server error");
    let text = resp.text().expect("Unable to read response");

    let document = Html::parse_document(&text);
    let selector = Selector::parse(r#"dl.translations-list dt a"#).expect("Selector error");

    let mut book_names = Vec::new();
    let mut book_urls = Vec::new();

    document
        .select(&selector)
        .map(|a| {
            book_names.push(a.text().collect::<Vec<_>>().join("").trim().to_owned());
            book_urls.push(format!(
                "{}{}",
                SITE_BASE_URL,
                a.value().attr("href").unwrap().replace("/trans/", "/")
            ));
        })
        .for_each(drop);
    println!("{:#?}", book_names);
    println!("{:#?}", book_urls);

    book_names
        .iter()
        .zip(book_urls.iter())
        .map(|(name, url)| parse_book(name, url, &dump_dir_name))
        .for_each(drop);
}

fn parse_book(name: &str, url: &str, dump_dir_name: &str) {
    println!("Book: {}", name);
    println!("Book url: {}", url);

    let about_page_url = format!("{}stats/", url);
    let book_file_url = format!("{}.txt", url);

    let book_dir_name = format!("{}/{}", dump_dir_name, name);
    DirBuilder::new()
        .create(&book_dir_name)
        .expect("Unable to create directory");

    let resp = reqwest::blocking::get(about_page_url).expect("Server error");
    let text = resp.text().expect("Unable to read response");

    let document = Html::parse_document(&text);
    let selector = Selector::parse(r#"#about-translation blockquote"#).expect("Selector error");
    let blockquote = match document.select(&selector).next() {
        Some(a) => a.text().collect::<Vec<_>>().join("").trim().to_owned(),
        None => String::new(),
    };

    fs::write(
        format!("{}/about.txt", book_dir_name),
        format!("URL - {}\n{}", url, blockquote).as_bytes(),
    )
    .expect("Unable to write file");

    let resp = reqwest::blocking::get(book_file_url).expect("Server error");
    let text = resp.text().expect("Unable to read response");

    fs::write(format!("{}/book.txt", book_dir_name), text.as_bytes())
        .expect("Unable to write file");
}
