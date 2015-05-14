//
//     (The MIT License)
//
//  Copyright (c) 2015 Waldemar Quevedo. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person
//  obtaining a copy of this software and associated documentation
//  files (the "Software"), to deal in the Software without
//  restriction, including without limitation the rights to use, copy,
//  modify, merge, publish, distribute, sublicense, and/or sell copies
//  of the Software, and to permit persons to whom the Software is
//  furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufStream;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

extern crate rustc_serialize;
use rustc_serialize::json;

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

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct InfoOp  {
    server_id: String,
    host: String,
    port: i64,
    version: String,
    auth_required: bool, // FIXME: can be 'null' in older nats servers
    ssl_required:  bool,
    max_payload: i64,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct ConnectOp {
    verbose: bool,
    pedantic: bool,
    user: String,
    pass: String,
}

pub struct Client {
    io: Arc<Mutex<BufStream<TcpStream>>>,
    options:  HashMap<&'static str, &'static str>,
    ssid: u8,
    // subs: HashMap<u8, Box<Fn(&str, &Client)>>,
    subs: HashMap<u8, Box<Fn(&str)>>,
}

pub struct Parser;

impl Client {

    pub fn new(server: &str) -> Client {

        let stream = TcpStream::connect(server).unwrap();
        let mut bufstream = BufStream::new(stream);
        let mut io = Arc::new(Mutex::new(bufstream));

        return Client {
            io: io,
            options: HashMap::new(),
            ssid: 1,
            subs: HashMap::new(),
        };
    }

    pub fn connect(&self, options: &mut HashMap<&str, &str>) -> Result<(), &str> {
        println!("Connecting...");

        // TODO: Implement pongs handler callback
        if !options.contains_key("ping_interval") {
            options.insert("ping_interval", "120");
        }

        // Send connect first by processing INFO
        let line = {
            let mut l = String::new();
            let mut nats_io = self.io.clone();
            let mut io = nats_io.try_lock().unwrap();
            let result = io.read_line(&mut l).unwrap();

            // Return the line
            l
        };

        let mut proto = line.splitn(2, " ");
        let nats_op = proto.nth(0);

        match nats_op {
            Some(INFO) => {
                // Parse the connection
                println!("Got INFO!");
                let info_op: Vec<&str> = line.splitn(2, " ").collect();
                let server_info: InfoOp = json::decode(info_op[1]).unwrap();
                if server_info.auth_required {
                    // TODO: Send CONNECT with auth via options hash
                    println!("Sending CONNECT with auth...");
                    let connect_payload = json::encode(&ConnectOp{
                        pedantic: false,
                        verbose: false,
                        user: options.get("user").unwrap().to_string(),
                        pass: options.get("pass").unwrap().to_string(),
                    }).unwrap();

                    let connect_op = format!("{} {}{}", CONNECT, connect_payload.to_string(), CR_LF);

                    // TODO: Deadlock
                    // Client::send_command(connect_op.to_string(), self.io.clone());

                    let send_result = {
                        let mut nats_io = self.io.clone();
                        let mut io = nats_io.try_lock().unwrap();
                        write!(io, "{}", connect_op)
                    };

                    match send_result {
                        Err(e) => panic!("Failed to connect! {}", e),
                        Ok(_) => {
                            println!("CMD sent: {}", connect_op);
                        }
                    }
                } else {
                    // Just CONNECT
                    println!("Got INFO! Sending CONNECT without auth...");
                    let connect_payload = json::encode(&ConnectOp{
                        pedantic: false,
                        verbose: false,
                        user: "".to_string(),
                        pass: "".to_string(),
                    }).unwrap();

                    let connect_op = format!("{} {}{}", CONNECT, connect_payload.to_string(), CR_LF);

                    // TODO: Deadlock
                    // Client::send_command(connect_op.to_string(), self.io.clone());

                    let send_result = {
                        let mut nats_io = self.io.clone();
                        let mut io = nats_io.try_lock().unwrap();
                        write!(io, "{}", connect_op)
                    };

                    match send_result {
                        Err(e) => panic!("Failed to connect! {}", e),
                        Ok(_) => {
                            println!("CMD sent: {}", connect_op);
                        }
                    }
                }
            },
            Some(_) => panic!("Expected INFO from server!"),
            None => panic!("No INFO from server!"),
        }

        // Starts the thread which processes the messsages
        // and dispatches the subscription callbacks
        Parser::process_protocol(&self, self.io.clone());

        // TODO: Return proper Result type
        Ok(())
    }

    // TODO: Gracefully exit... does it make sense in Rust?
    pub fn close(&self) {
        println!("NATS client stopping...");
    }

    pub fn publish(&self, subject: &str, message: String) {
        let msg_size = message.len();
        let pub_op = format!("{} {} {}{}{}{}", PUB, subject, msg_size, CR_LF, message, CR_LF);
        Client::send_command(pub_op.to_string(), self.io.clone());
    }

    pub fn subscribe(&mut self, subject: &str, subcb: Box<Fn(&str)>) {
        self.ssid += 1; // TODO: No issues so far in borrow checker but should be Arc
        let sub_op = format!("{} {} {}{}", SUB, subject, self.ssid, CR_LF);
        Client::send_command(sub_op.to_string(), self.io.clone());

        // Store the callback to be dispatched
        &self.subs.insert(self.ssid, subcb);
    }

    // TODO: Investigate deadlock
    pub fn send_command(command: String, eio: Arc<Mutex<BufStream<TcpStream>>>) {
        println!("About to send: {}", command);

        let mut nats_io = eio.clone();
        let mut io = nats_io.try_lock().unwrap();
        let send_result = write!(io, "{}", command);

        match send_result {
            Err(e) => panic!("Failed to connect! {}", e),
            Ok(_) => {
                println!("CMD sent: {}", command);
            }
        }

        io.flush().unwrap();
    }

    // TODO: Figure out how to fetch exact number of bytes from the stream
    //       also, deadlock
    pub fn read_message_payload(msg_size: u64, eio: Arc<Mutex<BufStream<TcpStream>>>) -> String {
        println!("Will try to read the next line.....");
        let mut nats_io = eio.clone();
        let mut io = nats_io.try_lock().unwrap();
        let mut payload = String::new();

        // TODO: Figure out how to get N bytes
        // let result = io.take(msg_size);
        // let result = io.read_line(&mut payload).unwrap();

        println!("Read the message!");
        return payload;
    }
}

impl Parser {

    pub fn process_protocol(nats: &Client, eio: Arc<Mutex<BufStream<TcpStream>>>) {

        thread::spawn(move || {
            loop {
                // Acquire the io resource and release by scoping its lifetime
                let line = {
                    let mut l = String::new();
                    let mut nats_io = eio.clone();
                    let mut io = nats_io.try_lock().unwrap();
                    let result = io.read_line(&mut l).unwrap();

                    // return the line
                    l
                };

                // Skip empty lines
                if line.len() < 1 { continue }

                let mut proto = line.splitn(2, " ");
                let nats_op = proto.nth(0);

                // TODO: When handling the messsage other clients set a flag like:
                //        AWAITING_MSG_PAYLOAD or AWAITING_CONTROL
                // so that the line is buffered and then captured in the next loop
                // then at the end reset the buffer, figure out how to do this in Rust
                match nats_op {
                    Some(INFO) => { /* TODO: connect in case no connection already? */ },
                    Some(PING) => {
                        println!("Got PING -> Sending PONG...");
                        Client::send_command(PONG.to_string(), eio.clone());
                    },
                    Some(PONG) => {
                        println!("Got PONG from server");
                        // TODO: After starting the ping interval callback
                        //       keep track of the pongs received and close the connection
                        //       in case we have more than `max_outstanding_pings`
                        Client::send_command(PING.to_string(), eio.clone());
                    },
                    Some(MSG)  => {
                        println!("Got a MSG: {}", line);

                        let msg_op: Vec<&str> = line.split(" ").collect();
                        let mut msg_op_count = msg_op.len();
                        match msg_op_count {
                            4 => {
                                // PUB: MSG workers.double 3 8
                                println!("Simple PUB");
                                let msg_subject = msg_op[1];
                                let msg_sub_id = msg_op[2];
                                let mut msg_size = msg_op[3].trim().parse::<u64>().unwrap();
                                println!("Reading MSG payload for: {}", msg_subject);

                                // TODO: Deadlock??
                                // let mut msg_payload = Client::read_message_payload(msg_size, eio.clone());

                                let mut msg_payload = {
                                    let mut l = String::new();
                                    let mut nats_io = eio.clone();
                                    let mut io = nats_io.try_lock().unwrap();

                                    // TODO: Get bytes we want
                                    let result = io.read_line(&mut l).unwrap();

                                    // return the line
                                    l
                                };

                                println!("Got: {}", msg_payload);

                                // TODO: Dispatch the callback using proper type
                                //
                                // let cb = {
                                //     let subs = nats.subs;
                                //     subs.get(&2).unwrap()
                                // };
                            },
                            5 => {
                                // TODO: Implement request pattern
                                // REQ: MSG workers.double 3 _INBOX.aa8493b6562cd616e899f86147 8
                                println!("Request Pattern");
                                let msg_subject = msg_op[1];
                                let msg_sub_id = msg_op[2];
                                let msg_inbox = msg_op[3];
                                let msg_size = msg_op[4];
                            },
                            _ => {
                                println!("Malformed MSG: {}", msg_op_count);
                            },
                        }
                    },
                    Some(OK)   => { /* ignore */  },
                    Some(ERR)  => println!("Error in the protocol: {}", line),
                    Some(_)    => println!("Unknown Protocol: {}", line),
                    None       => println!("No Protocol: {}", line),
                }
            }
        });
    }
}

fn main() {
    println!("--------- Rust NATS client prototype --------");

    let mut nats = Client::new("10.42.0.36:4222");
    let mut opts = HashMap::new();

    match nats.connect(&mut opts) {
        Ok(()) => println!("Successfully connected!"),
        Err(e) => println!("Failed! {}", e)
    }

    nats.subscribe("workers.results",
                   Box::new(|msg| {
                       println!("Results: {}", msg);
                   }));

    nats.subscribe("workers.double",
                   Box::new(|msg| {
                       let n = msg.parse::<u8>().unwrap();
                       let result = n * 2;
                       println!("{} x 2 = {}", msg, result);

                       // TODO: Borrow again the connection to publish the result
                       // nats.publish("workers.results", result.to_string());
                   }));

    // Subscription should double the number
    nats.publish("workers.double", "10".to_string());

    loop {}
}