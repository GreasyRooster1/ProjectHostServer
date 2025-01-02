mod workers;

use std::collections::HashMap;
use base64::prelude::*;
use std::{fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, Error, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use httpcodec::{BodyDecoder, ResponseDecoder};
use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::io::IoDecodeExt;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use serde_json::Value;
use crate::workers::ThreadPool;
#[macro_use]
extern crate rouille;

pub const THREAD_POOL_SIZE:usize = 64;
pub const NOT_FOUND_PAGE:&str = include_str!("../404.html");

pub const HOST_IP:&str = "127.0.0.1";
pub const HOST_PORT:&str = "1313";
pub const BLOCK_INDEXING:bool = true;

fn main() {
    let address = format!("{HOST_IP}:{HOST_PORT}");
    println!("Now listening on {address}");
    rouille::start_server(address, move |request| {
        router!(request,
            (GET) (/) => {
                rouille::Response::redirect_302("/index.html")
            },

            (GET) (/stats) => {
                panic!("Not implemented yet")
            },

            (GET) (/{uri: String}) => {
                let host = request.header("Host").unwrap();
                let path = get_path_from_host(host.to_string(),uri).unwrap();
                println!("Requested path: {:?}", path);
                let contents = File::open(&path).unwrap();

                rouille::Response::from_file(get_mime_type(path.as_str()),contents)
            },

            (PUT) (/{uri: String}) => {
                let host = request.header("Host").unwrap();
                let path = get_path_from_host(host.to_string(),uri).unwrap();
                let mut buffer = String::new();

                request.data().unwrap().read_to_string(&mut buffer).expect("couldnt read body");
                let mut file = File::create(&path).unwrap();
                file.write_all(buffer.as_bytes()).unwrap();

                println!("Wrote to path: {:?}", path);

                rouille::Response::empty_204()
            },
            _ => rouille::Response::empty_404()
        )
    });
}

pub(crate) fn get_mime_type(path:&str)->String{
    match path.split(".").last().unwrap() {
        "html"=>{"text/html".to_string()}
        "css"=>{"text/css".to_string()}
        "js"=>{"text/javascript".to_string()}
        "mjs"=>{"text/javascript".to_string()}
        "ico"=>{"image/vnd.microsoft.icon".to_string()}
        "png"=>{"image/png".to_string()}
        "jpg"=>{"image/jpeg".to_string()}
        _ => {"".to_string()}
    }
}

fn get_path_from_host(host:String,uri:String)->Result<String,String>{
    let host_words:Vec<&str> = host.split(".").collect();
    let path = PathBuf::from_str(format!("./data/{1}/{0}/{uri}",host_words[0],host_words[1]).as_str()).unwrap();
    if path.components().any(|x| x == Component::ParentDir) {
        return Err("directory traversal".to_string());
    }
    Ok(path.as_path().to_str().unwrap().to_string())
}

