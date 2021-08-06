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
use async_std::net::{SocketAddr, UdpSocket};
use async_std::sync::Arc;
use async_std::task;
use rand::RngCore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId};
use zenoh::net::protocol::io::{WBuf, ZBuf, ZSlice};
use zenoh::net::protocol::proto::{InitSyn, OpenSyn, SessionBody, SessionMessage};

macro_rules! zsend {
    ($msg:expr, $socket:expr, $addr:expr) => {{
        // Create the buffer for serializing the message
        let mut wbuf = WBuf::new(32, false);
        // Serialize the message
        assert!(wbuf.write_session_message(&$msg));
        let mut bytes = vec![0u8; wbuf.len()];
        wbuf.copy_into_slice(&mut bytes[..]);
        // Send the message on the link
        let res = $socket.send_to(&bytes, $addr).await;
        log::trace!("Sending {:?}: {:?}", $msg, res);
        res
    }};
}

macro_rules! zrecv {
    ($socket:expr, $buffer:expr) => {{
        let (n, addr) = $socket.recv_from(&mut $buffer).await.unwrap();
        let mut zbuf = ZBuf::from(&$buffer[..n]);
        (zbuf.read_session_message().unwrap(), addr)
    }};
}

async fn handle_client(socket: Arc<UdpSocket>) -> Result<(), Box<dyn std::error::Error>> {
    let my_whatami = whatami::ROUTER;
    let mut my_pid = [0u8; PeerId::MAX_SIZE];
    rand::thread_rng().fill_bytes(&mut my_pid);
    let my_pid = PeerId::new(PeerId::MAX_SIZE, my_pid);

    // Create the reading buffer
    let mut buffer = vec![0u8; 16_000_000];

    // Read the InitSyn
    let (message, addr) = zrecv!(socket, buffer);
    match &message.body {
        SessionBody::InitSyn(InitSyn { is_qos, .. }) => {
            let whatami = my_whatami;
            let sn_resolution = None;
            let cookie = ZSlice::from(vec![0u8; 8]);
            let attachment = None;
            let message = SessionMessage::make_init_ack(
                whatami,
                my_pid.clone(),
                sn_resolution,
                *is_qos,
                cookie,
                attachment,
            );
            // Send the InitAck
            let _ = zsend!(message, socket, addr).unwrap();
        }
        _ => panic!(),
    }

    // Read the OpenSyn
    let (message, a) = zrecv!(socket, buffer);
    if a != addr {
        panic!("Received data from {}, expected from {}", a, addr);
    }
    match &message.body {
        SessionBody::OpenSyn(OpenSyn {
            lease, initial_sn, ..
        }) => {
            let attachment = None;
            let message = SessionMessage::make_open_ack(*lease, *initial_sn, attachment);
            // Send the OpenAck
            let _ = zsend!(message, socket, addr).unwrap();
        }
        _ => panic!(),
    }

    // Spawn the loggin task
    let counter = Arc::new(AtomicUsize::new(0));
    let c_c = counter.clone();
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(1)).await;
            let c = c_c.swap(0, Ordering::Relaxed);
            if c > 0 {
                println!("{:.6} Gbit/s", (8_f64 * c as f64) / 1000000000_f64);
            }
        }
    });

    // Spawn the KeepAlive task
    let c_socket = socket.clone();
    let c_addr = addr;
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(1)).await;
            let message = SessionMessage::make_keep_alive(None, None);
            let _ = zsend!(message, c_socket, c_addr).unwrap();
        }
    });

    // Read from the socket
    loop {
        let (n, a) = socket.recv_from(&mut buffer).await.unwrap();
        if a != addr {
            panic!("Received data from {}, expected from {}", a, addr);
        }
        let _ = counter.fetch_add(n, Ordering::Relaxed);
    }
}

async fn run(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(addr).await?;
    handle_client(Arc::new(socket)).await
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_sink_udp")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: SocketAddr,
}

#[async_std::main]
async fn main() {
    env_logger::init();
    let opt = Opt::from_args();
    let _ = run(opt.locator).await;
}
