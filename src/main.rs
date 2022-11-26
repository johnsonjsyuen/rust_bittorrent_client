use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str;
use anyhow::{anyhow, Error};
use anyhow::Result;
use lava_torrent::torrent::v1::Torrent;
use urlencoding::{decode, encode, encode_binary};
use serde::{Deserialize, Deserializer};
use reqwest::{Client, Url};
use sha1::{Digest, Sha1};
use lava_torrent::bencode::BencodeElem;
use tokio_byteorder::{BigEndian, AsyncReadBytesExt};
use std::io::Cursor;

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
    peers: Vec<SocketAddrV4>,
}

async fn get_peer_list(torrent: Torrent, client: Client) -> Result<BencodeTrackerResponse> {
    let encoded_info_hash = torrent.clone().info_hash();
    println!("Info hash {}", encoded_info_hash.clone());
    println!("URL encoded Info hash {}", encode(&*encoded_info_hash.clone()));

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
    let url = format!("{}&info_hash=%9D%CC%EDN%3F%C4%97S%88%8FY%D5Y%CA%D9%EA%9D%D9%9E%A5", url);
    println!("{}", url.clone());
    let resp = client.get(url)
        .send()
        .await?;
    let dd = BencodeElem::from_bytes(resp.bytes().await.unwrap()).unwrap();
    let decoded_response =
        dd.get(0).unwrap();

    if let BencodeElem::Dictionary(d)  = decoded_response{
        println!("{:?}", decoded_response);
        let peers =
            if let BencodeElem::Bytes(b) = d.get("peers").unwrap(){
                b
            }else{
                panic!()
            };
        let interval: usize =
            if let BencodeElem::Integer(i) = d.get("interval").unwrap(){
                *i as usize
            }else{
                900
            };
        Ok(BencodeTrackerResponse {
            interval: interval,
            peers: unmarshal_peers(peers).await?,
        })
    }else{
        return Err(anyhow!("Invalid peers response"));
    }

}

async fn unmarshal_peers(peers_bin: &[u8]) -> Result<Vec<SocketAddrV4>> {
    const peersize: usize = 6;// 4 bytes for IP, 2 for port
    if peers_bin.len() % peersize != 0 {
        return Err(anyhow!("Invalid peers response. Size is not multiple of 6"));
    }
    let mut peers = Vec::new();
    let num_peers = peers_bin.len() / peersize;
    for num_peer in 0..num_peers {
        let offset = num_peer * peersize;
        peers.push(bytes_to_addr(&peers_bin[offset..offset+5]).await)
    };
    Ok(peers)
}

async fn bytes_to_addr(p: &[u8]) -> SocketAddrV4 {
    let ip = Ipv4Addr::new(p[0], p[1], p[2], p[3]);
    let mut rdr =  Cursor::new(&p[4..]);
    SocketAddrV4::new(ip, rdr.read_u16::<BigEndian>().await.unwrap())
}