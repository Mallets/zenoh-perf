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
use async_std::sync::{Arc, Barrier, Mutex};
use async_std::task;
use async_trait::async_trait;
use rand::RngCore;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    whatami, CongestionControl, PeerId, Reliability, ResKey, WhatAmI,
};
use zenoh::net::protocol::io::{RBuf, WBuf};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::{Data, ZenohBody, ZenohMessage};
use zenoh::net::protocol::session::{
    Session, SessionDispatcher, SessionEventHandler, SessionHandler, SessionManager,
    SessionManagerConfig,
};
use zenoh_util::core::ZResult;

// Session Handler for the non-blocking peer
struct MySHParallel {
    scenario: String,
    name: String,
    interval: f64,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl MySHParallel {
    fn new(
        scenario: String,
        name: String,
        interval: f64,
        pending: Arc<Mutex<HashMap<u64, Instant>>>,
    ) -> Self {
        Self {
            scenario,
            name,
            interval,
            pending,
        }
    }
}

#[async_trait]
impl SessionHandler for MySHParallel {
    async fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMHParallel::new(
            self.scenario.clone(),
            self.name.clone(),
            self.interval,
            self.pending.clone(),
        )))
    }
}

// Message Handler for the peer
struct MyMHParallel {
    scenario: String,
    name: String,
    interval: f64,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl MyMHParallel {
    fn new(
        scenario: String,
        name: String,
        interval: f64,
        pending: Arc<Mutex<HashMap<u64, Instant>>>,
    ) -> Self {
        Self {
            scenario,
            name,
            interval,
            pending,
        }
    }
}

#[async_trait]
impl SessionEventHandler for MyMHParallel {
    async fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        match message.body {
            ZenohBody::Data(Data { mut payload, .. }) => {
                let mut count_bytes = [0u8; 8];
                payload.read_bytes(&mut count_bytes);
                let count = u64::from_le_bytes(count_bytes);
                let instant = self.pending.lock().await.remove(&count).unwrap();
                println!(
                    "session,{},latency.parallel,{},{},{},{},{}",
                    self.scenario,
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

// Session Handler for the blocking peer
struct MySHSequential {
    pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>,
}

impl MySHSequential {
    fn new(pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>) -> Self {
        Self { pending }
    }
}

#[async_trait]
impl SessionHandler for MySHSequential {
    async fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMHSequential::new(self.pending.clone())))
    }
}

// Message Handler for the peer
struct MyMHSequential {
    pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>,
}

impl MyMHSequential {
    fn new(pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>) -> Self {
        Self { pending }
    }
}

#[async_trait]
impl SessionEventHandler for MyMHSequential {
    async fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        match message.body {
            ZenohBody::Data(Data { mut payload, .. }) => {
                let mut count_bytes = [0u8; 8];
                payload.read_bytes(&mut count_bytes);
                let count = u64::from_le_bytes(count_bytes);
                let barrier = self.pending.lock().await.remove(&count).unwrap();
                barrier.wait().await;
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
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
    #[structopt(short = "i", long = "interval")]
    interval: f64,
    #[structopt(long = "parallel")]
    parallel: bool,
}

async fn single(opt: Opt, whatami: WhatAmI, pid: PeerId) {
    let pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>> = Arc::new(Mutex::new(HashMap::new()));
    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: SessionDispatcher::SessionHandler(Arc::new(MySHSequential::new(pending.clone()))),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.peer).await.unwrap();

    let sleep = Duration::from_secs_f64(opt.interval);
    let barrier = Arc::new(Barrier::new(2));
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
        pending.lock().await.insert(count, barrier.clone());
        let now = Instant::now();
        session.handle_message(message).await.unwrap();
        // Wait for the pong to arrive
        barrier.wait().await;
        println!(
            "session,{},latency.sequential,{},{},{},{},{}",
            opt.scenario,
            opt.name,
            payload.len(),
            opt.interval,
            count,
            now.elapsed().as_micros()
        );

        task::sleep(sleep).await;
        count += 1;
    }
}

async fn parallel(opt: Opt, whatami: WhatAmI, pid: PeerId) {
    let pending: Arc<Mutex<HashMap<u64, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: SessionDispatcher::SessionHandler(Arc::new(MySHParallel::new(
            opt.scenario,
            opt.name,
            opt.interval,
            pending.clone(),
        ))),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.peer).await.unwrap();

    let sleep = Duration::from_secs_f64(opt.interval);
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

        session.handle_message(message).await.unwrap();

        task::sleep(sleep).await;
        count += 1;
    }
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

    if opt.parallel {
        parallel(opt, whatami, pid).await;
    } else {
        single(opt, whatami, pid).await;
    }
}
