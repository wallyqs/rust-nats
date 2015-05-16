Experimental Rust client for NATS
---------------------------------

Note that this is still an experiment, it was written mostly to get
familiar with Rust, so do not use!  A proper client should be much more robust!

Here is an example of how it looks at the moment:

```rust
use std::collections::HashMap;
use std::sync::mpsc::channel;

fn main() {
    let mut nats = Client::new("127.0.0.1:4222");
    let mut opts = HashMap::new();
    opts.insert("user", "");
    opts.insert("pass", "");

    match nats.connect(&mut opts) {
        Ok(()) => println!("Successfully connected!"),
        Err(e) => println!("Failed! {}", e)
    }

    let (tx, rx) = channel();
    
    nats.subscribe("workers.double",
                   Box::new(move |msg| {
                       let tx = tx.clone();
                       let m = msg.trim();
                       let n = m.parse::<u64>().unwrap();
                       let result = n * 2;
                       println!("{} x 2 = {}", m, result);
                       tx.send("DONE!");
                   }));

    // Subscription should double the number
    nats.publish("workers.double", "20".to_string());

    // Stop when done
    let done = rx.recv().unwrap();
    println!("Status: {}", done);
}
```
