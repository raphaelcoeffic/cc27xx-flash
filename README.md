# cc27xx-flash

A [probe-rs](https://probe.rs) flash algorithm for the TI CC27xx (CC2745, CC2755): main
flash in 2 KB sectors, the CCFG at 0x4E02_0000 and the SCFG at 0x4E04_0000.

Erase and program call the flash functions of the boot ROM, found through its function
table (driverlib `hapi.h`). The cache is turned off while the algorithm runs.

The CCFG is erased and written only when it differs from the image: an erased CCFG keeps
the device from booting. The boot ROM can't write the SCFG: programming it fails with
0x5CF6_0000 unless the device already holds the image's SCFG. Only SACI writes it, after
a chip erase. There is no chip erase either.

`VendorFunc_Signatures_md5` signs flash ranges with MD5, so probe-rs compares them without
reading them back (`--preverify`, `--verify`).

## Building

```sh
rustup target add thumbv7m-none-eabi
cargo build --release
```

## Updating probe-rs

```sh
target-gen elf --fixed-load-address --name cc27xx-flash \
    target/thumbv7m-none-eabi/release/cc27xx-flash cc27xx.yaml
```

and copy the algorithm into `probe-rs/targets/CC23XX_CC27XX_Series.yaml`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
