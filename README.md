# toy-payments-engine

A small transactions processing engine which takes a csv file
as input and applies them to accounts, writing the resulting accounts
out to the console in csv format.

This code is not meant to be treated as production ready.

It has, however, has been approached from the point of what would likely 
be present in a minimal production application which handles large CSV files 
and tries not to re-invent the wheel.

## Features

- Streams CSV file
- Supports `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`
- Processes clients concurrently
- Embedded `sled` database acting as a ledger
- Logging for malformed entries and other errors
- Decimal amounts are parsed directly to `u128` and then back again for display, avoiding floating point arithmetic and rounding errors.

## Assumptions

- Each client has a single account
- Clients are `u16` integers instead of UUIDs
- Transaction IDs are `u32` integers instead of UUIDs
- Errors are non-fatal, should be logged and then execution should continue
- Decimals with precision greater than 4 decimal places are truncated to 4 decimal places
- Malformed rows are skipped
- Only deposits can be disputed currently
- Dispute, Resolve and Chargeback can only be applied to a transaction once (for simplicity)
- Resolve and Chargeback have to have a Dispute already open for the respective transaction
- Resolve and Chargeback are mutually exclusive
- Chargeback locks an account, only further deposits can succeed on locked accounts
- Ledger only exists for the lifetime of the application (for simplicity).
- Output ordering is sorted by client id

## Usage

Development Checks
```bash
 cargo +nightly fmt --all  
 tombi format # Requires separate install
 cargo clippy --all-targets --all-features -- -D warnings
 cargo +nightly rustdoc -- -Z unstable-options --check 
 cargo test
```

Running it
```bash
cargo run -- test_input/basic_test.csv
```

## Notes

- There are still some TODOs in the codebase that I didn't get to yet.
- There is no audit trail for accounts currently

