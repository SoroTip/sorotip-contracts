# SoroTip Contracts

**On-chain tipping and creator monetization protocol on Stellar Soroban**

[![Rust](https://img.shields.io/badge/Rust-1.84%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-22.0.0-blue)](https://developers.stellar.org/docs/build/smart-contracts/overview)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Stellar Network](https://img.shields.io/badge/Stellar-Network-brightgreen?logo=stellar)](https://stellar.org)
[![Drips Wave](https://img.shields.io/badge/Drips-Wave%20Program-8A2BE2)](https://drips.network/wave)

## What is SoroTip

SoroTip is the first on-chain tipping and creator monetization protocol built on
Stellar Soroban. Anyone can send USDC tips to any Stellar wallet — one-time or
recurring monthly — and any creator can publish a public funding page that
supporters can back directly, with no platform in between.

Tips and subscription charges move **directly from a supporter's wallet to a
creator's wallet in a single transfer**. This contract never custodies funds:
it is a routing and record-keeping layer, not a wallet.

## The problem it solves

Content creators, independent developers, musicians, and open-source
contributors across Africa and LATAM are frequently locked out of PayPal,
Stripe, and Patreon due to geographic payout restrictions, even when their
audience and income are entirely legitimate. SoroTip removes that gatekeeper:
anyone with a Stellar wallet — which anyone can create for free, from
anywhere — gets a global monetization tool with near-zero transaction fees
and no custody risk.

## How it works

**One-time tips.** A creator registers a public profile, shares their
`sorotip.app/tip/<address>` link, and a supporter tips them directly from
their wallet — USDC moves from supporter to creator in the same transaction
that records the tip.

**Recurring subscriptions.** A supporter opens a monthly subscription to a
creator. No funds move at subscription time; instead the supporter approves
the contract as a token spender, and any "keeper" (anyone, including an
automated cron job) can call `process_due_subscriptions` to charge every
subscription that's come due, routing funds directly to each creator.

**Tip goals.** A creator can publish a funding goal with a target amount and
description. Every tip and subscription charge toward that creator counts
against the goal automatically, and the contract emits a `GoalReached` event
the moment the target is hit.

## Tech Stack

- **Rust** 1.84+
- **soroban-sdk** 22.0.0
- **stellar-cli** for building and deploying to testnet/mainnet

## Local Setup

```bash
# Install Rust (if you don't already have it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the Soroban-compatible wasm target
rustup target add wasm32v1-none

# Install the Stellar CLI
cargo install --locked stellar-cli --features opt

# Clone and build
git clone https://github.com/SoroTip/sorotip-contracts.git
cd sorotip-contracts
cargo build --workspace

# Run the full test suite
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Build the optimized wasm binary
cargo build --target wasm32v1-none -p tip --release
```

> **Note on dependencies:** `soroban-env-host` 22.1.0 (pulled in transitively by
> `soroban-sdk` 22.0.0) declares an unbounded `ed25519-dalek = ">=2.0.0"`
> requirement. Since `ed25519-dalek` 3.0.0 was published, that unbounded range
> drifts to a version whose `rand_core` requirement is incompatible with the
> rest of the dependency tree, breaking `cargo test`'s testutils. This repo
> pins the workspace to a vendored, known-good `ed25519-dalek` 2.2.0 (see
> `vendor/` and the `[patch.crates-io]` section of the root `Cargo.toml`) so
> the whole graph resolves to a single, compatible version.

## Contract Functions

| Function | Description | Parameters | Returns |
|---|---|---|---|
| `initialize` | One-time setup of the contract admin, protocol fee, treasury, and token. | `admin: Address, fee_basis_points: u32, treasury: Address, token: Address` | — |
| `register_creator` | Creates a creator's public profile. Idempotent — returns the existing id if already registered. | `wallet: Address, name: String, bio: String, avatar_ipfs: String` | `u64` (profile id) |
| `update_profile` | Updates an existing creator's name, bio, and avatar. | `wallet: Address, name: String, bio: String, avatar_ipfs: String` | — |
| `tip` | Sends a one-time tip directly from supporter to creator, minus the protocol fee. | `from: Address, to: Address, amount: i128, message_ipfs: String` | `u64` (tip id) |
| `subscribe` | Opens a recurring monthly subscription. | `from: Address, to: Address, amount_per_month: i128` | `u64` (subscription id) |
| `cancel_subscription` | Cancels an active subscription. Only the original supporter may call this. | `from: Address, subscription_id: u64` | — |
| `process_due_subscriptions` | Charges every subscription due today. Callable by anyone. | — | `u32` (number charged) |
| `set_tip_goal` | Publishes a new funding goal for a creator, replacing any previous one. | `creator: Address, goal_amount: i128, description: String` | `u64` (goal id) |
| `complete_tip_goal` | Manually marks a goal complete (goals also auto-complete when the target is reached). | `creator: Address, goal_id: u64` | — |
| `get_profile` | Reads a creator's public profile. | `wallet: Address` | `CreatorProfile` |
| `get_tip_history` | Most recent tips involving a wallet, as sender or recipient. | `wallet: Address, limit: u32` | `Vec<Tip>` |
| `get_subscriptions_by_supporter` | All subscriptions a supporter has opened. | `supporter: Address` | `Vec<Subscription>` |
| `get_subscriptions_by_creator` | All subscriptions a creator has received. | `creator: Address` | `Vec<Subscription>` |
| `get_tip_goal` | A creator's current funding goal, if any. | `creator: Address` | `Option<TipGoal>` |
| `get_protocol_stats` | Aggregate protocol-wide statistics. | — | `ProtocolStats` |
| `get_top_creators` | Leaderboard of top creators by total USDC received. | `limit: u32` | `Vec<LeaderboardEntry>` |
| `update_fee` | Updates the protocol fee. Admin only. | `admin: Address, fee_basis_points: u32` | — |
| `get_fee` | Reads the current protocol fee, in basis points. | — | `u32` |

## Testnet Deployment

Deploy manually with the Stellar CLI once you have a funded testnet identity:

```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

stellar keys generate deployer --network testnet --fund

stellar contract build

WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32v1-none/release/tip.wasm \
  --source deployer \
  --network testnet)

CONTRACT_ID=$(stellar contract deploy \
  --wasm-hash "$WASM_HASH" \
  --source deployer \
  --network testnet)
```

Record the resulting contract ID in
[`deployments/testnet.json`](./deployments/testnet.json):

```json
{
  "TipContract": "",
  "network": "testnet",
  "deployedAt": ""
}
```

## Contributing via Drips Wave

This repo is part of the [Stellar Wave Program](https://drips.network/wave)
on Drips. Contributors browse open issues, get assigned by the maintainer,
and earn USDC rewards for merged pull requests that resolve an issue. See
[CONTRIBUTING.md](./CONTRIBUTING.md) for the full workflow.

👉 https://drips.network/wave
