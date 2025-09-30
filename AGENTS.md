# Repository Guidelines

## Project Structure & Module Organization
- `contracts/` holds Foundry Solidity core (src/, script/, test/) plus deployment helpers.
- `gateway/` Rust workspace (crates/vagus-gateway, vagus-telemetry, vagus-chain) bridges device telemetry and execution.
- `oracle/` and `relayer/` house Rust services for tone scoring and cross-chain message flow.
- `planner/` packages Python tooling (`vagus_planner`, templates, planner/tests/) for intent generation and CBOR vectors.
- `schemas/`, `spec/`, and `docs/` provide YAML policies, invariants, and reference docs; `tests/golden/` drives the multichain invariant harness.
- `infra/` supplies devnet scripts, `demo/` contains walkthroughs, and `monitoring/` stores dashboards.

## Build, Test, and Development Commands
- `./infra/devnet/anvil.sh` boots the local EVM chain; run before Foundry deployments.
- `forge build` / `forge test -vv` from `contracts/` compile and exercise Solidity flows.
- `cargo build --workspace` in `gateway/`, `oracle/`, or `relayer/` compiles all crates; `cargo run -p tone-oracle` and `cargo run -p vagus-gateway -- --sim` start core services.
- `pip install -e .[dev]` inside `planner/` installs planner tooling; `python -m planner.examples.send_move_to` runs the sample pipeline.
- `./demo/scripts/cross-chain-demo.sh` executes the end-to-end capability lifecycle once both chains are running.

## Coding Style & Naming Conventions
- `.editorconfig` enforces UTF-8, LF, and 4-space indents (2 for JS/TS); configure editors to respect it.
- Run `cargo fmt` and `cargo clippy --all-targets --all-features` before pushing Rust changes; files stay snake_case, types remain CamelCase.
- Execute `forge fmt` to normalize Solidity; contracts use PascalCase types, I-prefixed interfaces, and underscore unused params.
- Python modules and tests are snake_case; keep `ruff check`, `black .`, and `mypy --strict` clean before review.

## Testing Guidelines
- Solidity: `forge test` uses 256 fuzz runs by default; name cases descriptively (`testRevertsWhen...`).
- Rust services: `cargo test --workspace` plus focused `cargo test -p vagus-gateway telemetry::` during iteration.
- Cross-chain regression: `cd tests/golden && cargo run -- run-all --evm-rpc http://localhost:8545 --cosmos-rpc http://localhost:26657`.
- Planner: `pytest` inside `planner/`; add fixtures under `planner/tests/fixtures/` and document new CBOR vectors via `planner/generate_cbor_vectors.py`.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`feat:`, `fix:`, `chore:`) as seen in history; add scopes when clarifying (`feat(gateway): ...`).
- Keep PRs focused, summarizing protocol or schema changes and linking relevant files under `spec/` or `schemas/`; mention dependent services.
- Attach output snippets for critical commands (forge test, cargo test, pytest) and screenshots for dashboard changes.
- Flag breaking changes or migrations in the PR description; coordinate contract ABI updates with gateway, oracle, and relayer owners before merge.
