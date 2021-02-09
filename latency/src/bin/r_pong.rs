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
use structopt::StructOpt;
use zenoh_protocol::core::{
    CongestionControl, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo,
    SubMode, ZInt,
};
use zenoh_protocol::io::RBuf;
use zenoh_protocol::proto::{DataInfo, RoutingContext};
use zenoh_protocol::session::Primitives;
use zenoh_router::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_LISTENER_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

struct LatencyPrimitives {
    tx: Mutex<Option<Arc<dyn Primitives + Send + Sync>>>,
}

impl LatencyPrimitives {
    fn new() -> LatencyPrimitives {
        LatencyPrimitives {
            tx: Mutex::new(None),
        }
    }

    async fn set_tx(&self, tx: Arc<dyn Primitives + Send + Sync>) {
        let mut guard = self.tx.lock().await;
        *guard = Some(tx);
    }
}

#[async_trait]
impl Primitives for LatencyPrimitives {
    async fn resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    async fn forget_resource(&self, _rid: ZInt) {}
    async fn publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    async fn data(
        &self,
        _reskey: &ResKey,
        payload: RBuf,
        reliability: Reliability,
        congestion_control: CongestionControl,
        data_info: Option<DataInfo>,
        routing_context: Option<RoutingContext>,
    ) {
        let reskey = ResKey::RName("/test/pong".to_string());
        self.tx
            .lock()
            .await
            .as_ref()
            .unwrap()
            .data(
                &reskey,
                payload,
                reliability,
                congestion_control,
                data_info,
                routing_context,
            )
            .await;
    }

    async fn query(
        &self,
        _reskey: &ResKey,
        _predicate: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: QueryConsolidation,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: RBuf,
    ) {
    }
    async fn reply_final(&self, _qid: ZInt) {}
    async fn pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    async fn close(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_sub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "d", long = "id")]
    id: String,
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
        match opt.mode.as_str() {
            "peer" | "router" => config.insert(ZN_LISTENER_KEY, opt.locator.unwrap()),
            "client" => config.insert(ZN_PEER_KEY, opt.locator.unwrap()),
            _ => panic!("Unsupported mode: {}", opt.mode),
        };
    }

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(LatencyPrimitives::new());
    let tx_primitives = runtime
        .read()
        .await
        .router
        .new_primitives(rx_primitives.clone())
        .await;
    rx_primitives.set_tx(tx_primitives).await;

    let rid = ResKey::RName("/test/ping".to_string());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    rx_primitives.subscriber(&rid, &sub_info, None).await;

    // Stop forever
    future::pending::<()>().await;
}
