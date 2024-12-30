mod workers;

use std::collections::HashMap;
use base64::prelude::*;
use std::{fs, io};
use std::io::{BufRead, BufReader, Write};
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
                Err(_) => {
                    println!("error occurred when handling connection");
                }
            }
        });
    }
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let buf_reader = BufReader::new(&stream);
    let lines: Vec<_> = buf_reader.lines().collect::<Result<_, _>>().unwrap();

    let header = &lines[0];
    let host = &lines[1].replace("Host: ","");

    println!("{} {}", header, host);

    stream.write("dsdf".as_bytes())?;
    stream.flush()
}