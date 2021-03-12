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
use async_std::sync::{Arc, Mutex};
use async_std::task;
use async_trait::async_trait;
use rand::RngCore;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, CongestionControl, PeerId, Reliability, ResKey};
use zenoh::net::protocol::io::{RBuf, WBuf};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::{Data, ZenohBody, ZenohMessage};
use zenoh::net::protocol::session::{
    Session, SessionDispatcher, SessionEventHandler, SessionHandler, SessionManager,
    SessionManagerConfig,
};
use zenoh_util::core::ZResult;

// Session Handler for the peer
struct MySH {
    name: String,
    interval: f64,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl MySH {
    fn new(name: String, interval: f64, pending: Arc<Mutex<HashMap<u64, Instant>>>) -> Self {
        Self {
            name,
            interval,
            pending,
        }
    }
}

#[async_trait]
impl SessionHandler for MySH {
    async fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMH::new(
            self.name.clone(),
            self.interval,
            self.pending.clone(),
        )))
    }
}

// Message Handler for the peer
struct MyMH {
    name: String,
    interval: f64,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl MyMH {
    fn new(name: String, interval: f64, pending: Arc<Mutex<HashMap<u64, Instant>>>) -> Self {
        Self {
            name,
            interval,
            pending,
        }
    }
}

#[async_trait]
impl SessionEventHandler for MyMH {
    async fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        match message.body {
            ZenohBody::Data(Data { mut payload, .. }) => {
                let mut count_bytes = [0u8; 8];
                payload.read_bytes(&mut count_bytes);
                let count = u64::from_le_bytes(count_bytes);
                let instant = self.pending.lock().await.remove(&count).unwrap();
                println!(
                    "session,ping,latency,{},{},{},{},{}",
                    self.name,
                    payload.len(),
                    self.interval,
                    count,
                    instant.elapsed().as_micros()
                );
            }
            _ => panic!("Invalid message"),
        }
        Ok(())
    }

    async fn new_link(&self, _link: Link) {}
    async fn del_link(&self, _link: Link) {}
    async fn closing(&self) {}
    async fn closed(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_sub_thr")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Locator,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "i", long = "interval")]
    interval: f64,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let whatami = match opt.mode.as_str() {
        "peer" => whatami::PEER,
        "client" => whatami::CLIENT,
        _ => panic!("Unsupported mode: {}", opt.mode),
    };

    // Initialize the Peer Id
    let mut pid = [0u8; PeerId::MAX_SIZE];
    rand::thread_rng().fill_bytes(&mut pid);
    let pid = PeerId::new(1, pid);

    let pending: Arc<Mutex<HashMap<u64, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: SessionDispatcher::SessionHandler(Arc::new(MySH::new(
            opt.name,
            opt.interval,
            pending.clone(),
        ))),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.peer).await.unwrap();

    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    loop {
        // Create and send the message
        let reliability = Reliability::Reliable;
        let congestion_control = CongestionControl::Block;
        let key = ResKey::RName("/test/ping".to_string());
        let info = None;

        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: RBuf = data.into();
        let routing_context = None;
        let reply_context = None;
        let attachment = None;

        let message = ZenohMessage::make_data(
            key,
            data,
            reliability,
            congestion_control,
            info,
            routing_context,
            reply_context,
            attachment,
        );

        // Insert the pending ping
        pending.lock().await.insert(count, Instant::now());

        session.handle_message(message.clone()).await.unwrap();

        task::sleep(Duration::from_secs_f64(opt.interval)).await;
        count += 1;
    }
}
