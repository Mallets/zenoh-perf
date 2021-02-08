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
use async_std::sync::Arc;
use async_std::task;
use async_trait::async_trait;
use rand::RngCore;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh_protocol::core::{whatami, CongestionControl, PeerId, Reliability, ResKey};
use zenoh_protocol::io::RBuf;
use zenoh_protocol::link::{Link, Locator};
use zenoh_protocol::proto::ZenohMessage;
use zenoh_protocol::session::{
    Session, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
};
use zenoh_util::core::ZResult;

// Session Handler for the peer
struct MySH {
    counter: Arc<AtomicUsize>,
    active: AtomicBool,
}

impl MySH {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self {
            counter,
            active: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl SessionHandler for MySH {
    async fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        if !self.active.swap(true, Ordering::Acquire) {
            let count = self.counter.clone();
            task::spawn(async move {
                loop {
                    task::sleep(Duration::from_secs(1)).await;
                    let c = count.swap(0, Ordering::Relaxed);
                    println!("{} msg/s", c);
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

#[async_trait]
impl SessionEventHandler for MyMH {
    async fn handle_message(&self, _message: ZenohMessage) -> ZResult<()> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn new_link(&self, _link: Link) {}
    async fn del_link(&self, _link: Link) {}
    async fn closing(&self) {}
    async fn closed(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_pubsub_thr")]
struct Opt {
    #[structopt(short = "l", long = "listener")]
    listener: Locator,
    #[structopt(short = "e", long = "peer")]
    peer: Locator,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
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

    let count = Arc::new(AtomicUsize::new(0));
    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: Arc::new(MySH::new(count)),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let _ = manager.add_listener(&opt.listener).await.unwrap();

    let session = loop {
        match manager.open_session(&opt.peer).await {
            Ok(s) => break s,
            Err(_) => task::sleep(Duration::from_secs(1)).await,
        }
    };

    // Send reliable messages
    let reliability = Reliability::Reliable;
    let congestion_control = CongestionControl::Block;
    let key = ResKey::RName("test".to_string());
    let info = None;
    let payload = RBuf::from(vec![0u8; opt.payload]);
    let reply_context = None;
    let routing_context = None;
    let attachment = None;

    let message = ZenohMessage::make_data(
        key,
        payload,
        reliability,
        congestion_control,
        info,
        routing_context,
        reply_context,
        attachment,
    );

    loop {
        let res = session.handle_message(message.clone()).await;
        if res.is_err() {
            break;
        }
    }
}
