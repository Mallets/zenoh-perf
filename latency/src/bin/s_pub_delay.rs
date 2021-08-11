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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, Channel, PeerId, Priority, Reliability, ResKey};
use zenoh::net::protocol::link::Locator;
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    DummySessionEventHandler, Session, SessionEventHandler, SessionHandler, SessionManager,
    SessionManagerConfig,
};
use zenoh_util::core::ZResult;

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
#[structopt(name = "z_ping")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Locator,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
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

    let config = SessionManagerConfig {
        version: 0,
        whatami,
        id: pid,
        handler: Arc::new(MySH::new()),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    let session = manager.open_session(&opt.locator).await.unwrap();

    let mut count: u64 = 0;
    loop {
        // Send reliable messages
        let channel = Channel {
            priority: Priority::Data,
            reliability: Reliability::Reliable,
        };
        let key = ResKey::RName("/test/ping".to_string());
        let info = None;
        let routing_context = None;
        let reply_context = None;
        let attachment = None;

        // u64 (8 bytes) for seq num
        // u128 (16 bytes) for system time in nanoseconds
        let mut payload = vec![0u8; opt.payload];
        let count_bytes: [u8; 8] = count.to_le_bytes();
        let now_bytes: [u8; 16] = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes();
        payload[0..8].copy_from_slice(&count_bytes);
        payload[8..24].copy_from_slice(&now_bytes);

        let message = ZenohMessage::make_data(
            key,
            payload.into(),
            channel,
            info,
            routing_context,
            reply_context,
            attachment,
        );

        let _ = session.handle_message(message.clone()).unwrap();

        task::sleep(Duration::from_secs_f64(opt.interval)).await;
        count += 1;
    }
}
