# eth-tui

A terminal-based Ethereum blockchain explorer. Browse blocks, inspect transactions, look up addresses, decode contract calls, monitor gas prices, watch the mempool, and debug transaction execution, all from the command line. eth-tui connects to any Ethereum JSON-RPC endpoint and presents chain data through a fast, keyboard-driven interface built on [ratatui](https://ratatui.rs/).

No API keys are required to get started. Point it at a public RPC node (or your own) and you're exploring the chain in seconds. Multi-chain support lets you switch between Ethereum, Arbitrum, Optimism, Base, and Polygon with a single flag.

<img width="1312" height="617" alt="image" src="https://github.com/user-attachments/assets/0e998a27-25eb-4d14-8d58-2ca4fe2969e4" />


## Features

### Core Explorer
- **Dashboard** with recent blocks and transactions at a glance
- **Block explorer** with a scrollable block list, per-block detail views, gas utilization gauges, and ETH burned per block
- **Transaction inspector** with decoded method calls (ABI resolution via Sourcify, Etherscan, and built-in ERC-20/721/1155 ABIs), token transfer extraction, internal transaction traces, and decoded event logs
- **Address lookup** showing ETH balance, nonce, contract detection, proxy detection (EIP-1967), implementation address resolution, and transaction history (via Etherscan API)
- **Gas tracker** with slow/standard/fast price estimates, base fee history sparkline, blob base fee display, priority fee percentile distribution, and network congestion indicator
- **Search** that auto-detects addresses, transaction hashes, block numbers, and ENS names

### Live Data
- **WebSocket subscriptions** for real-time new block headers and pending transactions (with automatic reconnection and exponential backoff)
- **Mempool viewer** showing pending transactions from the network, sorted by gas price

### Smart Contract Tools
- **Contract read interface** for calling view/pure functions on verified contracts with parameter input and result display
- **Storage inspector** for querying arbitrary storage slots on any contract, with hex and decimal value display
- **Event log decoding** that matches log topics against known ABIs and decodes indexed and non-indexed parameters
- **Proxy contract detection** that reads the EIP-1967 implementation slot and resolves the underlying implementation's ABI
- **Method name resolution** via ABI lookup, Sourcify, Etherscan, and built-in selector matching

### Data & Analysis
- **ENS resolution** for looking up addresses by `.eth` name (namehash per EIP-137, direct registry + resolver calls)
- **Token metadata enrichment** for ERC-20 tokens (name, symbol, decimals) via on-chain reads batched through Multicall3
- **Internal transaction tracing** via `trace_transaction` (Parity/OpenEthereum) or `debug_traceTransaction` with callTracer (Geth/Reth)
- **Burn tracker** showing ETH burned per block (base fee * gas used) in both block list and detail views
- **Gas intelligence** with priority fee percentiles (10th/25th/50th/75th/90th) and congestion detection

### Power User Tools
- **Watch list** with persistent storage (`~/.config/eth-tui/watchlist.json`), custom labels, and live balance display
- **Transaction debugger** with opcode-level execution trace, step-by-step navigation, stack display, and CALL/CREATE/REVERT highlighting
- **Export to CSV/JSON** for blocks (CSV), transactions (JSON), and address info (JSON)
- **Multicall batching** for efficient on-chain reads via Multicall3 aggregate3
- **L2/Multi-chain support** with built-in presets for Ethereum, Arbitrum, Optimism, Base, and Polygon

### Navigation
- **Vim-style navigation** (`j`/`k`, `g`/`G`, `Ctrl-D`/`Ctrl-U`) alongside arrow keys
- **LRU caching with TTL** to minimize redundant RPC calls

## Requirements

- **Rust 1.85+** (edition 2024). Install via [rustup](https://rustup.rs/) if you don't have it:
  ```
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Access to an Ethereum JSON-RPC endpoint (HTTP). You do **not** need to run your own node. See the next section for details.

## Connecting to Ethereum Mainnet

eth-tui connects to Ethereum through any standard JSON-RPC endpoint over HTTP. You do not need to run your own node. By default it uses `https://eth.merkle.io`, a free public endpoint, so running `eth-tui` with no flags will connect you to mainnet immediately.

### Free Public RPC Endpoints (no account required)

These are community/public endpoints that work out of the box with no signup or API key:

| Provider | URL |
|---|---|
| Merkle (default) | `https://eth.merkle.io` |
| CloudFlare | `https://cloudflare-eth.com` |
| PublicNode | `https://ethereum-rpc.publicnode.com` |
| 1RPC | `https://1rpc.io/eth` |
| DRPC | `https://eth.drpc.org` |

To use one of these alternatives:

```bash
eth-tui --rpc-url https://cloudflare-eth.com
```

Public endpoints are rate-limited and best suited for casual browsing. If you experience slow responses or errors, try a different one from the list above, or use a provider account for higher limits.

### Provider Accounts (free tier, signup required)

For heavier usage, services like Alchemy and Infura offer generous free tiers with higher rate limits:

- **Alchemy**: `https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY` ([sign up](https://www.alchemy.com/))
- **Infura**: `https://mainnet.infura.io/v3/YOUR_KEY` ([sign up](https://www.infura.io/))
- **Ankr**: `https://rpc.ankr.com/eth/YOUR_KEY` ([sign up](https://www.ankr.com/))

```bash
eth-tui --rpc-url https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
```

### Local Node

If you run your own node (Geth, Reth, Erigon, Nethermind, etc.), point eth-tui at its RPC port:

```bash
eth-tui --rpc-url http://localhost:8545
```

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

Connect to Ethereum mainnet using the default public endpoint (no configuration needed):

```bash
eth-tui
```

Or specify a different RPC endpoint:

```bash
eth-tui --rpc-url https://cloudflare-eth.com
```

Jump directly to a block, transaction, or address:

```bash
eth-tui --search 19000000
eth-tui --search 0x1234...abcd
eth-tui --search vitalik.eth
```

Explore an L2 chain:

```bash
eth-tui --chain arbitrum
eth-tui --chain optimism
eth-tui --chain base
eth-tui --chain polygon
```

Enable live WebSocket subscriptions for real-time block and pending transaction updates:

```bash
eth-tui --ws-url wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
```

## Usage

```
eth-tui [OPTIONS]

Options:
  -r, --rpc-url <RPC_URL>              RPC endpoint URL [default: https://eth.merkle.io]
      --etherscan-api-key <API_KEY>    Etherscan API key for ABI resolution [env: ETHERSCAN_API_KEY]
  -s, --search <QUERY>                 Start with a specific search query
      --ws-url <WS_URL>               WebSocket endpoint for live subscriptions
      --chain <CHAIN>                  Chain preset: ethereum, arbitrum, optimism, base, polygon
                                       [default: ethereum]
      --tick-rate-ms <MS>              UI refresh interval in milliseconds [default: 100]
  -h, --help                           Print help
```

### Etherscan API Key

An Etherscan API key is optional but improves ABI resolution for verified contracts and enables address transaction history. You can pass it as a flag or set it as an environment variable:

```bash
export ETHERSCAN_API_KEY=your_key_here
eth-tui
```

Without it, eth-tui will still resolve ABIs through Sourcify and its built-in ERC-20/721/1155 function signatures.

### WebSocket Subscriptions

Connecting a WebSocket endpoint enables real-time data:
- **New block headers** appear in the dashboard and block list as they are mined
- **Pending transactions** stream into the mempool viewer

The WebSocket connection automatically reconnects with exponential backoff if it drops. The status bar shows the current connection state.

```bash
# Alchemy WebSocket
eth-tui --ws-url wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY

# Local node
eth-tui --ws-url ws://localhost:8546
```

### Multi-Chain Support

Use the `--chain` flag to connect to a supported L2/sidechain with preconfigured RPC endpoints:

| Chain | Flag | Aliases | Native Symbol |
|---|---|---|---|
| Ethereum | `--chain ethereum` | `eth`, `mainnet` | ETH |
| Arbitrum One | `--chain arbitrum` | `arb` | ETH |
| Optimism | `--chain optimism` | `op` | ETH |
| Base | `--chain base` | | ETH |
| Polygon | `--chain polygon` | `matic` | MATIC |

The `--chain` flag sets the default RPC endpoint. You can override it with `--rpc-url`:

```bash
# Use Arbitrum's default public RPC
eth-tui --chain arbitrum

# Use a custom Arbitrum RPC endpoint
eth-tui --chain arbitrum --rpc-url https://arb-mainnet.g.alchemy.com/v2/YOUR_KEY
```

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
| `4` | Watch list |
| `5` | Mempool |

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
- **ENS names** (e.g., `vitalik.eth`)

### Context Actions

These keys are available in specific views:

| Key | Context | Action |
|---|---|---|
| `w` | Address view | Add address to watch list |
| `r` | Address view (contract) | Open contract read interface |
| `S` | Address view (contract) | Open storage inspector |
| `d` | Transaction detail | Open transaction debugger |
| `e` | Any detail view | Export current view data to file |

### Storage Inspector

| Key | Action |
|---|---|
| `i` | Enter slot number input mode |
| `Enter` | Query the storage slot |
| `Esc` | Exit input mode / go back |
| `j` / `k` | Navigate results |

### Contract Read Interface

| Key | Action |
|---|---|
| `j` / `k` | Navigate function list |
| `Enter` | Select function / submit call |
| `Tab` | Move between parameter fields |
| `Esc` | Go back |

### Transaction Debugger

| Key | Action |
|---|---|
| `j` / `k` | Step through execution trace |
| `g` / `G` | Jump to start / end of trace |
| `Ctrl+D` / `Ctrl+U` | Page through trace |
| `Esc` | Go back |

### Watch List

| Key | Action |
|---|---|
| `a` | Add new address |
| `d` | Delete selected address |
| `Enter` | View address details |
| `j` / `k` | Navigate list |

### Other

| Key | Action |
|---|---|
| `?` | Toggle help overlay |
| `q` | Quit |
| `Ctrl+C` | Quit |

## Feature Details

### ENS Resolution

Search for any `.eth` name and eth-tui will resolve it to an address via on-chain ENS registry calls. This works without any external API -- the resolver performs namehash computation per EIP-137 and calls the ENS registry contract directly.

### Proxy Detection

When viewing a contract address, eth-tui automatically checks the EIP-1967 implementation storage slot (`0x360894...`). If a proxy is detected, the implementation address is displayed and its ABI is loaded for decoding.

### Internal Transactions

Transaction detail views show internal calls (CALL, DELEGATECALL, CREATE, etc.) traced via `trace_transaction` (Parity-compatible nodes) or `debug_traceTransaction` with the `callTracer` preset (Geth/Reth). Each internal call shows the call type, from/to addresses, value transferred, and depth level.

Note: Internal transaction tracing requires an archive node or a node with tracing APIs enabled.

### Token Metadata

When ERC-20 token transfers are detected in a transaction, eth-tui fetches token metadata (name, symbol, decimals) via on-chain calls. Multiple tokens are batched through Multicall3 for efficiency.

### Export

Press `e` in any detail view to export data:
- **Block list** exports to CSV with columns for block number, hash, timestamp, tx count, gas used, base fee, and ETH burned
- **Transaction detail** exports to JSON with all decoded information
- **Address info** exports to JSON with balance, nonce, contract info, and transaction history

Files are written to the current directory with descriptive filenames (e.g., `blocks_19000000_19000009.csv`).

### Watch List

The watch list persists across sessions at `~/.config/eth-tui/watchlist.json`. Add addresses with custom labels and monitor their ETH balances. Press `Enter` on any watched address to jump to its detail view.

### Burn Tracker

Every block shows the amount of ETH burned (base fee * gas used), visible both in the block list table as a "Burned" column and in the block detail view as a dedicated row.

### Gas Intelligence

The gas tracker shows more than just current gas prices:
- **Priority fee percentiles** (10th through 90th) give a distribution of recent tip levels
- **Network congestion indicator** flags when the base fee exceeds 100 gwei
- **Blob base fee** (EIP-4844) is displayed when available

## Architecture

```
src/
  main.rs                Entry point, CLI parsing, terminal setup, chain config
  app.rs                 Main event loop, view routing, navigation stack
  config.rs              CLI argument definitions (clap)
  events.rs              Event types, search target parsing
  theme.rs               Color scheme and style constants
  utils.rs               Formatting helpers (ETH, gwei, timestamps, etc.)
  components/
    mod.rs               Component trait
    dashboard.rs         Dual-panel overview (blocks + transactions)
    block_list.rs        Scrollable block table with burn column
    block_detail.rs      Single block with gas gauge, tx list, burn display
    tx_detail.rs         Transaction detail with decoded input, token transfers,
                           internal transactions, and decoded events
    address_view.rs      Address balance, contract/proxy info, tx history
    gas_tracker.rs       Gas prices, base fee sparkline, percentiles, blob fee
    contract_read.rs     Interactive contract function caller
    watch_list.rs        Persistent watch list with balances
    mempool.rs           Live pending transaction viewer
    tx_debugger.rs       Opcode-level transaction execution trace
    storage_inspector.rs Storage slot query interface
    header.rs            Top bar with tabs, chain name, and network info
    status_bar.rs        Bottom bar with key hints and WebSocket status
    search.rs            Popup search bar
    help.rs              Keyboard shortcut overlay
  data/
    mod.rs               DataService orchestrator (async fetch + cache + decode)
    provider.rs          Ethereum RPC wrapper (alloy) with Multicall3
    cache.rs             LRU cache with per-category TTL
    abi.rs               ABI resolution (Sourcify, Etherscan, built-in)
    decoder.rs           Calldata decoding, token transfer extraction, event log decoding
    types.rs             Domain types (blocks, transactions, addresses, gas, traces)
    ens.rs               ENS resolution (EIP-137 namehash, registry + resolver calls)
    chains.rs            L2/multi-chain presets (Ethereum, Arbitrum, Optimism, Base, Polygon)
    export.rs            CSV and JSON export for blocks, transactions, addresses
    watchlist.rs         Persistent watch list storage
    ws.rs                WebSocket subscription service (newHeads, pendingTransactions)
abis/
  erc20.json             Standard ERC-20 ABI
  erc721.json            Standard ERC-721 ABI
  erc1155.json           Standard ERC-1155 ABI
```

The application uses a channel-based async architecture. `DataService` spawns tokio tasks for every RPC request and sends results back to the main event loop through an unbounded channel. The main loop (in `app.rs`) multiplexes three event sources with `tokio::select!`: a render tick interval, terminal keyboard events (via crossterm's async `EventStream`), and incoming data events from background tasks.

All RPC responses are cached in an LRU cache with type-specific TTLs: blocks and transactions use a 1-hour TTL (they are immutable once confirmed), token metadata uses a 1-hour TTL, balances use 30 seconds, and gas info uses 12 seconds (roughly one block).

When a WebSocket connection is configured, `WsService` runs a background task that subscribes to `newHeads` and `newPendingTransactions` streams. Events are forwarded through the same channel as RPC responses. The connection automatically reconnects with exponential backoff (1s to 30s) on failure.

## Running Tests

```bash
cargo test
```

The test suite (75 tests) covers:
- Formatting utilities (ETH, gwei, timestamps, gas usage, selectors)
- Search target parsing (addresses, tx hashes, block numbers)
- Display trait implementations (TxType, TxStatus, ContractType)
- Calldata decoding (ERC-20 transfer ABI decode)
- Token transfer extraction from event logs
- Event log decoding
- Cache behavior (LRU eviction, TTL, per-category storage)
- ENS namehash computation (EIP-137 test vectors)
- Chain config presets and aliases
- Watch list operations (add, remove, contains, persistence path)
- CSV/JSON export formatting

## License

MIT
