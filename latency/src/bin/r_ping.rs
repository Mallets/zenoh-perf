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
use async_std::sync::{Arc, Mutex};
use async_std::task;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    CongestionControl, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo,
    SubMode, ZInt,
};
use zenoh::net::protocol::io::{RBuf, WBuf};
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::routing::OutSession;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

struct LatencyPrimitives {
    name: String,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl LatencyPrimitives {
    pub fn new(name: String, pending: Arc<Mutex<HashMap<u64, Instant>>>) -> LatencyPrimitives {
        LatencyPrimitives { name, pending }
    }
}

#[async_trait]
impl Primitives for LatencyPrimitives {
    async fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    async fn forget_resource(&self, _rid: ZInt) {}
    async fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn decl_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    async fn send_data(
        &self,
        _reskey: &ResKey,
        mut payload: RBuf,
        _reliability: Reliability,
        _congestion_control: CongestionControl,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        let mut count_bytes = [0u8; 8];
        payload.read_bytes(&mut count_bytes);
        let count = u64::from_le_bytes(count_bytes);
        let instant = self.pending.lock().await.remove(&count).unwrap();
        println!(
            "router,ping,latency,{},{},{},{}",
            self.name,
            payload.len(),
            count,
            instant.elapsed().as_micros()
        );
    }

    async fn send_query(
        &self,
        _reskey: &ResKey,
        _predicate: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: QueryConsolidation,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn send_reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: RBuf,
    ) {
    }
    async fn send_reply_final(&self, _qid: ZInt) {}
    async fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    async fn send_close(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_pub_thr")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "n", long = "name")]
    name: String,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());

    if opt.scout {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "true".to_string());
    } else {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
        config.insert(ZN_PEER_KEY, opt.peer.unwrap());
    }

    let pending: Arc<Mutex<HashMap<u64, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(LatencyPrimitives::new(opt.name, pending.clone()));
    let tx_primitives = runtime
        .read()
        .await
        .router
        .new_primitives(OutSession::Primitives(rx_primitives))
        .await;

    let rid = ResKey::RName("/test/pong".to_string());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    tx_primitives.decl_subscriber(&rid, &sub_info, None).await;

    // @TODO: Fix writer starvation in the RwLock and remove this sleep
    // Wait for the declare to arrive
    task::sleep(Duration::from_millis(1_000)).await;

    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    let reskey = ResKey::RName("/test/ping".to_string());
    loop {
        // Create and send the message
        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: RBuf = data.into();

        tx_primitives
            .send_data(
                &reskey,
                data,
                Reliability::Reliable,
                CongestionControl::Block,
                None,
                None,
            )
            .await;

        count += 1;
    }
}
