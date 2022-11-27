mod protocol;



use std::net::{Ipv4Addr, SocketAddrV4};
use std::str;
use anyhow::{anyhow};
use anyhow::Result;
use lava_torrent::torrent::v1::Torrent;
use urlencoding::{decode, encode, encode_binary};
use serde::{Deserialize};
use reqwest::{Client, Url};
use lava_torrent::bencode::BencodeElem;
use tokio_byteorder::{BigEndian, AsyncReadBytesExt};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream};

use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use crate::protocol::Message;
use crate::protocol::Message::Handshake;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref PEER_ID: [u8; 20] = {
        use rand::Rng;

        let mut pid = [0u8; 20];
        let prefix = b"-SY0010-";
        pid[..prefix.len()].clone_from_slice(&prefix[..]);

        let mut rng = rand::thread_rng();
        for p in pid.iter_mut().skip(prefix.len()) {
            *p = rng.gen();
        }
        pid
    };
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let file_path = "ubuntu.torrent";
    let client = Client::new();
    let torrent = Torrent::read_from_file(file_path).unwrap();

    let peer_list = get_peer_list(torrent, client).await?;
    println!("Peer List: {:?}", peer_list.clone());
    Ok(())
}

#[derive(Debug, Deserialize, Clone)]
struct BencodeTrackerResponse {
    interval: usize,
    peers: Vec<SocketAddrV4>,
}

async fn get_peer_list(torrent: Torrent, client: Client) -> Result<BencodeTrackerResponse> {
    let info_hash_bytes = torrent.clone().info_hash_bytes();
    let percent_encoded_info_hash = percent_encode(&*info_hash_bytes.clone(), NON_ALPHANUMERIC).to_string();
    println!("Info hash {:?}", info_hash_bytes.clone());
    println!("URL encoded Info hash {}", percent_encoded_info_hash.clone());

    let ss = percent_encoded_info_hash.clone();
    let params = [
        ("peer_id", str::from_utf8(&*PEER_ID).unwrap()),
        ("port", "6881"),
        ("uploaded", "0"),
        ("downloaded", "0"),
        ("compact", "1"),
        ("left", &torrent.length.to_string())];

    let url = Url::parse_with_params(&*torrent.announce.unwrap(),
                                     &params)?;

    // parse_with_params does it's own encoding so we are encoding the info hash part separately
    let url = format!("{}&info_hash={}", url, ss.as_str());
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

async fn handshake_with_peer(peer: Peer) -> Result<()>{
    // Connect to a peer
    let mut stream = TcpStream::connect(peer.socket).await?;
    let handshake = Message::handshake(&PEER_ID, &peer.info_hash);
    let mut buf = [0u8; 68];
    handshake.encode(&mut buf).expect("TODO: panic message");
    stream.write_all(&buf).await?;

    Ok(())
}

struct Peer {
    socket: SocketAddrV4,
    info_hash: [u8; 20],
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
    let mut rdr =  Cursor::new(&p[3..]);
    SocketAddrV4::new(ip, rdr.read_u16::<BigEndian>().await.unwrap())
}