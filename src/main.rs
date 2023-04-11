use chrono;
use scraper::{Html, Selector};
use std::error::Error;
use std::fs::{self, DirBuilder};
use std::time::SystemTime;

const SITE_BASE_URL: &str = "http://translatedby.com";
const TAG: &str = "GURPS";
const TAG_URL: &str = "http://translatedby.com/you/tags/GURPS/";

fn get_pages_urls(url: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut pages_urls: Vec<String> = vec![url.to_owned()];

    let text = reqwest::blocking::get(url).expect("Server error").text()?;
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

    let pages_urls = get_pages_urls(TAG_URL)?;
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

fn parse_page(url: &str, dump_dir_name: &str) -> Result<(), Box<dyn Error>> {
    println!("Page: {}", url);

    let text = reqwest::blocking::get(url)?.text()?;

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

    book_names
        .iter()
        .zip(book_urls.iter())
        .map(|(name, url)| parse_book(name, url, &dump_dir_name))
        .for_each(drop);
    Ok(())
}

fn parse_book(name: &str, url: &str, dump_dir_name: &str) -> Result<(), Box<dyn Error>> {
    println!("Book: {}", name);
    println!("Book url: {}", url);

    let about_page_url = format!("{}stats/", url);
    let book_file_url = format!("{}.txt", url);

    let book_dir_name = format!("{}/{}", dump_dir_name, name);
    DirBuilder::new().create(&book_dir_name)?;

    let text = reqwest::blocking::get(&about_page_url)?.text()?;

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

    let text = reqwest::blocking::get(&book_file_url)?.text()?;

    fs::write(format!("{}/book.txt", &book_dir_name), text.as_bytes())?;
    Ok(())
}
