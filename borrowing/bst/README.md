Set backed by Binary Search Tree

## Benchmarks

Benchmark tool for cargo:
```
cargo install cargo-criterion
```

Start benchmarks: `make bench`. Author's results:

```
100k_random_insertions/rust_BstSet                                                                            
                        time:   [15.117 ms 15.165 ms 15.220 ms]
100k_random_insertions/c_BstSet                                                                            
                        time:   [12.708 ms 12.745 ms 12.788 ms]

100k_random_lookup_hits/rust_BstSet                                                                            
                        time:   [12.257 ms 12.298 ms 12.346 ms]
100k_random_lookup_hits/c_BstSet                                                                            
                        time:   [8.8203 ms 8.8449 ms 8.8745 ms]

100k_random_lookup_misses/rust_BstSet                                                                            
                        time:   [14.652 ms 14.762 ms 14.892 ms]
100k_random_lookup_misses/c_BstSet                                                                            
                        time:   [11.444 ms 11.577 ms 11.723 ms]
```