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
use async_std::future;
use async_std::sync::Arc;
use async_std::task;
use rand::RngCore;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    Session, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
};
use zenoh_util::core::ZResult;

// Session Handler for the peer
struct MySH {
    scenario: String,
    name: String,
    payload: usize,
    counter: Arc<AtomicUsize>,
    active: AtomicBool,
}

impl MySH {
    fn new(scenario: String, name: String, payload: usize, counter: Arc<AtomicUsize>) -> Self {
        Self {
            scenario,
            name,
            payload,
            counter,
            active: AtomicBool::new(false),
        }
    }
}

impl SessionHandler for MySH {
    fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        if !self.active.swap(true, Ordering::Acquire) {
            let count = self.counter.clone();
            let scenario = self.scenario.clone();
            let name = self.name.clone();
            let payload = self.payload;
            task::spawn(async move {
                loop {
                    let now = Instant::now();
                    task::sleep(Duration::from_secs(1)).await;
                    let elapsed = now.elapsed().as_micros() as f64;

                    let c = count.swap(0, Ordering::Relaxed);
                    if c > 0 {
                        let interval = 1_000_000.0 / elapsed;
                        println!(
                            "session,{},throughput,{},{},{}",
                            scenario,
                            name,
                            payload,
                            (c as f64 / interval).floor() as usize
                        );
                    }
                }
            });
        }
        Ok(Arc::new(MyMH::new(self.counter.clone())))
    }
}

// Message Handler for the peer
struct MyMH {
    counter: Arc<AtomicUsize>,
}

impl MyMH {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self { counter }
    }
}

impl SessionEventHandler for MyMH {
    fn handle_message(&self, _message: ZenohMessage) -> ZResult<()> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn new_link(&self, _link: Link) {}
    fn del_link(&self, _link: Link) {}
    fn closing(&self) {}
    fn closed(&self) {}
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

    let count = Arc::new(AtomicUsize::new(0));
    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: Arc::new(MySH::new(opt.scenario, opt.name, opt.payload, count)),
    };
    let manager = SessionManager::new(config, None);

    // Connect to the peer or listen
    if whatami == whatami::PEER {
        manager.add_listener(&opt.locator).await.unwrap();
    } else {
        let _session = manager.open_session(&opt.locator).unwrap();
    }

    // Stop forever
    future::pending::<()>().await;
}
