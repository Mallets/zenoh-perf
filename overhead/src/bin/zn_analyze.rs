//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//

extern crate serde;
use structopt::StructOpt;
//use zenoh::net::ResKey::*;
use async_std::fs;
use serde::{Deserialize, Serialize};
use std::io::Read;
use zenoh::net::protocol::io::ZBuf;
use zenoh::net::protocol::proto::{
    FramePayload, SessionBody, SessionMessage, ZenohBody, ZenohMessage,
};

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_analyze")]
struct Opt {
    #[structopt(short = "j", long = "json")]
    file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PcapData {
    #[serde(alias = "_index")]
    pub index: String,
    #[serde(alias = "_type")]
    pub pcap_type: String,
    #[serde(alias = "_score")]
    pub score: Option<String>,
    #[serde(alias = "_source")]
    pub source: Layers,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Layers {
    #[serde(alias = "_layers")]
    pub layers: PcapLayers,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PcapLayers {
    #[serde(alias = "frame.len")]
    pub frame_len: Option<Vec<String>>,
    #[serde(alias = "ip.len")]
    pub ip_len: Option<Vec<String>>,
    #[serde(alias = "tcp.dstport")]
    pub tcp_dest: Option<Vec<String>>,
    #[serde(alias = "tcp.srcport")]
    pub tcp_src: Option<Vec<String>>,
    #[serde(alias = "tcp.payload")]
    pub tcp_payload: Option<Vec<String>>,
}

fn read_session_messages(mut data: &[u8]) -> Vec<SessionMessage> {
    let mut messages: Vec<SessionMessage> = Vec::with_capacity(1);
    let mut length_bytes = [0u8; 2];
    loop {
        match data.read_exact(&mut length_bytes) {
            Ok(_) => {
                let to_read = u16::from_le_bytes(length_bytes) as usize;
                // Read the message
                let mut buffer = vec![0u8; to_read];
                let _ = data.read_exact(&mut buffer).unwrap();

                let mut zbuf = ZBuf::from(buffer);

                while zbuf.can_read() {
                    match zbuf.read_session_message() {
                        Some(msg) => messages.push(msg),
                        None => (),
                    }
                }
            }
            Err(_) => break,
        }
    }

    messages
}

fn read_zenoh_messages(data: Vec<SessionMessage>) -> Vec<ZenohMessage> {
    let mut messages = vec![];
    for m in data.iter() {
        match &m.body {
            SessionBody::Frame(f) => match &f.payload {
                FramePayload::Messages { messages: msgs } => messages.extend_from_slice(&msgs),
                _ => (),
            },
            _ => (),
        }
    }
    messages
}

#[async_std::main]
async fn main() {
    // initiate logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let contents = fs::read_to_string(opt.file).await.unwrap();
    let pkts: Vec<PcapData> = serde_json::from_str(&contents).unwrap();
    let mut zenoh_data: Vec<u8> = vec![];

    let mut payload_size = 0;
    //let mut total_bytes = 0;
    let mut data_count = 0;

    for pkt in pkts.iter() {
        if let Some(payload) = &pkt.source.layers.tcp_payload {
            let p = payload.first().unwrap();
            let mut d = hex::decode(p).unwrap();
            zenoh_data.append(&mut d);
        }
    }

    println!("Total Size of Zenoh messages: {} bytes", zenoh_data.len());

    let session_messages = read_session_messages(zenoh_data.as_slice());

    println!("Total SessionMessages: {}", session_messages.len());

    let zenoh_messages = read_zenoh_messages(session_messages);

    println!("Total Zenoh Messages: {}", zenoh_messages.len());

    for m in zenoh_messages.iter() {
        match &m.body {
            ZenohBody::Data(d) => {
                data_count += 1;
                payload_size += d.payload.len();
            }
            _ => (),
        }
    }

    let payload = (payload_size / data_count) as u64;

    println!("Total Data messages: {}", data_count);
    println!("Total Payload: {} bytes", payload_size);
    println!("Per message Payload: {} bytes", payload);
}
