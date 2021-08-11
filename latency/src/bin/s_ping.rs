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
use async_std::task;
use rand::RngCore;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    whatami, Channel, PeerId, Priority, Reliability, ResKey, WhatAmI,
};
use zenoh::net::protocol::io::{WBuf, ZBuf};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::{Data, ZenohBody, ZenohMessage};
use zenoh::net::protocol::session::{
    Session, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
};
use zenoh_util::core::ZResult;

// Session Handler for the non-blocking locator
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

impl SessionHandler for MySHParallel {
    fn new_session(
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

// Message Handler for the locator
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

impl SessionEventHandler for MyMHParallel {
    fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        match message.body {
            ZenohBody::Data(Data { mut payload, .. }) => {
                let mut count_bytes = [0u8; 8];
                payload.read_bytes(&mut count_bytes);
                let count = u64::from_le_bytes(count_bytes);
                let instant = self.pending.lock().unwrap().remove(&count).unwrap();
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

    fn new_link(&self, _link: Link) {}
    fn del_link(&self, _link: Link) {}
    fn closing(&self) {}
    fn closed(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Session Handler for the blocking locator
struct MySHSequential {
    pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>,
}

impl MySHSequential {
    fn new(pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>) -> Self {
        Self { pending }
    }
}

impl SessionHandler for MySHSequential {
    fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMHSequential::new(self.pending.clone())))
    }
}

// Message Handler for the locator
struct MyMHSequential {
    pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>,
}

impl MyMHSequential {
    fn new(pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>) -> Self {
        Self { pending }
    }
}

impl SessionEventHandler for MyMHSequential {
    fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        match message.body {
            ZenohBody::Data(Data { mut payload, .. }) => {
                let mut count_bytes = [0u8; 8];
                payload.read_bytes(&mut count_bytes);
                let count = u64::from_le_bytes(count_bytes);
                let barrier = self.pending.lock().unwrap().remove(&count).unwrap();
                barrier.wait();
            }
            _ => panic!("Invalid message"),
        }
        Ok(())
    }

    fn new_link(&self, _link: Link) {}
    fn del_link(&self, _link: Link) {}
    fn closing(&self) {}
    fn closed(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_sub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Locator,
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
        handler: Arc::new(MySHSequential::new(pending.clone())),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.locator).await.unwrap();

    let sleep = Duration::from_secs_f64(opt.interval);
    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    loop {
        // Create and send the message
        let channel = Channel {
            priority: Priority::Data,
            reliability: Reliability::Reliable,
        };
        let key = ResKey::RName("/test/ping".to_string());
        let info = None;

        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: ZBuf = data.into();
        let routing_context = None;
        let reply_context = None;
        let attachment = None;

        let message = ZenohMessage::make_data(
            key,
            data,
            channel,
            info,
            routing_context,
            reply_context,
            attachment,
        );

        // Insert the pending ping
        let barrier = Arc::new(Barrier::new(2));
        pending.lock().unwrap().insert(count, barrier.clone());
        let now = Instant::now();
        session.handle_message(message).unwrap();
        // Wait for the pong to arrive
        barrier.wait();
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
        handler: Arc::new(MySHParallel::new(
            opt.scenario,
            opt.name,
            opt.interval,
            pending.clone(),
        )),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.locator).await.unwrap();

    let sleep = Duration::from_secs_f64(opt.interval);
    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    loop {
        // Create and send the message
        let channel = Channel {
            priority: Priority::Data,
            reliability: Reliability::Reliable,
        };
        let key = ResKey::RName("/test/ping".to_string());
        let info = None;

        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: ZBuf = data.into();
        let routing_context = None;
        let reply_context = None;
        let attachment = None;

        let message = ZenohMessage::make_data(
            key,
            data,
            channel,
            info,
            routing_context,
            reply_context,
            attachment,
        );

        // Insert the pending ping
        pending.lock().unwrap().insert(count, Instant::now());

        session.handle_message(message).unwrap();

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
