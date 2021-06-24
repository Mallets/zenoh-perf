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
use rand::RngCore;
use std::any::Any;
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    Session, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
};
use zenoh_util::core::ZResult;

// Session Handler for the peer
struct MySH;

impl MySH {
    fn new() -> Self {
        Self
    }
}

impl SessionHandler for MySH {
    fn new_session(&self, session: Session) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        Ok(Arc::new(MyMH::new(session)))
    }
}

// Message Handler for the peer
struct MyMH {
    session: Session,
}

impl MyMH {
    fn new(session: Session) -> Self {
        Self { session }
    }
}

impl SessionEventHandler for MyMH {
    fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        self.session.handle_message(message)
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

    // Connect to the peer or listen
    if whatami == whatami::PEER {
        manager.add_listener(&opt.locator).await.unwrap();
    } else {
        let _session = manager.open_session(&opt.locator).await.unwrap();
    }

    // Stop forever
    future::pending::<()>().await;
}
