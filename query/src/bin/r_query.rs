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
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex};
use std::time::Instant;
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    Channel, PeerId, QueryConsolidation, QueryTarget, ResKey, SubInfo, ZInt,
};
use zenoh::net::protocol::io::ZBuf;
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

type Pending = Arc<Mutex<HashMap<u64, (Instant, Arc<Barrier>)>>>;

struct QueryPrimitives {
    scenario: String,
    name: String,
    pending: Pending,
}

impl QueryPrimitives {
    pub fn new(scenario: String, name: String, pending: Pending) -> QueryPrimitives {
        QueryPrimitives {
            scenario,
            name,
            pending,
        }
    }
}

impl Primitives for QueryPrimitives {
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
        _payload: ZBuf,
        _channel: Channel,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
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
        qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        payload: ZBuf,
    ) {
        let tuple = self.pending.lock().unwrap().remove(&qid).unwrap();
        let (instant, barrier) = (tuple.0, tuple.1);
        barrier.wait();
        println!(
            "router,{},query.latency,{},{},{},{}",
            self.scenario,
            self.name,
            payload.len(),
            qid,
            instant.elapsed().as_micros()
        );
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
#[structopt(name = "r_query")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: String,
    #[structopt(short = "m", long = "mode")]
    mode: String,
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

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());

    config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
    config.insert(ZN_PEER_KEY, opt.locator);

    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(QueryPrimitives::new(
        opt.scenario,
        opt.name,
        pending.clone(),
    ));
    let tx_primitives = runtime.router.new_primitives(rx_primitives);

    let barrier = Arc::new(Barrier::new(2));
    let mut count: u64 = 0;
    loop {
        let reskey = ResKey::RName("/test/query".to_string());
        let predicate = "";
        let qid = count;
        let target = QueryTarget::default();
        let consolidation = QueryConsolidation::default();
        let routing_context = None;

        // Insert the pending query
        pending
            .lock()
            .unwrap()
            .insert(count, (Instant::now(), barrier.clone()));
        tx_primitives.send_query(
            &reskey,
            predicate,
            qid,
            target.clone(),
            consolidation.clone(),
            routing_context,
        );
        // Wait for the reply to arrive
        barrier.wait();

        count += 1;
    }
}
