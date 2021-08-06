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
use async_std::net::{SocketAddr, TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::sync::Arc;
use async_std::task;
use rand::RngCore;
use std::convert::TryInto;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::protocol::core::{whatami, PeerId};
use zenoh::net::protocol::io::{WBuf, ZBuf, ZSlice};
use zenoh::net::protocol::proto::{InitSyn, OpenSyn, SessionBody, SessionMessage};

macro_rules! zsend {
    ($msg:expr, $stream:expr) => {{
        // Create the buffer for serializing the message
        let mut wbuf = WBuf::new(32, false);
        // Reserve 16 bits to write the length
        assert!(wbuf.write_bytes(&[0u8, 0u8]));
        // Serialize the message
        assert!(wbuf.write_session_message(&$msg));
        // Write the length on the first 16 bits
        let length: u16 = wbuf.len() as u16 - 2;
        let bits = wbuf.get_first_slice_mut(..2);
        bits.copy_from_slice(&length.to_le_bytes());
        let mut bytes = vec![0u8; wbuf.len()];
        wbuf.copy_into_slice(&mut bytes[..]);

        // Send the message on the link
        let res = $stream.write_all(&bytes).await;
        log::trace!("Sending {:?}: {:?}", $msg, res);

        res
    }};
}

macro_rules! zrecv {
    ($stream:expr, $buffer:expr) => {{
        let _ = $stream.read_exact(&mut $buffer[0..2]).await.unwrap();
        let length: [u8; 2] = $buffer[0..2].try_into().unwrap();
        // Decode the total amount of bytes that we are expected to read
        let to_read = u16::from_le_bytes(length) as usize;
        $stream.read_exact(&mut $buffer[0..to_read]).await.unwrap();
        let mut zbuf = ZBuf::from(&$buffer[..]);
        zbuf.read_session_message().unwrap()
    }};
}

async fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let my_whatami = whatami::ROUTER;
    let mut my_pid = [0u8; PeerId::MAX_SIZE];
    rand::thread_rng().fill_bytes(&mut my_pid);
    let my_pid = PeerId::new(1, my_pid);

    // Create the reading buffer
    let mut buffer = vec![0u8; 16_000_000];

    // Read the InitSyn
    let message = zrecv!(stream, buffer);
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
            let _ = zsend!(message, stream).unwrap();
        }
        _ => panic!(),
    }

    // Read the OpenSyn
    let message = zrecv!(stream, buffer);
    match &message.body {
        SessionBody::OpenSyn(OpenSyn {
            lease, initial_sn, ..
        }) => {
            let attachment = None;
            let message = SessionMessage::make_open_ack(*lease, *initial_sn, attachment);
            // Send the OpenAck
            let _ = zsend!(message, stream).unwrap();
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
    let mut c_stream = stream.clone();
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(1)).await;
            let message = SessionMessage::make_keep_alive(None, None);
            let _ = zsend!(message, c_stream);
        }
    });

    // Read from the socket
    loop {
        let n = stream.read(&mut buffer).await?;
        let _ = counter.fetch_add(n, Ordering::Relaxed);
    }
}

async fn run(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let locator = TcpListener::bind(addr).await?;
    let mut incoming = locator.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        task::spawn(async move {
            let _ = handle_client(stream).await;
        });
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name = "s_sink_tcp")]
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
