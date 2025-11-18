# Repository Guidelines

This monorepo hosts Solidity contracts, Rust services, and Python tooling for the Vagus platform; use the sections below to keep contributions aligned and predictable.

## Project Structure & Module Organization
- `contracts/` contains Foundry sources, deployment scripts, and tests; use `infra/devnet/anvil.sh` to spin up the local EVM before interacting with them.
- `gateway/`, `oracle/`, and `relayer/` each house Rust workspaces for telemetry, tone scoring, and cross-chain relaying; shared schemas live in `schemas/` and invariants in `spec/`.
- `planner/` provides the Python intent generator plus CBOR vector tooling; cross-chain harnesses sit under `tests/golden/`, while `demo/` and `monitoring/` store walkthroughs and dashboards.

## Build, Test, and Development Commands
- Solidity: `cd contracts && forge build` to compile, `forge test -vv` for verbose fuzz-plus-unit coverage.
- Rust services: run `cargo build --workspace` or `cargo test --workspace` inside `gateway/`, `oracle/`, or `relayer/`; start core sims via `cargo run -p vagus-gateway -- --sim` or `cargo run -p tone-oracle`.
- Planner: `cd planner && pip install -e .[dev]`, then `pytest` or `python -m planner.examples.send_move_to` for end-to-end validation.
- Full demo: `./demo/scripts/cross-chain-demo.sh` once both chains are up.

## Coding Style & Naming Conventions
Follow `.editorconfig` (UTF-8, LF, 4-space indents; JS/TS use 2). Run `forge fmt`, `cargo fmt`, `cargo clippy --all-targets --all-features`, `ruff check`, `black .`, and `mypy --strict` before review. Solidity types stay PascalCase, interfaces get an `I` prefix, Python/Rust modules remain snake_case.

## Testing Guidelines
`forge test` already fuzzes 256 cases—name specs like `testRevertsWhen...`. Rust suites rely on `cargo test --workspace`; narrow focus with `cargo test -p vagus-gateway telemetry::`. Planner features use `pytest` plus fixtures in `planner/tests/fixtures/`. Run `cd tests/golden && cargo run -- run-all --evm-rpc http://localhost:8545 --cosmos-rpc http://localhost:26657` for cross-chain regression.

## Commit & Pull Request Guidelines
Write Conventional Commits (`feat(gateway): ...`, `fix(relayer): ...`). PRs should summarize protocol or schema deltas, link affected files, and attach output snippets for `forge test`, `cargo test`, or `pytest`. Flag breaking changes, ABI updates, and provide screenshots for dashboard tweaks; coordinate contract ABI changes with gateway, oracle, and relayer owners before merging.

## Security & Environment Tips
Keep secrets out of the repo; use local `.env` files referenced by services. Scripts may assume the devnet RPCs (`http://localhost:8545`, `http://localhost:26657`) are reachable—verify before running demos or the golden tests.
