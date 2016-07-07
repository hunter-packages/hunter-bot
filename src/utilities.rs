//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::io::Read;

extern crate hyper;
use hyper::Client;
use hyper::client::Body;
use hyper::client::IntoUrl;
use hyper::client::response::Response;
use hyper::header::Headers;
use hyper::Url;


////////////////////////////////////////////////////////////
//                          Funcs                         //
////////////////////////////////////////////////////////////


pub fn github_post_request(endpoint: String, body: String) -> Result<(), &'static str>{

    let     http_client   = Client::new();
    let     api_call      = format!("https://api.github.com/{}", endpoint);
    let     api_call_url:   Url;
    let mut header        = Headers::new();
    let mut response:       Response;
    let mut response_body = String::new();

    match api_call.into_url() {
        Ok(url) => api_call_url = url,
        Err(_)  => return Err("Failed to parse the API call url.")
    }

    header.set_raw("User-Agent", vec![b"hunter-bot".to_vec()]);
    let body_len = body.len().clone();
    match http_client.post(api_call_url)
        .headers(header)
        .body(Body::BufBody(&body.into_bytes()[..], body_len))
        .send() {
        Ok(res) => response = res,
        Err(_)  => return Err("Failed to  call the API.")
    }

    if response.status == hyper::status::StatusCode::Unauthorized {
        return Err("Bad Credentials")
    }

    if response.status == hyper::status::StatusCode::NotFound {
        return Err("Endpoint not found or insufficient privileges.")
    }

    match response.read_to_string(&mut response_body){
        Ok(_)  => Ok(()),
        Err(_) => Err("Failed to  convert the API response to a string.")
    }
}