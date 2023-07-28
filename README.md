# Warp 2 Technical Demo

This demonstrates the performance of Warp Sync 2 using the compact blockchain
retrieved LightWalletd and post-processed into a WS 2 compatible form.

The data is up to date to height 2166554. It skips transactions that have more
than 50 inputs/outputs/actions. Most of them are caused by the spam attack
on the Zcash blockchain starting Jul 2022.

# Program binary

## Build from sources

To build the binary you need `rust` (and `cargo`).

Checkout the repository and run
```
git clone https://github.com/hhanh00/warp2.git
cd warp2
cargo b -r
```

This should produce a binary `target/release/warp2`

## Use a release

Github also builds release binaries from Windows and Linux.

# Data file

You also need the **blockchain data file**. It is 2.2 GB file available for 
[download](https://drive.google.com/file/d/1DjRo-J1-ob9-AQzFEPyhpBdcqU_s-RMb/view?usp=sharing)

Place it in project directory.

# Usage

```
./target/release/warp2 <FVK>
```

where FVK is the sapling full viewing key. It begins with `zxviews`.

# Video Clip - Using it with the ZecPages viewing key

[YouTube](https://youtu.be/_QMeevR4a3E)
