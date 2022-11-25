use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::str;
use bencode::{BEncode, decode_buf_first};

fn main() {
    let file_path = "ubuntu.torrent";
    let contents = fs::read(file_path)
        .expect("Should have been able to read the file");

    let b = decode_buf_first(&contents).unwrap();
    let mut m:HashMap<String,BEncode> = HashMap::new();
    match b.clone() {
        BEncode::Int(a) => {println!("Int {:?}",a)}
        BEncode::String(a) => {println!("String {:?}",a)}
        BEncode::List(a) => {println!("List {:?}",a.first().unwrap())}
        BEncode::Dict(a) => {

            for (k, v) in a {
                m.insert(String::from_utf8(k.clone()).unwrap(),v);
            }
        }
    }
    println!("Decoded {:?}",m.keys());
    println!("Decoded {:?}",m.get("announce").unwrap().as_str().unwrap())


}