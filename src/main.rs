extern crate hyper;
extern crate hyper_native_tls;
extern crate pbr;
extern crate clap;
extern crate regex;

use std::process;
use pbr::ProgressBar;
use std::{process,str};
use std::collections::HashMap;
use hyper::client::response::Response;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use hyper::header::ContentLength;
use std::io::Read;
use std::io::prelude::*;
use std::fs::File;
use clap::{Arg, App};
use regex::Regex;

fn main() {
    //Regex for youtube URLs.
    let url_regex = Regex::new(r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*").unwrap();
    let args = App::new("youtube-downloader")
        .version("0.1.0")
        .arg(Arg::with_name("video-id")
            .help("The ID of the video to download.")
            .required(true)
            .index(1))
        .arg(Arg::with_name("quality")
             .help("The index of the quality to download, 1 is always the best quality.")
             .required(false)
             .long("--quality")
             .short("-q")
             .value_name("quality")
             .takes_value(true))
        .get_matches();

    let quality = args.value_of("quality");
    let mut vid = args.value_of("video-id").unwrap();
    if url_regex.is_match(vid) {
        let vid_split = url_regex.captures(vid).unwrap();
        vid = vid_split.get(1).unwrap().as_str();
    }

    let url = format!("https://youtube.com/get_video_info?video_id={}", vid);
    download(&url);
}

fn download(url: &str, quality: Option<&str>) {
    let mut response = send_request(url);
    let mut response_str = String::new();
    response.read_to_string(&mut response_str).unwrap();
    let hq = parse_url(&response_str);

    if hq["status"] != "ok" {
        println!("Video not found!");
        process::exit(1);
    }

    // get video info
    let streams: Vec<&str> = hq["url_encoded_fmt_stream_map"]
        .split(',')
        .collect();

    // list of available qualities
    let mut qualities: HashMap<i32, (String, String)> = HashMap::new();
    for (i, url) in streams.iter().enumerate() {
        let quality = parse_url(url);
        let extension = quality["type"]
            .split('/')
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        qualities.insert(i as i32,
                         (quality["url"].to_string(), extension.to_owned()));
        println!("{}- {} {}",
                 i,
                 quality["quality"],
                 quality["type"]);
    }

    println!("Choose quality: ");
    let mut input = 0;
    let mut picked = false;
    //Check if the -q argument was passed.
    if !quality.is_some() {
        while !picked {
            input = match read_line().trim().parse() {
                Ok(num) => {
                    if num <= i && num > 0 {
                        picked = true;
                        num
                    } else {
                        println!("Please pick a number between 1 and {}", i);
                        0
                    }
                },
                Err(_) => {
                    println!("Please input a number.");
                    0
                }
            };
        }
    } else {
        input = match quality.unwrap().parse() {
            Ok(num) => {
                if num <= 1 && num > 0 {
                    num
                } else {
                        println!("Please pick a number between 1 and {}", i);
                        process::exit(1);
                }
            },
            Err(_) => {
                println!("The quality must be a number");
                process::exit(1);
            }
        };
    }

    println!("Please wait...");

    let url = &qualities[&input].0;
    let extension = &qualities[&input].1;

    // get response from selected quality
    let response = send_request(url);
    println!("Download is starting...");

    // get file size from Content-Length header
    let file_size = get_file_size(&response);

    let filename = format!("{}.{}", hq["title"], extension);

    // write file to disk
    write_file(response, &filename, file_size);
}

// get file size from Content-Length header
fn get_file_size(response: &Response) -> u64 {
    let mut file_size = 0;
    match response.headers.get::<ContentLength>(){
        Some(length) => file_size = length.0,
        None => println!("Content-Length header missing"),
    };
    file_size
}

fn write_file(mut response: Response, title: &str, file_size: u64) {
    // initialize progressbar
    let mut pb = ProgressBar::new(file_size);
    pb.format("╢▌▌░╟");

    // Download and write to file
    let mut buf = [0; 128 * 1024];
    let mut file = File::create(title).unwrap();
    loop {
        match response.read(&mut buf) {
            Ok(len) => {
                file.write_all(&buf[..len]).unwrap();
                pb.add(len as u64);
                if len == 0 {
                    break;
                }
                len
            }
            Err(why) => panic!("{}", why),
        };
    }
}

fn send_request(url: &str) -> Response {
    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);
    client.get(url).send().unwrap_or_else(|e| {
        println!("Network request failed: {}", e);
        process::exit(1);
    })
}

fn parse_url(query: &str) -> HashMap<String, String> {
    let u = format!("{}{}", "http://e.com?", query);
    let parsed_url = hyper::Url::parse(&u).unwrap();
    parsed_url.query_pairs().into_owned().collect()
}

fn read_line() -> String {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Could not read stdin!");
    input
}
