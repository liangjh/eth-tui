# eth-tui

A terminal-based Ethereum blockchain explorer. Browse blocks, inspect transactions, look up addresses, and monitor gas prices, all from the command line. eth-tui connects to any Ethereum JSON-RPC endpoint and presents chain data through a fast, keyboard-driven interface built on [ratatui](https://ratatui.rs/).

No API keys are required to get started. Point it at a public RPC node (or your own) and you're exploring the chain in seconds.

## Features

- **Dashboard** with recent blocks and transactions at a glance
- **Block explorer** with a scrollable list of blocks and per-block detail views including gas utilization gauges
- **Transaction inspector** with decoded method calls (ABI resolution via Sourcify, Etherscan, and built-in ERC-20/721/1155 ABIs), token transfer extraction, and full calldata display
- **Address lookup** showing ETH balance, nonce, contract detection, and transaction history
- **Gas tracker** with slow/standard/fast price estimates and a base fee history sparkline
- **Search** that auto-detects addresses, transaction hashes, and block numbers
- **Vim-style navigation** (j/k, g/G, Ctrl-D/Ctrl-U) alongside arrow keys
- **LRU caching with TTL** to minimize redundant RPC calls

## Requirements

- **Rust 1.85+** (edition 2024). Install via [rustup](https://rustup.rs/) if you don't have it:
  ```
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Access to an Ethereum JSON-RPC endpoint (HTTP). Any of the following will work:
  - A public endpoint like `https://eth.merkle.io` (the default)
  - Your own node (Geth, Reth, Erigon, etc.) at `http://localhost:8545`
  - A provider like Alchemy or Infura: `https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY`

## Installation

Clone the repository and build a release binary:

```bash
git clone https://github.com/yourusername/eth-tui.git
cd eth-tui
cargo build --release
```

The compiled binary will be at `target/release/eth-tui`.

To install it to your Cargo bin directory (`~/.cargo/bin/`):

```bash
cargo install --path .
```

## Quick Start

Run with the default public RPC endpoint:

```bash
eth-tui
```

Connect to a local node:

```bash
eth-tui --rpc-url http://localhost:8545
```

Connect to Alchemy or Infura:

```bash
eth-tui --rpc-url https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
```

## Usage

```
eth-tui [OPTIONS]

Options:
  -r, --rpc-url <RPC_URL>                  RPC endpoint URL [default: https://eth.merkle.io]
      --etherscan-api-key <API_KEY>        Etherscan API key for ABI resolution [env: ETHERSCAN_API_KEY]
  -s, --search <QUERY>                     Start with a specific search query
      --tick-rate-ms <MS>                  UI refresh interval in milliseconds [default: 100]
  -h, --help                               Print help
```

### Etherscan API Key

An Etherscan API key is optional but improves ABI resolution for verified contracts. You can pass it as a flag or set it as an environment variable:

```bash
export ETHERSCAN_API_KEY=your_key_here
eth-tui
```

Without it, eth-tui will still resolve ABIs through Sourcify and its built-in ERC-20/721/1155 function signatures.

## Keyboard Shortcuts

### Navigation

| Key | Action |
|---|---|
| `Up` / `k` | Move up |
| `Down` / `j` | Move down |
| `Enter` | Select / open detail view |
| `Esc` / `Backspace` | Go back |
| `Tab` | Switch panel (on dashboard) |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `Ctrl+D` | Page down |
| `Ctrl+U` | Page up |

### Views

| Key | Action |
|---|---|
| `1` | Dashboard |
| `2` | Block list |
| `3` | Gas tracker |

### Search

| Key | Action |
|---|---|
| `/` or `s` | Open search bar |
| `Enter` | Submit search |
| `Esc` | Cancel search |

Search accepts:
- **Addresses** (42 characters, `0x`-prefixed)
- **Transaction hashes** (66 characters, `0x`-prefixed)
- **Block numbers** (plain integers)

### Other

| Key | Action |
|---|---|
| `?` | Toggle help overlay |
| `q` | Quit |
| `Ctrl+C` | Quit |

## Architecture

```
src/
  main.rs              Entry point, CLI parsing, terminal setup
  app.rs               Main event loop, view routing, navigation stack
  config.rs            CLI argument definitions (clap)
  events.rs            Event types, search target parsing
  theme.rs             Color scheme and style constants
  utils.rs             Formatting helpers (ETH, gwei, timestamps, etc.)
  components/
    mod.rs             Component trait
    dashboard.rs       Dual-panel overview (blocks + transactions)
    block_list.rs      Scrollable block table
    block_detail.rs    Single block with gas gauge and transaction list
    tx_detail.rs       Transaction detail with decoded input and token transfers
    address_view.rs    Address balance, contract info, transaction history
    gas_tracker.rs     Gas price estimates and base fee sparkline
    header.rs          Top bar with tabs and network info
    status_bar.rs      Bottom bar with key hints and connection status
    search.rs          Popup search bar
    help.rs            Keyboard shortcut overlay
  data/
    mod.rs             DataService orchestrator (async fetch + cache + decode)
    provider.rs        Ethereum RPC wrapper (alloy)
    cache.rs           LRU cache with per-category TTL
    abi.rs             ABI resolution (Sourcify, Etherscan, built-in)
    decoder.rs         Calldata decoding and token transfer extraction
    types.rs           Domain types (blocks, transactions, addresses, gas)
abis/
  erc20.json           Standard ERC-20 ABI
  erc721.json          Standard ERC-721 ABI
  erc1155.json         Standard ERC-1155 ABI
```

The application uses a channel-based async architecture. `DataService` spawns tokio tasks for every RPC request and sends results back to the main event loop through an unbounded channel. The main loop (in `app.rs`) multiplexes three event sources with `tokio::select!`: a render tick interval, terminal keyboard events (via crossterm's async `EventStream`), and incoming data events from background tasks.

All RPC responses are cached in an LRU cache with type-specific TTLs: blocks and transactions use a 1-hour TTL (they are immutable once confirmed), balances use 30 seconds, and gas info uses 12 seconds (roughly one block).

## Running Tests

```bash
cargo test
```

The test suite covers formatting utilities, search parsing, Display trait implementations, calldata decoding, token transfer extraction, and cache behavior.

## License

MIT
