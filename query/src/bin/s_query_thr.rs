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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId, QueryConsolidation, QueryTarget, ResKey};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::{Data, ZenohBody, ZenohMessage};
use zenoh::net::protocol::session::{
    Session, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
};
use zenoh_util::core::ZResult;

type Pending = Arc<Mutex<HashMap<u64, Arc<Barrier>>>>;

// Session Handler for the blocking locator
struct MySH {
    pending: Pending,
}

impl MySH {
    fn new(pending: Pending) -> Self {
        Self { pending }
    }
}

impl SessionHandler for MySH {
    fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMH::new(self.pending.clone())))
    }
}

// Message Handler for the locator
struct MyMH {
    pending: Pending,
}

impl MyMH {
    fn new(pending: Pending) -> Self {
        Self { pending }
    }
}

impl SessionEventHandler for MyMH {
    fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        match message.body {
            ZenohBody::Data(Data { reply_context, .. }) => {
                let reply_context = reply_context.unwrap();
                let barrier = self
                    .pending
                    .lock()
                    .unwrap()
                    .remove(&reply_context.qid)
                    .unwrap();
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
#[structopt(name = "s_query")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Locator,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
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

    let rtt = Arc::new(AtomicUsize::new(0));
    let counter = Arc::new(AtomicUsize::new(0));
    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: Arc::new(MySH::new(pending.clone())),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.locator).await.unwrap();

    let c_rtt = rtt.clone();
    let c_counter = counter.clone();
    task::spawn(async move {
        loop {
            let now = Instant::now();
            task::sleep(Duration::from_secs(1)).await;
            let elapsed = now.elapsed().as_micros() as f64;

            let r = c_rtt.swap(0, Ordering::Relaxed);
            let c = c_counter.swap(0, Ordering::Relaxed);
            if c > 0 {
                let interval = 1_000_000.0 / elapsed;
                println!(
                    "session,{},query.throughput,{},{},{},{}",
                    opt.scenario,
                    opt.name,
                    opt.payload,
                    (c as f64 / interval).floor() as usize,
                    (r as f64 / c as f64).floor() as usize,
                );
            }
        }
    });

    let mut count: u64 = 0;
    loop {
        // Create and send the message
        let key = ResKey::RName("/test/query".to_string());
        let predicate = "".to_string();
        let qid = count;
        let target = Some(QueryTarget::default());
        let consolidation = QueryConsolidation::default();
        let routing_context = None;
        let attachment = None;

        let message = ZenohMessage::make_query(
            key,
            predicate,
            qid,
            target,
            consolidation,
            routing_context,
            attachment,
        );

        // Insert the pending query
        let barrier = Arc::new(Barrier::new(2));
        pending.lock().unwrap().insert(count, barrier.clone());
        let now = Instant::now();
        session.handle_message(message).unwrap();
        // Wait for the reply to arrive
        barrier.wait();
        rtt.fetch_add(now.elapsed().as_micros() as usize, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);

        count += 1;
    }
}
