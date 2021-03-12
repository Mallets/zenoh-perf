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
use async_std::sync::{Arc, Barrier};
use async_std::task;
use async_trait::async_trait;
use rand::RngCore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId, QueryConsolidation, ResKey};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    Session, SessionDispatcher, SessionEventHandler, SessionHandler, SessionManager,
    SessionManagerConfig,
};
use zenoh_util::core::ZResult;

struct MySH {
    barrier: Arc<Barrier>,
}

impl MySH {
    fn new(barrier: Arc<Barrier>) -> Self {
        Self { barrier }
    }
}

#[async_trait]
impl SessionHandler for MySH {
    async fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMH::new(self.barrier.clone())))
    }
}

// Message Handler for the peer
struct MyMH {
    barrier: Arc<Barrier>,
}

impl MyMH {
    fn new(barrier: Arc<Barrier>) -> Self {
        Self { barrier }
    }
}

#[async_trait]
impl SessionEventHandler for MyMH {
    async fn handle_message(&self, _message: ZenohMessage) -> ZResult<()> {
        self.barrier.wait().await;
        Ok(())
    }

    async fn new_link(&self, _link: Link) {}
    async fn del_link(&self, _link: Link) {}
    async fn closing(&self) {}
    async fn closed(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_pub_thr")]
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
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    // Initialize the Peer Id
    let mut pid = [0u8; PeerId::MAX_SIZE];
    rand::thread_rng().fill_bytes(&mut pid);
    let pid = PeerId::new(1, pid);

    let whatami = match opt.mode.as_str() {
        "peer" => whatami::PEER,
        "client" => whatami::CLIENT,
        _ => panic!("Unsupported mode: {}", opt.mode),
    };

    // Initialize the barrier
    let barrier = Arc::new(Barrier::new(2));

    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: SessionDispatcher::SessionHandler(Arc::new(MySH::new(barrier.clone()))),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.peer).await.unwrap();

    // Send reliable messages
    let key = ResKey::RName("test".to_string());
    let predicate = "".to_string();
    let qid = 0;
    let target = None;
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

    let count = Arc::new(AtomicUsize::new(0));
    let c_count = count.clone();
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(1)).await;
            let c = count.swap(0, Ordering::AcqRel);
            if c > 0 {
                println!(
                    "session,{},query,{},{},{}",
                    opt.scenario, opt.name, opt.payload, c
                );
            }
        }
    });

    loop {
        let res = session.handle_message(message.clone()).await;
        if res.is_err() {
            break;
        }
        barrier.wait().await;
        c_count.fetch_add(1, Ordering::AcqRel);
    }
}
