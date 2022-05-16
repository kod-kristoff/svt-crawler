// #!/usr/bin/env python3
//
// """Crawler for SVT news."""
//
use clap::{Arg, Command};
// import json
// import math
// import re
// import sys
// import time
// import traceback
// from collections import defaultdict
// from datetime import datetime
use std::time::Duration;
use std::path::{Path, PathBuf};

#[macro_use]
extern crate lazy_static;
use svt_crawler::Page;
//
// import requests
// from lxml import etree
//
const DATADIR: &str = "data";
// CRAWLED = DATADIR / Path("crawled_pages.json")
// FAILED = DATADIR / Path("failed_urls.json")
// PROCESSED_JSON = DATADIR / Path("processed_json.json")
//
//
// -------------------------------------------------------------------------------
//  Define the command line args
// -------------------------------------------------------------------------------
fn parse_args() -> Args {
    let matches = Command::new("svt-crawler")
        .about("Programme for crawling svt.se for news articles and converting the data to XML.")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("crawl")
                .about("Crawl svt.se and download news articles")
                .arg(
                    Arg::new("retry")
                        .short('r')
                        .long("retry")
                        .help("try to crawl pages that have failed previously")
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help("crawl all pages even if they have been crawled before")
                )
                .arg(
                    Arg::new("debug")
                        .short('d')
                        .long("debug")
                        .help("print some debug info while crawling")
                )
        )
        .subcommand(
            Command::new("summary")
                .about("Print summary of collected data")
        )
        .subcommand(
            Command::new("xml")
                .about("Convert articles from JSON to XML")
                .arg(
                    Arg::new("override")
                        .short('o')
                        .long("override")
                        .help("override existing xml files")
                )
        )
        .subcommand(
            Command::new("build-index")
                .about("Compile an index of the crawled data based on the downloaded files")
                .arg(
                    Arg::new("out")
                        .long("out")
                        .takes_value(true)
                        .value_name("OUT")
                        .default_value("crawled_pages_from_files.json")
                        .help(&*format!("name of the output file (will be stored in '{}')", DATADIR))
                )
        )
        .get_matches();
    let command = match matches.subcommand() {
        Some(("crawl", sub_m)) => {
            Cmd::Crawl {
                force: sub_m.is_present("force"),
                retry: sub_m.is_present("retry"),
                debug: sub_m.is_present("debug"),
            }
        },
        Some(("summary", _)) => Cmd::Summary,
        Some(("xml", sub_m)) => {
            Cmd::Xml {
                r#override: sub_m.is_present("override"),
            }
        },
        Some(("build-index", sub_m)) => {
            let mut out = PathBuf::from(DATADIR);
            out.push(sub_m.value_of("out").unwrap());
            Cmd::BuildIndex { out }
        },
        _ => { unreachable!() }
    };
    Args { command }
}

#[derive(Debug)]
struct Args {
    command: Cmd,
}

#[derive(Debug)]
enum Cmd {
    Crawl {
        retry: bool,
        force: bool,
        debug: bool,
    },
    Summary,
    Xml {
        r#override: bool,
    },
    BuildIndex {
        out: PathBuf,
    },
}
// -------------------------------------------------------------------------------
//  Parser for article listings
// -------------------------------------------------------------------------------
//
/// Parser for 'nyheter' article listing pages.
pub struct SvtParser {
    http_client: reqwest::blocking::Client,
    debug: bool,
}

const API_URL: &str = "https://api.svt.se/nss-api/page/";
const ARTICLE_URL: &str = "https://api.svt.se/nss-api/page{}?q=articles";
const LIMIT: u32 = 50;
const LIMIT_STR: &str = "50";

lazy_static! {
    static ref LOCAL: Vec<&'static str> = vec![
       "blekinge",
       "dalarna",
       "gavleborg",
       "halland",
       "helsingborg",
       "jamtland",
       "jonkoping",
       "norrbotten",
       "skane",
       "smaland",
       "stockholm",
       "sodertalje",
       "sormland",
       "uppsala",
       "varmland",
       "vast",
       "vasterbotten",
       "vasternorrland",
       "vastmanland",
       "orebro",
       "ost",
    ];
    static ref TOPICS: Vec<String> = {
        let mut topics: Vec<String> = vec![
            String::from("nyheter/ekonomi"),
            String::from("nyheter/granskning"),
            String::from("nyheter/inrikes"),
            String::from("nyheter/svtforum"),
            String::from("nyheter/nyhetstecken"),
            String::from("nyheter/vetenskap"),
            String::from("nyheter/konsument"),
            String::from("nyheter/utrikes"),
            String::from("sport"),
            String::from("vader"),
            String::from("kultur"),
        ];
        for area in LOCAL.iter() {
            topics.push(format!("nyheter/lokalt/{}", area));
        }
        topics
    };
}


impl SvtParser {
    pub fn new(debug: bool) -> Self {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .expect("svt_parser: creating http_client");
//         self.get_crawled_data()
        Self { http_client, debug }
    }
//     def get_crawled_data(self):
//         """Get list of crawled URLs from CRAWLED file."""
//         self.crawled_data = dict()
//         self.saved_urls = set()
//         if CRAWLED.is_file():
//             with open(CRAWLED) as f:
//                 self.crawled_data = json.load(f)
//                 self.saved_urls = set(self.crawled_data.keys())
//
//         # Keep track of articles that could not be downloaded
//         self.failed_urls = []
//         if FAILED.is_file():
//             with open(FAILED) as f:
//                 self.failed_urls = json.load(f)
//
    /// Get all article URLs from a certain topic from the SVT API.
    pub fn crawl(&self, force: bool) {
        eprintln!("SvtParser.crawl(force={force}) called");
//         self.query_params = {"q": "auto", "limit": self.LIMIT, "page": 1}
        for topic in TOPICS.iter() {
//             topic_name = topic
            let topic_name = if topic.contains('/') {
                topic.clone()
            } else {
                topic.split('/').last().unwrap().to_string()
            };
            let topic_url = format!("{}{}/", API_URL, topic);
            let response = self.http_client
                .get(&topic_url)
                .query(&[("q", "auto"), ("limit", LIMIT_STR), ("page", "1")])
                .send()
                .expect("svt_parser: get first page");
            let firstpage: Page = response.json().expect("crawl: deserialize");
//             items = firstpage.get("auto", {}).get("pagination", {}).get("totalAvailableItems", 0)
            let pages = firstpage.auto.pagination.total_available_items / LIMIT;
            println!(
                "\nCrawling {}: {} items, {} pages",
                topic,
                firstpage.auto.pagination.total_available_items,
                pages,
            );
            self.get_urls(topic_name, topic_url, pages, firstpage, force)
        }
    }

    /// Get article URLs from every page.
    fn get_urls(&self, topic_name: String, topic_url: String, pages: u32, firstpage: Page, force: bool) {
        eprintln!("SvtParser.get_urls(topic_url={topic_url}) called");
//
//     def get_urls(self, topic_name, topic_url, pages, firstpage, request, force=False):
//         prev_crawled = len(self.saved_urls)
        let mut done = false;
        for i in 1..=pages {
            if done {
                break;
            }
//
//             self.query_params["page"] = i
//             encoded_params = ",".join(f"{k}={v}" for k, v in self.query_params.items())
//             pagecontent = []
//             try:
            let pagecontent = if i == 1 {
                firstpage.auto.content
            } else {
                let response = self.http_client.get(&topic_url)
                    .query(&[("q", "auto"), ("limit", LIMIT_STR), ("page", &*format!("{}", i))])
                    .send()
                    .expect("get_urls: send request");
                let page: Page = response.json()
                    .expect("get_urls: deserialize json");
                page.auto.content
            };
//                     pagecontent = request.json().get("auto", {}).get("content", {})
//                     if request.url in self.failed_urls:
//                         self.remove_from_failed(request.url)
//             except Exception:
//                 tb = traceback.format_exc().replace("\n", "\n  ")
//                 if self.debug:
//                     print(f"  Error when parsing listing '{request.url}'\n  {tb}")
//                 self.add_to_failed(request.url)
//
//             for c in pagecontent:
//                 short_url = c.get("url", "")
//                 if short_url.startswith("https://www.svt.se"):
//                     short_url = short_url[18:]
//                 if short_url:
//                     # Stop crawling pages when reaching an article that has already been processed
//                     # (this should work because pages are sorted by publication date)
//                     if not force and short_url in self.saved_urls:
//                         if self.debug:
//                             print(f"  Article already saved, skipping remaining. Date: {c.get('published', None)}")
//                         done = True
//                         break
//
//                     # Save article
//                     succeeded = self.get_article(short_url, topic_name, force)
//                     if succeeded:
//                         self.remove_from_failed(short_url)
//                     else:
//                         self.add_to_failed(short_url)
//
//             write_json(self.failed_urls, FAILED)
//             if len(self.saved_urls) > prev_crawled:
//                 write_json(self.crawled_data, CRAWLED)
//                 prev_crawled = len(self.saved_urls)
        }
    }
//
//     def get_article(self, short_url, topic_name, force=False):
//         """Get the content from the article URL and save as json."""
//         # Check if article has been downloaded already
//         if short_url.startswith("https://www.svt.se"):
//             short_url = short_url[18:]
//         if short_url in self.saved_urls and not force:
//             return True
//
//         article_url = self.ARTICLE_URL.format(short_url)
//         if self.debug:
//             print(f"  New article: {article_url}")
//         try:
//             article_json = requests.get(article_url).json().get("articles", {}).get("content", [])
//
//             if len(article_json) == 0:
//                 if self.debug:
//                     print(f"  No data found in article '{article_url}'")
//                 return False
//
//             if len(article_json) > 1:
//                 print(f"  Found article with multiple content entries: {short_url}")
//
//             article_id = str(article_json[0].get("id"))
//
//             year = 0
//             if article_json[0].get("published"):
//                 year = int(article_json[0].get("published")[:4])
//             elif article_json[0].get("modified"):
//                 year = int(article_json[0].get("modified")[:4])
//
//             # If year is out of range, put article in nodate folder
//             this_year = int(datetime.today().strftime("%Y"))
//             if (year < 2004) or (year > this_year):
//                 year = "nodate"
//
//             filepath = DATADIR / Path("svt-" + str(year)) / topic_name / Path(article_id + ".json")
//             write_json(article_json, filepath)
//
//             self.crawled_data[short_url] = [article_id, str(year), topic_name]
//             self.saved_urls.add(short_url)
//             return True
//
//         except Exception:
//             tb = traceback.format_exc().replace("\n", "\n  ")
//             if self.debug:
//                 print(f"  Error when parsing article '{article_url}'\n  {tb}")
//             return False
//
//     def add_to_failed(self, url):
//         """Add URL to list of failed URLs."""
//         if url not in self.failed_urls:
//             self.failed_urls.append(url)
//
//     def remove_from_failed(self, url):
//         """Remove from failed URLs if present."""
//         if url in self.failed_urls:
//             self.failed_urls.remove(url)
//
//     def get_articles_summary(self):
//         """Print number of articles per topic."""
//         summary = defaultdict(int)
//         local = defaultdict(int)
//         per_year = defaultdict(int)
//         translations = {
//             "blekinge": "Blekinge",
//             "dalarna": "Dalarna",
//             "gavleborg": "Gävleborg",
//             "granskning": "uppdrag granskning",
//             "halland": "Halland",
//             "helsingborg": "Helsingborg",
//             "jamtland": "Jämtland",
//             "jonkoping": "Jönköping",
//             "norrbotten": "Norrbotten",
//             "nyhetstecken": "nyheter teckenspråk",
//             "orebro": "Örebro",
//             "ost": "Öst",
//             "skane": "Skåne",
//             "smaland": "Småland",
//             "sodertalje": "Södertälje",
//             "sormland": "Sörmland",
//             "stockholm": "Stockholm",
//             "uppsala": "Uppsala",
//             "vader": "väder",
//             "varmland": "Värmland",
//             "vast": "Väst",
//             "vasterbotten": "Västerbotten",
//             "vasternorrland": "Västernorrland",
//             "vastmanland": "Västmanland",
//         }
//         if not self.crawled_data:
//             print("No crawled data available!")
//             return
//
//         for _article_id, year, topic in self.crawled_data.values():
//             if topic in self.LOCAL:
//                 local[translations.get(topic, topic)] += 1
//             else:
//                 summary[translations.get(topic, topic)] += 1
//             per_year[year] += 1
//
//         # Count number of articles per topic
//         print("SVT nyheter")
//         total = 0
//         for topic, amount in sorted(summary.items(), key=lambda x: x[1], reverse=True):
//             total += amount
//             print(f"{topic}\t{amount}")
//         print(f"SVT nyheter totalt\t{total}")
//         print()
//
//         # Count local news separately
//         print("SVT lokalnyheter")
//         local_total = 0
//         for area, amount in sorted(list(local.items()), key=lambda x: x[1], reverse=True):
//             local_total += amount
//             total += amount
//             print(f"{area}\t{amount}")
//         print(f"Lokalnyheter totalt\t{local_total}")
//         print()
//
//         # Articles per year
//         print("SVT artiklar per år")
//         for year, n in sorted(per_year.items()):
//             print(f"{year}\t{n}")
//         print()
//
//         # Total of all news items
//         print(f"Alla nyhetsartiklar\t{total}")
//
    /// Retry crawling/downloading failed URLs.
    pub fn retry_failed(&self) {
//         if not self.failed_urls:
//             print("Can't find any URLs that failed previously")
//             return
//
//         success = set()
//         new_failed = set()
//
//         for url in self.failed_urls:
//             short_url = url
//             if short_url.startswith("https://api.svt.se/nss-api/page"):
//                 short_url = url[31:]
//             if short_url.startswith("/nyheter/lokalt"):
//                 topic_name = short_url.split("/")[3]
//             elif short_url.startswith("/nyheter"):
//                 topic_name = short_url.split("/")[2]
//             else:
//                 topic_name = short_url.split("/")[1]
//
//             # Process article listing
//             if url.startswith("https://api.svt.se/nss-api/page"):
//                 try:
//                     request = requests.get(url)
//                     pagecontent = request.json().get("auto", {}).get("content", {})
//                     for c in pagecontent:
//                         short_url = c.get("url", "")
//                         if short_url:
//                             if self.get_article(short_url, topic_name):
//                                 success.add(url)
//                             else:
//                                 new_failed.add(url)
//                     success.add(url)
//                 except Exception:
//                     tb = traceback.format_exc().replace("\n", "\n  ")
//                     if self.debug:
//                         print(f"  Error when parsing listing '{request.url}'\n  {tb}")
//                     new_failed.add(url)
//
//             # Process article
//             else:
//                 if self.get_article(url, topic_name):
//                     success.add(url)
//                 else:
//                     new_failed.add(url)
//
//         # Update fail file
//         for i in success:
//             self.remove_from_failed(i)
//         for i in new_failed:
//             self.add_to_failed(i)
//         write_json(self.failed_urls, FAILED)
//
//         # Update file with crawled data
//         write_json(self.crawled_data, CRAWLED)
    }
}
//
// #-------------------------------------------------------------------------------
// # Process JSON data
// #-------------------------------------------------------------------------------
//
// def process_articles(override_existing=False):
//     """Convert json data to Sparv-friendly XML."""
//     def write_contents(contents, contents_dir, filecounter):
//         contents += "</articles>"
//         filepath = contents_dir / (str(filecounter) + ".xml")
//         print(f"writing file {filepath}")
//         write_data(contents, filepath)
//
//     # Get previously processed data
//     processed_json = {}
//     if PROCESSED_JSON.is_file():
//         with open(PROCESSED_JSON) as f:
//             processed_json = json.load(f)
//
//     # Loop through json files and convert them to XML
//     for topicpath in sorted(DATADIR.rglob("svt-*/*")):
//         yeardir = topicpath.parts[1]
//         make_corpus_config(yeardir, Path(yeardir))
//         contents = "<articles>\n"
//         contents_dir = Path(yeardir) / "source" / topicpath.name
//         if not override_existing and list(contents_dir.glob("*.xml")):
//             filecounter = max(int(p.stem) for p in list(contents_dir.glob("*.xml"))) + 1
//         else:
//             filecounter = 1
//         for p in sorted(topicpath.rglob("./*")):
//             if p.is_file() and p.suffix == ".json":
//
//                 if not override_existing and str(p) in processed_json:
//                     print(f"Skipping {p}, already processed in {processed_json[str(p)]}")
//                     continue
//
//                 print(f"processing {p}")
//                 with open(p) as f:
//                     article_json = json.load(f)
//                     xml = process_article(article_json[0])
//                     contents += xml + "\n"
//                     processed_json[str(p)] = str(contents_dir / str(filecounter)) + ".xml"
//                     # Write files that are around 5 MB in size
//                     if len(contents.encode("utf-8")) > 5000000:
//                         write_contents(contents, contents_dir, filecounter)
//                         contents = "<articles>\n"
//                         filecounter += 1
//         # Write remaining contents
//         if len(contents) > 11:
//             write_contents(contents, contents_dir, filecounter)
//
//         write_json(processed_json, PROCESSED_JSON)
//
//
// def process_article(article_json):
//     """Parse JSON for one article and transform to XML"""
//     def parse_element(elem, parent):
//         xml_elem = parent
//         json_tag = elem.get("type")
//         # Skip images and videos
//         if elem.get("type") not in ["svt-image", "svt-video", "svt-scribblefeed"]:
//             if parent.text is not None:
//                 # If parent already contains text, don't override it
//                 parent.text = parent.text + " " + elem.get("content", "")
//             elif elem.get("content", "").strip():
//                 parent.text = elem.get("content", "")
//         if "children" in elem:
//             # Keep only p and h* tags (but convert h* to p), avoid nested p tags
//             # if re.match(r"p|h\d", json_tag):
//             if re.match(r"p|h\d", json_tag) and parent.tag != "p":
//                 xml_elem = etree.SubElement(parent, "p")
//             # xml_elem = etree.SubElement(parent, elem.get("type"))
//             for c in elem.get("children"):
//                 return parse_element(c, xml_elem)
//         return parent
//
//     def set_attribute(xml_elem, article_json, json_name, xml_name):
//         attr = str(article_json.get(json_name, "")).strip()
//         if attr:
//             xml_elem.set(xml_name, attr)
//
//     article = etree.Element("text")
//
//     # Set article date or omit if year is out of range
//     this_year = int(datetime.today().strftime("%Y"))
//     if article_json.get("published"):
//         year = int(article_json.get("published")[:4])
//         if (year >= 2004) and (year <= this_year):
//             article.set("date", article_json.get("published"))
//     elif article_json.get("modified"):
//         year = int(article_json.get("modified")[:4])
//         if (year >= 2004) and (year <= this_year):
//             article.set("date", article_json.get("modified"))
//
//     # Set article attributes
//     set_attribute(article, article_json, "id", "id")
//     set_attribute(article, article_json, "sectionDisplayName", "section")
//     set_attribute(article, article_json, "title", "title")
//     set_attribute(article, article_json, "subtitle", "subtitle")
//     set_attribute(article, article_json, "url", "url")
//     url = str(article_json.get("url", "")).strip()
//     if url and not url.startswith("http") or url.startswith("www"):
//         article.set("url", "https://www.svt.se" + url)
//     authors = "|".join(a.get("name", "").strip() for a in article_json.get("authors", []))
//     if authors:
//         article.set("authors", "|" + authors + "|")
//     tags = "|".join(a.get("name", "") for a in article_json.get("tags", []))
//     if tags:
//         article.set("tags", "|" + tags + "|")
//
//     # Include the title and lead in the text
//     title = etree.SubElement(article, "p")
//     title.text = article_json.get("title", "").strip()
//     title.set("type", "title")
//     if article_json.get("structuredLead"):
//         for i in article_json.get("structuredLead"):
//             p = parse_element(i, article)
//             p.set("type", "lead")
//
//     # Process body
//     if article_json.get("structuredBody"):
//         for i in article_json.get("structuredBody"):
//             try:
//                 parse_element(i, article)
//             except Exception as e:
//                 print("Something went wrong with this element:")
//                 print(json.dumps(i))
//                 print(e)
//                 exit()
//
//     # Remove empty elemets (not tested!!)
//     # https://stackoverflow.com/questions/30652470/clean-xml-remove-line-if-any-empty-tags
//     for element in article.xpath(".//*[not(node())]"):
//         element.getparent().remove(element)
//     # Stringify tree
//     contents = etree.tostring(article, encoding="utf-8").decode("utf-8")
//     # Replace non-breaking spaces with ordinary spaces
//     contents = contents.replace(u"\xa0", u" ")
//     return contents
//
//
// def crawled_data_from_files(outfile):
//     """Compile an index of the crawled data based on downloaded files."""
//     crawled_data = {}
//     for jsonpath in DATADIR.rglob("svt-*/*/*.json"):
//         year = jsonpath.parts[1][4:]
//         topic = jsonpath.parts[2]
//         article_id = jsonpath.stem
//
//         with open(jsonpath) as f:
//             article_json = json.load(f)[0]
//             url = article_json.get("url")
//             crawled_data[url] = [article_id, year, topic]
//
//     write_json(crawled_data, DATADIR / outfile)
//     print(f"Done writing index of crawled data to '{DATADIR / outfile}'\n")
//
//
// #-------------------------------------------------------------------------------
// # Auxiliaries
// #-------------------------------------------------------------------------------
//
// def write_json(data, filepath):
//     """Write json data to filepath."""
//     dirpath = filepath.parent
//     dirpath.mkdir(parents=True, exist_ok=True)
//     with open(filepath, "w") as f:
//         json.dump(data, f, ensure_ascii=False, indent=2)
//
//
// def write_data(data, filepath):
//     """Write arbitrary data to filepath."""
//     dirpath = filepath.parent
//     dirpath.mkdir(parents=True, exist_ok=True)
//     with open(filepath, "w") as f:
//         f.write(data)
//
//
// def make_corpus_config(corpus_id, path):
//     """Write Sparv corpus config file for sub corpus."""
//     config_file = path / "config.yaml"
//     path.mkdir(parents=True, exist_ok=True)
//     year = corpus_id.split("-")[-1]
//     config_content = (
//         "parent: ../config.yaml\n"
//         "\n"
//         "metadata:\n"
//         f"  id: {corpus_id}\n"
//         "  name:\n"
//         f"    eng: SVT news {year if year != 'nodate' else 'okänt datum'}\n"
//         f"    swe: SVT nyheter {year if year != 'nodate' else 'unknown date'}\n"
//     )
//     with open(config_file, "w") as f:
//         f.write(config_content)
//     print(f"{config_file} written")
//
//
// #-------------------------------------------------------------------------------
fn main() {
    // Parse command line args, print help if none are given
    let args = parse_args();
    eprintln!("args = {:?}", args);

    match args.command {
        Cmd::Crawl { retry, force, debug } => {
            if retry {
                println!("\nTrying to crawl pages that failed last time ...");
                if force {
                    println!("Argument '--force' is ignored when recrawling failed pages.");
                }
                let mut svt_parser = SvtParser::new(true);
                svt_parser.retry_failed();
            } else {
                println!("\nStarting to crawl svt.se ...");
//             time.sleep(5)
                let mut svt_parser = SvtParser::new(debug);
                svt_parser.crawl(force)
            }
        },
        Cmd::Summary => {
            println!("\nCalculating summary of collected articles ...");
//         SvtParser().get_articles_summary()
        },
        Cmd::Xml { r#override } => {
            println!("\nPreparing to convert articles to XML ...");
//         process_articles(override_existing=args.override)
        },
        Cmd::BuildIndex { out } => {
            println!("\nBuilding an index of crawled files based on the downloaded JSON files ...");
//         crawled_data_from_files(args.out)
        }
//
//
//     ## DEBUG STUFF
//
//     # SvtParser().get_article("/nyheter/inrikes/toppmote-om-arktis-i-kiruna", "inrikes")
//
//     # with open("data/svt-2020/konsument/28334881.json") as f:
//     #     article_json = json.load(f)
//     #     xml = process_article(article_json[0])
//     #     print(xml)
    }
}
