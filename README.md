# Bonkfun Copy Trading Bot (Rust)

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

A Rust implementation of a copy-trading bot for **Bonkfun**, focused on mirroring trades from selected wallets with low latency and configurable execution. This project is developed and maintained by **[DexorynLabs](https://t.me/dexoryn_here)**.

## Features

- Real-time trade mirroring from target addresses
- Low-latency execution on the Solana stack
- Private key handling via environment configuration
- Configurable parameters (slippage, fees, RPC endpoints)
- Multi-wallet workflows where supported by your setup
- Transaction logging for operational review

## Prerequisites

- **Rust** 1.70 or newer (`rustup` recommended)
- **Solana CLI** (when building or submitting Solana transactions locally)
- **Node.js** (only if you use optional frontend or tooling in this repo)

## Installation

1. **Clone the repository**

   ```bash
   git clone https://github.com/DexorynLabs/bonkfun-rust-copytrading-bot.git
   cd bonkfun-rust-copytrading-bot
   ```

2. **Configure environment**

   Copy `.env.example` to `.env` and set all required values, for example:

   ```bash
   PRIVATE_KEY= # your wallet private key
   RPC_HTTP=https://solana-rpc.publicnode.com
   RPC_WSS=wss://solana-rpc.publicnode.com
   YELLOWSTONE_GRPC_HTTP=https://solana-yellowstone-grpc.publicnode.com:443
   SLIPPAGE=10
   JITO_BLOCK_ENGINE_URL=https://ny.mainnet.block-engine.jito.wtf
   JITO_TIP_VALUE=0.0001
   ```

   Use RPC and Yellowstone endpoints that match your provider and network (mainnet, devnet, etc.).

3. **Build**

   ```bash
   cargo build --release
   ```

4. **Run**

   ```bash
   cargo run --release
   ```

## Security notice

Never commit real private keys or production RPC credentials. Treat `.env` as secret and restrict file permissions on machines where you run the bot.

## Disclaimer

Copy trading and on-chain automation carry financial and technical risk. This software is provided as-is; you are responsible for compliance with applicable laws and for any funds you deploy.

## Contact

**DexorynLabs** — questions, integrations, or collaboration:

- Telegram: [@dexoryn_here](https://t.me/dexoryn_here)
