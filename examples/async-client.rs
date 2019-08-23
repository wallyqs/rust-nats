//! TCP client.
//!
//! Run the client:
//!
//! ```sh
//! $ cargo run --example async-client
//! ```

#![feature(async_closure)]

use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;
use async_std::io::BufReader;
use serde::{Serialize, Deserialize};
use std::time::Duration;

// Protocol
const CONNECT: &'static str = "CONNECT";
const INFO:    &'static str = "INFO";
const PING:    &'static str = "PING\r\n";
const PONG:    &'static str = "PONG\r\n";
const PUB:     &'static str = "PUB";
const SUB:     &'static str = "SUB";
const UNSUB:   &'static str = "UNSUB";
const MSG:     &'static str = "MSG";
const OK:      &'static str = "+OK\r\n";
const ERR:     &'static str = "-ERR";
const CR_LF:   &'static str = "\r\n";
const SPC:     &'static str = " ";

#[derive(Serialize, Deserialize, Debug)]
struct ConnectOp {
    verbose: bool,
    pedantic: bool,
}

fn main() -> io::Result<()> {
    // async fn process_connect_init
    task::block_on(async {
        // Raw I/O
        let mut socket = TcpStream::connect("127.0.0.1:4222").await?;
        let mut stream = BufReader::new(&mut socket);

        let info_line = {
            let mut l = String::new();
            let mut stream = BufReader::new(&mut socket);
            stream.read_line(&mut l).await?;
            l
        };
        let mut proto = info_line.splitn(2, " ");
        let nats_op = proto.nth(0);

        // TODO: Not best way of doing this probably...
        match nats_op {
            Some(INFO) => {
                println!("Got INFO: {:?}", proto.nth(0));
            },
            Some(_) => panic!("No INFO from server!"),
            None => panic!("No INFO from server!"),
        }

        // FIXME: unwrap
        let connect_payload = serde_json::to_string(&ConnectOp{
            pedantic: false,
            verbose: false,
        }).unwrap();

        let connect_op = format!("{} {}{}", CONNECT, connect_payload.to_string(), CR_LF);
        socket.write_all(connect_op.as_bytes()).await?;

        let ping_op = format!("{}", PING);
        socket.write_all(ping_op.as_bytes()).await?;

        // Wait for pong...
        let pong_line = {
            let mut l = String::new();
            let mut stream = BufReader::new(&mut socket);
            stream.read_line(&mut l).await?;
            l
        };

        // TODO: map of callbacks ((Arc, Mutex, [sid] -> async Fn))?
        let sub_op = format!("{} hello 1 {}", SUB, CR_LF);
        socket.write_all(sub_op.as_bytes()).await?;

        // Need to borrow once again here somehow.
        async fn foo(s: &mut TcpStream){
            println!("->> Sending...");
            let pub_op = format!("{} foo 0{}{}", PUB, CR_LF, CR_LF);
            s.write_all(pub_op.as_bytes()).await;
        };

        async fn bar(s: &mut TcpStream){
            println!("->> Sending...");
            let pub_op = format!("{} bar 0{}{}", PUB, CR_LF, CR_LF);
            s.write_all(pub_op.as_bytes()).await;
        };

        // --- connect init ends ownership here and passes it to the read_task.

        // 'read_loop' task
        task::spawn(async move {
            loop {
                // Await the line.
                let line = {
                    // TODO: Borrow each time here??
                    let mut stream = BufReader::new(&mut socket);
                    let mut l = String::new();
                    stream.read_line(&mut l).await;
                    l
                };
                // TODO: Await the payload and make a read for number of bytes.
                println!("<<- Received.. {:?}", line);

                // TODO: callbacks that can be defined dynamically.
                foo(&mut socket).await;
                bar(&mut socket).await;
            }
        });

        // 'select {}'
        task::block_on(async {
            loop {
                task::sleep(Duration::from_millis(2000)).await;
            }
        });
        
        Ok(())
    })
}
