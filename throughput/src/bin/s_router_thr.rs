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
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use rand::RngCore;
use slab::Slab;
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId};
use zenoh::net::protocol::link::{Link, Locator};
use zenoh::net::protocol::proto::ZenohMessage;
use zenoh::net::protocol::session::{
    Session, SessionDispatcher, SessionEventHandler, SessionHandler, SessionManager, SessionManagerConfig,
};
use zenoh_util::core::ZResult;

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

#[async_trait]
impl SessionHandler for MySH {
    async fn new_session(
        &self,
        session: Session,
    ) -> ZResult<Arc<dyn SessionEventHandler + Send + Sync>> {
        let index = self.table.lock().await.insert(session);
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

#[async_trait]
impl SessionEventHandler for MyMH {
    async fn handle_message(&self, message: ZenohMessage) -> ZResult<()> {
        for (i, e) in self.table.lock().await.iter() {
            if i != self.index {
                let _ = e.handle_message(message.clone()).await;
            }
        }
        Ok(())
    }

    async fn new_link(&self, _link: Link) {}
    async fn del_link(&self, _link: Link) {}
    async fn closing(&self) {}
    async fn closed(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_router_thr")]
struct Opt {
    #[structopt(short = "l", long = "listener")]
    listener: Locator,
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
        handler: SessionDispatcher::SessionHandler(Arc::new(MySH::new())),
    };
    let manager = SessionManager::new(config, None);

    // Connect to publisher
    manager.add_listener(&opt.listener).await.unwrap();
    // Stop forever
    future::pending::<()>().await;
}
