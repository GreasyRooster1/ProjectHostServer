#[macro_use]
extern crate rouille;

use rs_firebase_admin_sdk::{
    auth::{FirebaseAuthService, UserIdentifiers},
    client::ApiHttpClient,
    App, credentials_provider,
};

use std::fs;
use rouille::{extension_to_mime, Request, Response};
use std::fs::File;
use std::io::{BufRead, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use rs_firebase_admin_sdk::auth::token::TokenVerifier;
use futures::executor::block_on;

pub const THREAD_POOL_SIZE:usize = 64;
pub const NOT_FOUND_PAGE:&str = include_str!("../404.html");

pub const HOST_IP:&str = "0.0.0.0";
pub const HOST_PORT:&str = "1312";
pub const BLOCK_INDEXING:bool = true;

pub const WHITELIST_EXTENSIONS: [&str;16] = ["png","jpg","wav","mp3","html","css","js","jsx","ts","tsx","jpeg","webp","txt","csv","json","http"];
pub const BLACKLIST_HOSTS: [&str;1] = ["code"];

#[tokio::main]
async fn main() {
    let gcp_service_account = credentials_provider().await.unwrap();
    // Create live (not emulated) context for Firebase app
    let live_app = App::live(gcp_service_account.into()).await.unwrap();
    let auth_admin = live_app.auth();
    let live_token_verifier = live_app.id_token_verifier().await.unwrap();

    let address = format!("{HOST_IP}:{HOST_PORT}");
    println!("Now listening on {address}");

    let cert = include_str!("../cert/cacert.pem").as_bytes().to_vec();
    let pkey = include_str!("../cert/cakey.pem").as_bytes().to_vec();
    let token = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjkwOTg1NzhjNDg4MWRjMDVlYmYxOWExNWJhMjJkOGZkMWFiMzRjOGEiLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL3NlY3VyZXRva2VuLmdvb2dsZS5jb20vcWNvZGUtY2RmYzYiLCJhdWQiOiJxY29kZS1jZGZjNiIsImF1dGhfdGltZSI6MTc0MTU1NTM0NywidXNlcl9pZCI6ImpFTWgza2VtM2VNUVZsaHl2ODZsaGl2dDNTSDMiLCJzdWIiOiJqRU1oM2tlbTNlTVFWbGh5djg2bGhpdnQzU0gzIiwiaWF0IjoxNzQ1NjkwODU4LCJleHAiOjE3NDU2OTQ0NTgsImVtYWlsIjoiZ3JlYXN5cm9vc3RlcjFAZ21haWwuY29tIiwiZW1haWxfdmVyaWZpZWQiOmZhbHNlLCJmaXJlYmFzZSI6eyJpZGVudGl0aWVzIjp7ImVtYWlsIjpbImdyZWFzeXJvb3N0ZXIxQGdtYWlsLmNvbSJdfSwic2lnbl9pbl9wcm92aWRlciI6InBhc3N3b3JkIn19.FTkmJBpl8DvKyR4BRf3d7-sVzBfzrcRI6gflQlafIhPMfBf2If8DV3TzLfIeaoqLOkOhfh_qE4MaHa-RagFsywY9AJjBR0TTJ2hYLnTxOi2ShkKZfnsV7OIQy32aK3_ln2ihzHJan5pKyapNfwZGR7IS1RR8kMfrRGEvL-5-bonHB_0Z3QCA-el6spfXRQIpKY5kgNRt4biTRc6skAET1ZYm-91YT_GlgCqdTA2GA-c2rYPUUusANW-TXL1_o2FHEs6iNqai_STX15Q2Sqz0XlLlngTg-CgPGQPexBN1EDw_8FPfgoCJhkHdy2zSPFrkPysZiMlTsym7wUVKJDdbKQ";

    rouille::start_server(address, move |request| {
        router!(request,
            (GET) (/) => {
                resolve_uri(request, "index.html".to_string())
            },

            (GET) (/stats) => {
                panic!("Not implemented yet")
            },

            (GET) (/{uri: String}) => {
                resolve_uri(request, uri)
            },

            (PUT) (/{uri: String}) => {

                block_on(verify_token(token, &live_token_verifier));

                let host = request.header("Host").unwrap();
                let path = get_path_from_host(host.to_string(),uri).unwrap();
                let mut buffer = String::new();
                let pathObj = Path::new(&path);
                let extension = pathObj.extension().unwrap().to_str().unwrap();

                if !WHITELIST_EXTENSIONS.contains(&extension){
                    return rouille::Response::text("forbidden extension").with_status_code(403);
                }

                request.data().unwrap().read_to_string(&mut buffer).expect("couldnt read body");
                fs::create_dir_all(pathObj.parent().unwrap()).expect("failed to make dirs");
                let mut file = File::create(&path).unwrap();
                file.write_all(buffer.as_bytes()).unwrap();

                println!("Wrote to path: {:?}", path);

                rouille::Response::empty_204()
            },
            _ => rouille::Response::empty_404()
        )
    });//,cert,pkey).unwrap().run();
}

fn resolve_uri(request: &Request,uri:String)->Response{
    let host = request.header("Host").unwrap();
    let words: Vec<_> = host.split(".").collect();
    println!("from host: {host}");
    let path = get_path_from_host(words[0] .to_string(),uri).unwrap();
    println!("Requested path: {:?}", path);
    let contents = match File::open(&path) {
        Ok(c) => c,
        Err(_) => {
            return Response::from_data("text/html", NOT_FOUND_PAGE).with_unique_header("X-Robots-Tag","no-index")
        }
    };

    Response::from_file(extension_to_mime(path.as_str()),contents).with_unique_header("X-Robots-Tag","no-index")
}

fn get_path_from_host(host:String,uri:String)->Result<String,String>{
    let path = PathBuf::from_str(format!("./data/{0}/{uri}",host).as_str()).unwrap();
    if path.components().any(|x| x == Component::ParentDir) {
        return Err("directory traversal".to_string());
    }
    Ok(path.as_path().to_str().unwrap().to_string())
}

async fn verify_token<T: TokenVerifier>(token: &str, verifier: &T) {
    match verifier.verify_token(token).await {
        Ok(token) => {
            let user_id = token.critical_claims.sub;
            println!("Token for user {user_id} is valid!")
        }
        Err(err) => {
            println!("Token is invalid because {err}!")
        }
    }
}