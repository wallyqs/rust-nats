Experimental Rust client for NATS
---------------------------------

Note that this is still an experiment, it was written mostly to get
familiar with Rust, so do not use!  A proper client should be much more robust!

Here is an example of how it looks at the moment:

```rust
fn main() {
    let mut nats = Client::new(127.0.0.1:4222");
    let mut opts = HashMap::new();
    opts.insert("user", "hello");
    opts.insert("pass", "world");

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
                       let m = msg.trim();
                       let n = m.parse::<u8>().unwrap();
                       let result = n * 2;
                       println!("{} x 2 = {}", m, result);
                   }));

    // Subscription should double the number
    nats.publish("workers.double", "20".to_string());

    loop {}
}
```
