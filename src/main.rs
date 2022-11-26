use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str;
use lava_torrent::torrent::v1::Torrent;
use urlencoding::{decode, encode, encode_binary};
use serde::{Deserialize, Deserializer};
use reqwest::{Client, Url};
use sha1::{Digest, Sha1};
use lava_torrent::bencode::BencodeElem;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let file_path = "ubuntu.torrent";
    let client = reqwest::Client::new();
    let torrent = Torrent::read_from_file(file_path).unwrap();

    get_peer_list(torrent, client).await?;
    Ok(())
}
#[derive(Debug, Deserialize)]
struct BencodeTrackerResponse {
    interval: usize,
    peers: Vec<SocketAddrV4>
}

async fn get_peer_list(torrent: Torrent, client: Client) -> Result<BencodeTrackerResponse, anyhow::Error>{
    let encoded_info_hash = torrent.clone().info_hash();
    //println!("{}", encode(Sha1::digest(torrent.clone().info_hash_bytes())));

    let params = [
        ("peer_id", "ABCDEFGHIJKLMNOPQRST"),
        ("port", "6881"),
        ("uploaded", "0"),
        ("downloaded", "0"),
        ("compact", "1"),
        ("left", &torrent.length.to_string())];

    let url = Url::parse_with_params(&*torrent.announce.unwrap(),
                                     &params)?;

    // parse_with_params does it's own encoding so we are encoding the info hash part separately
    let url = format!("{}&info_hash=%9D%CC%EDN%3F%C4%97S%88%8FY%D5Y%CA%D9%EA%9D%D9%9E%A5",url);
    println!("{}", url.clone());
    let resp = client.get(url)
        .send()
        .await?;
    let decoded_response = BencodeElem::from_bytes(resp.bytes().await.unwrap()).unwrap();
    //println!("{:?}", resp.text().await);
    println!("{:?}", decoded_response);
    Ok(BencodeTrackerResponse{
        interval: 100,
        peers: vec![]
    })
}