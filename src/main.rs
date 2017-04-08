extern crate hyper;
extern crate hyper_native_tls;
extern crate pbr;

use std::env;
use pbr::ProgressBar;
use std::str;
use std::collections::HashMap;
use hyper::client::response::Response;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use std::io::Read;
use std::io::prelude::*;
use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();
    let url = format!("https://youtube.com/get_video_info?video_id={}", args[1]);
    let mut response = send_request(&url);
    let mut response_str = String::new();
    response.read_to_string(&mut response_str).unwrap();
    let hq = parse_url(&response_str);

    if hq["status"] != "ok" {
        println!("Video not found!");
        return;
    }

    // get video info
    let streams: Vec<&str> = hq.get("url_encoded_fmt_stream_map")
        .unwrap()
        .split(",")
        .collect();

    // get video title
    let title = hq.get("title").unwrap();

    let mut i = 0;

    // list of available qualities
    let mut qualities: HashMap<i32, (String, String)> = HashMap::new();
    for url in streams.iter() {
        i += 1;
        let quality = parse_url(&url);
        let extension = quality.get("type")
            .unwrap()
            .split("/")
            .nth(1)
            .unwrap()
            .split(";")
            .next()
            .unwrap();
        qualities.insert(i,
                         (quality.get("url").unwrap().to_string(), extension.to_owned()));
        println!("{}- {} {}",
                 i,
                 quality.get("quality").unwrap(),
                 quality.get("type").unwrap());
    }

    println!("Choose quality: ");
    let input = read_line().trim().parse().unwrap();

    println!("Please wait...");

    let url = &qualities.get(&input).unwrap().0;
    let extension = &qualities.get(&input).unwrap().1;

    // get response from selected quality
    let mut response = send_request(url);
    println!("Download is starting...");

    // get headers
    let headers = std::mem::replace(&mut response.headers, hyper::header::Headers::new());

    // get file size from Content-Length header
    let content_length_header = headers.get_raw("Content-Length").unwrap();
    let file_size = str::from_utf8(&content_length_header[0])
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let filename = format!("{}.{}", title, extension);

    // write file to disk
    write_file(response, &filename, file_size);
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
    match client.get(url).send() {
        Ok(response) => response,
        Err(why) => panic!("{}", why),
    }
}


fn parse_url(query: &str) -> HashMap<String, String> {
    let u = format!("{}{}", "http://e.com?", query);
    let parsed_url = hyper::Url::parse(&u).unwrap();
    parsed_url.query_pairs().into_owned().collect()
}


fn read_line() -> String {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Could not read stdin!");
    input
}
