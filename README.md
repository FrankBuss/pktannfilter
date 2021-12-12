# pktannfilter

This program filters the output (stdout and stderr) of the packetcrypt_rs miner program. It removes all WARN lines which contains the strings `Error uploading ann batch` and `Failed to make request to`.

It also adds the pool names to the `goodrate` output and uses colors for it.

# how to compile and run the program

First install Rust, e.g. from https://rustup.rs. Then you can build the program with `cargo build --release`.

For starting the filter, specify the original mining program as the first program argument, and then the other parameters for the mining program itself as usual. For example:

```
./release/pktannfilter ../packetcrypt_rs/target/release/packetcrypt ann 'http://pool.pktpool.io' 'http://pool.pkt.world'  --paymentaddr pkt1xxx`
```

(instead of `pkt1xxx`, use your wallet address)

The shell script `miner-simulation` outputs some simulated miner lines on stderr to demonstrate how the filter works. If you run the script `miner-simulation`, you'll see the unmodified output. If you start it with the pktannfilter (you can use the `test` script for it), then you can see how the output is modified.
