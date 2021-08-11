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
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    Channel, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo, SubMode, ZInt,
};
use zenoh::net::protocol::io::ZBuf;
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::routing::face::Face;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_LISTENER_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

struct LatencyPrimitives {
    tx: Mutex<Option<Arc<Face>>>,
}

impl LatencyPrimitives {
    fn new() -> LatencyPrimitives {
        LatencyPrimitives {
            tx: Mutex::new(None),
        }
    }

    fn set_tx(&self, tx: Arc<Face>) {
        let mut guard = self.tx.lock().unwrap();
        *guard = Some(tx);
    }
}

impl Primitives for LatencyPrimitives {
    fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    fn forget_resource(&self, _rid: ZInt) {}
    fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn decl_queryable(
        &self,
        _reskey: &ResKey,
        _kind: ZInt,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    fn send_data(
        &self,
        _reskey: &ResKey,
        payload: ZBuf,
        channel: Channel,
        data_info: Option<DataInfo>,
        routing_context: Option<RoutingContext>,
    ) {
        let reskey = ResKey::RName("/test/pong".to_string());
        self.tx.lock().unwrap().as_ref().unwrap().send_data(
            &reskey,
            payload,
            channel,
            data_info,
            routing_context,
        );
    }

    fn send_query(
        &self,
        _reskey: &ResKey,
        _predicate: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: QueryConsolidation,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn send_reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: ZBuf,
    ) {
    }
    fn send_reply_final(&self, _qid: ZInt) {}
    fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    fn send_close(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_sub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: String,
    #[structopt(short = "m", long = "mode")]
    mode: String,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());

    config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
    match opt.mode.as_str() {
        "peer" | "router" => config.insert(ZN_LISTENER_KEY, opt.locator),
        "client" => config.insert(ZN_PEER_KEY, opt.locator),
        _ => panic!("Unsupported mode: {}", opt.mode),
    };

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(LatencyPrimitives::new());
    let tx_primitives = runtime.router.new_primitives(rx_primitives.clone());
    rx_primitives.set_tx(tx_primitives.clone());

    let rid = ResKey::RName("/test/ping".to_string());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    tx_primitives.decl_subscriber(&rid, &sub_info, None);

    // Stop forever
    future::pending::<()>().await;
}
