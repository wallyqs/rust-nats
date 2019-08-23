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

// #[derive(Serialize, Deserialize, Debug)]
// struct ServerInfo {
//     server_id: String,
//     host: String,
//     port: i64,
//     version: String,
//     auth_required: bool,
//     tls_required:  bool,
//     max_payload: i64,
// }

#[derive(Serialize, Deserialize, Debug)]
struct ConnectOp {
    verbose: bool,
    pedantic: bool,
}

fn main() -> io::Result<()> {
    // TODO: should be
    // async fn process_connect_init
    task::block_on(async {
        // Raw I/O
        let mut socket = TcpStream::connect("127.0.0.1:4222").await?;
        let mut stream = BufReader::new(&mut socket);

        // TODO: Borrow checker complains here.
        // println!("Connected to {}", &socket.peer_addr()?);

        let info_line = {
            let mut l = String::new();
            stream.read_line(&mut l).await?;
            l
        };
        // println!("-> {}", info_line);
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

        // TODO: unwrap
        let connect_payload = serde_json::to_string(&ConnectOp{
            pedantic: false,
            verbose: false,
        }).unwrap();

        let connect_op = format!("{} {}{}", CONNECT, connect_payload.to_string(), CR_LF);
        socket.write_all(connect_op.as_bytes()).await?;

        let ping_op = format!("{}", PING);
        socket.write_all(ping_op.as_bytes()).await?;

        // Wait for pong
        // let mut pong_line = String::new();
        // stream.read_line(&mut pong_line).await?;
        // println!("-> {}", pong_line);
        // match pong_line {
        //     "PONG\r\n" => {
        //         println!("Got PONG");
        //     },
        // }

        // TODO: map of callbacks
        let sub_op = format!("{} hello 1 {}", SUB, CR_LF);
        socket.write_all(sub_op.as_bytes()).await?;

        // Need to borrow once again here somehow.
        // async fn foo(s: &mut TcpStream){
        async fn foo(){
            // PONG back here.
            println!("Got message!!!!!!");
            // let pong_op = format!("{}", PONG);
            // socket.write_all(pong_op.as_bytes()).await;
        };

        // let foo = async move {
        //     println!("Got messasge!!!!!!!!!");
        // };

        // reader
        task::spawn(async move {
            // TODO: borrowed value does not live enough
            println!("waiting for messages");
            // try with clone?
            // let mut ss2 = &mut socket;
            // let mut str2 = BufReader::new(ss2);
            loop {
                // Await the line.
                let line = {
                    let mut str2 = BufReader::new(&mut socket);
                    let mut l = String::new();
                    str2.read_line(&mut l).await;
                    l
                };
                // Await the payload waiting for number of bytes.
                println!("{}", line);

                // Take copy of payload here...
                // foo(&mut socket).await;
                // foo(&mut ss2).await;
                let pub_op = format!("{} foo 0{}{}", PUB, CR_LF, CR_LF);
                socket.write_all(pub_op.as_bytes()).await;
                foo().await;
            }
        });
        // task::block_on(client_task);

        // select {} || try to publish here too
        task::block_on(async {
            loop {
                // let line = {
                //     let mut l = String::new();
                //     // nst.read_line(&mut l).await;
                //     l
                // };
                // println!("------------LINE: {}", line);
                println!("running...");
                task::sleep(Duration::from_millis(2000)).await;
            }
        });

        // TODO: Run forever and execute the callback dynamically defined.
        // let future = async move {
        //     // Scope of the future, request response
        //     // would work like this.
        //     // Ok(())
        // };
        
        Ok(())
    })
}
