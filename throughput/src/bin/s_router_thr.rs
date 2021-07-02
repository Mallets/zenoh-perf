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
use rand::RngCore;
use slab::Slab;
use std::any::Any;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    Session, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
    SessionManagerOptionalConfig,
};
use zenoh_util::core::ZResult;
use zenoh_util::properties::{IntKeyProperties, Properties};

type Table = Arc<Mutex<Slab<Session>>>;

// Session Handler for the peer
struct MySH {
    table: Table,
}

impl MySH {
    fn new() -> Self {
        Self {
            table: Arc::new(Mutex::new(Slab::new())),
        }
    }
}

impl SessionHandler for MySH {
    fn new_session(&self, session: Session) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        let index = self.table.lock().unwrap().insert(session);
        Ok(Arc::new(MyMH::new(self.table.clone(), index)))
    }
}

// Message Handler for the peer
struct MyMH {
    table: Table,
    index: usize,
}

impl MyMH {
    fn new(table: Table, index: usize) -> Self {
        Self { table, index }
    }
}

impl SessionEventHandler for MyMH {
    fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        for (i, e) in self.table.lock().unwrap().iter() {
            if i != self.index {
                let _ = e.handle_message(message.clone());
            }
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
#[structopt(name = "s_router_thr")]
struct Opt {
    #[structopt(short = "l", long = "listener")]
    listener: Vec<Locator>,
    #[structopt(short = "c", long = "conf", parse(from_os_str))]
    config: Option<PathBuf>,
}

#[async_std::main]
async fn main() {
    // Parse the args
    let opt = Opt::from_args();

    // Initialize the Peer Id
    let mut pid = [0u8; PeerId::MAX_SIZE];
    rand::thread_rng().fill_bytes(&mut pid);
    let pid = PeerId::new(1, pid);

    // Create the session manager
    let config = SessionManagerConfig {
        version: 0,
        whatami: whatami::PEER,
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
    for l in opt.listener.iter() {
        manager.add_listener(l).await.unwrap();
    }
    // Stop forever
    future::pending::<()>().await;
}
