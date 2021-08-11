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
use rand::RngCore;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, Channel, PeerId, Priority, Reliability, ResKey};
use zenoh::net::protocol::io::ZBuf;
use zenoh::net::protocol::link::Locator;
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    DummySessionEventHandler, Session, SessionEventHandler, SessionHandler, SessionManager,
    SessionManagerConfig, SessionManagerOptionalConfig,
};
use zenoh_util::core::ZResult;
use zenoh_util::properties::{IntKeyProperties, Properties};

struct MySH {}

impl MySH {
    fn new() -> Self {
        Self {}
    }
}

impl SessionHandler for MySH {
    fn new_session(
        &self,
        _session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(DummySessionEventHandler::new()))
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_pub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Locator,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "t", long = "print")]
    print: bool,
    #[structopt(short = "c", long = "conf", parse(from_os_str))]
    config: Option<PathBuf>,
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

    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: Arc::new(MySH::new()),
    };
    let opt_config = match opt.config.as_ref() {
        Some(f) => {
            let config = async_std::fs::read_to_string(f).await.unwrap();
            let properties = Properties::from(config);
            let int_props = IntKeyProperties::from(properties);
            SessionManagerOptionalConfig::from_properties(&int_props)
                .await
                .unwrap()
        }
        None => None,
    };
    let manager = SessionManager::new(config, opt_config);

    // Connect to publisher
    let session = manager.open_session(&opt.locator).await.unwrap();

    // Send reliable messages
    let channel = Channel {
        priority: Priority::Data,
        reliability: Reliability::Reliable,
    };
    let key = ResKey::RId(1);
    let info = None;
    let payload = ZBuf::from(vec![0u8; opt.payload]);
    let reply_context = None;
    let routing_context = None;
    let attachment = None;

    if opt.print {
        let count = Arc::new(AtomicUsize::new(0));
        let c_count = count.clone();
        task::spawn(async move {
            loop {
                task::sleep(Duration::from_secs(1)).await;
                let c = count.swap(0, Ordering::Relaxed);
                if c > 0 {
                    println!("{} msg/s", c);
                }
            }
        });

        loop {
            let message = ZenohMessage::make_data(
                key.clone(),
                payload.clone(),
                channel,
                info.clone(),
                routing_context,
                reply_context.clone(),
                attachment.clone(),
            );
            let res = session.handle_message(message);
            if res.is_err() {
                break;
            }
            c_count.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        loop {
            let message = ZenohMessage::make_data(
                key.clone(),
                payload.clone(),
                channel,
                info.clone(),
                routing_context,
                reply_context.clone(),
                attachment.clone(),
            );
            let res = session.handle_message(message);
            if res.is_err() {
                break;
            }
        }
    }
}
