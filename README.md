# magister

Vast.ai instance manager.

To be used along with [`Hierophant`](https://github.com/unattended-backpack/hierophant/).

## Running

### Requirements

- Vast.ai API with an account balance
- Running Hierophant instance

### Run in docker-compose with Hierophant (recommended)

This is a helper binary to Hierophant, so details for running both in a docker-compose setup can be found in [Hierophant's README](https://github.com/unattended-backpack/hierophant/).

### Run manually (not recommended)

If you're doing development work or have an advanced use case then running Magister manually is fine.  Otherwise, it's recommended to run in a docker-compose setup with Hierophant.  See [Run in docker-compose with Hierophant (recommended)](#run-in-docker-compose-with-hierophant-(recommended)).

Make a copy of `magister.example.toml` named `magister.toml` and fill in required variables, then run with `RUST_LOG=info cargo run --release`:

```bash
cp magister.example.toml magister.toml
# Fill in require variables

RUST_LOG=info cargo run --release
```
