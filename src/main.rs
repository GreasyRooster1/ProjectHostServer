#[macro_use]
extern crate rouille;

use rouille::extension_to_mime;
use std::fs::File;
use std::io::{BufRead, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

pub const THREAD_POOL_SIZE:usize = 64;
pub const NOT_FOUND_PAGE:&str = include_str!("../404.html");

pub const HOST_IP:&str = "127.0.0.1";
pub const HOST_PORT:&str = "1313";
pub const BLOCK_INDEXING:bool = true;

pub const WHITELIST_EXTENSIONS: [&str;16] = ["png","jpg","wav","mp3","html","css","js","jsx","ts","tsx","jpeg","webp","txt","csv","json","http"];


fn main() {
    let address = format!("{HOST_IP}:{HOST_PORT}");
    println!("Now listening on {address}");

    let cert = include_str!("/cert/server.crt").as_bytes().to_vec();
    let pkey = include_str!("/cert/server.key").as_bytes().to_vec();

    rouille::Server::new_ssl(address, move |request| {
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
                println!("Requested path: {:?}, from host: {host}", path);
                let contents = File::open(&path).unwrap();

                rouille::Response::from_file(extension_to_mime(path.as_str()),contents).with_unique_header("X-Robots-Tag","no-index")
            },

            (PUT) (/{uri: String}) => {
                let host = request.header("Host").unwrap();
                let path = get_path_from_host(host.to_string(),uri).unwrap();
                let mut buffer = String::new();
                let extension = Path::new(&path).extension().unwrap().to_str().unwrap();

                if !WHITELIST_EXTENSIONS.contains(&extension){
                    return rouille::Response::text("forbidden extension").with_status_code(403);
                }

                request.data().unwrap().read_to_string(&mut buffer).expect("couldnt read body");
                let mut file = File::create(&path).unwrap();
                file.write_all(buffer.as_bytes()).unwrap();

                println!("Wrote to path: {:?}", path);

                rouille::Response::empty_204()
            },
            _ => rouille::Response::empty_404()
        )
    },cert,pkey).unwrap().run();
}

fn get_path_from_host(host:String,uri:String)->Result<String,String>{
    let host_words:Vec<&str> = host.split(".").collect();
    let path = PathBuf::from_str(format!("./data/{1}/{0}/{uri}",host_words[0],host_words[1]).as_str()).unwrap();
    if path.components().any(|x| x == Component::ParentDir) {
        return Err("directory traversal".to_string());
    }
    Ok(path.as_path().to_str().unwrap().to_string())
}

