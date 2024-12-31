mod workers;

use std::collections::HashMap;
use base64::prelude::*;
use std::{fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use httpcodec::{BodyDecoder, ResponseDecoder};
use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::io::IoDecodeExt;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use serde_json::Value;
use crate::workers::ThreadPool;

pub const THREAD_POOL_SIZE:usize = 64;
pub const NOT_FOUND_PAGE:&str = include_str!("../404.html");

pub const HOST_IP:&str = "127.0.0.1";
pub const HOST_PORT:&str = "1313";


fn main() {
    let listener = TcpListener::bind(HOST_IP.to_owned()+":"+HOST_PORT).unwrap();
    let pool = ThreadPool::new(THREAD_POOL_SIZE);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(_) => {
                stream.unwrap()
            }
            Err(_) => {
                println!("error occurred when unwrapping stream");
                continue;
            }
        };
        pool.execute( || {
            match handle_connection(stream){
                Ok(_) => {}
                Err(err) => {
                    println!("{}", err);
                }
            }
        });
    }
}

pub(crate) fn extract_uri(http_request: &str) -> &str {
    let line = http_request.lines().next().unwrap();
    // return uri (remove GET prefix and HTTP/1.1 suffix)
    line.strip_prefix("GET")
        .unwrap()
        .strip_suffix("HTTP/1.1")
        .unwrap()
        .trim()
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

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buf_reader = BufReader::new(&stream);
    let mut lines = buf_reader.lines();

    let header = lines.next().unwrap()?;
    let host = lines.next().unwrap()?.replace("Host: ","");

    loop {
        let line=lines.next().unwrap()?;
        if line.is_empty(){
            break
        }
    }
    let mut body = "".to_string();
    for read_line in lines {
        let line = read_line.unwrap();
        if(line==)
        body = format!("{body}\n{line}");
    }
    println!("{:#?}",body);

    let uri = extract_uri(header.as_str());

    let response = respond(header.to_string(),host.to_string(),uri.to_string(),"str".to_string());

    let (status_line,content) = match response {
        Ok(content) => {
            ("HTTP/1.1 200 OK",content)
        }
        Err(err) => {
            ("HTTP/1.1 400 Bad Request",err)
        }
    };

    let binding = get_mime_type(uri);
    let mime = binding.as_str();
    let length = content.len();

    let header = format!("{status_line}\r\nContent-Type: {mime}\r\nContent-Length: {length}\r\n\r\n");

    stream.write(header.as_bytes())?;
    stream.write(content.as_bytes())?;
    stream.flush()
}

fn respond(req_header:String,host:String,uri:String,body:String) -> Result<String,String>{
    if req_header.starts_with("GET"){
       return respond_get(req_header,host,uri);
    }
    if req_header.starts_with("POST"){
        return respond_post(req_header,host,uri,body);
    }
    return Err("Incorrect protocol".to_string());
}

fn respond_post(req_header:String,host:String,uri:String,body:String)-> Result<String,String> {
    let host_words:Vec<&str> = host.split(".").collect();
    if host_words.len()!=4 {
        return Err("Malformed host".to_string())
    }
    let path = get_path_from_host(host,uri);
    return match File::open(path) {
        Ok(mut file) => {
            match file.write_all(body.as_bytes()) {
                Ok(_) => {
                    Ok("received file".to_string())
                }
                Err(err) => {
                    Err(format!("{}", err))
                }
            }
        }
        Err(err) => {
            Err(format!("{}", err))
        }
    }
}

fn respond_get(req_header:String,host:String,uri:String) -> Result<String,String> {

    let host_words:Vec<&str> = host.split(".").collect();
    if host_words.len()!=4 {
        return Err("Malformed host".to_string())
    }
    let path = get_path_from_host(host,uri);
    let contents = fs::read_to_string(path.clone());
    return match contents {
        Ok(content) => {
            Ok(content)
        }
        Err(error) => {
            Err(format!("File read error: {error}"))
        }
    }
}

fn get_path_from_host(host:String,uri:String)->String{
    let host_words:Vec<&str> = host.split(".").collect();
    format!("data/{1}/{0}{uri}",host_words[0],host_words[1])
}