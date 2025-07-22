# magister

[Vast.ai](https://vast.ai/) instance manager to be used along with [`Hierophant`](https://github.com/unattended-backpack/hierophant/).

Magister ensures a constant amount of Vast.ai instances of a specific template are running.  It creates those instances on startup and periodically checks the instance count.  If it is below the desired instance count then more instances are requested.  Magister hooks into Hierophant to allow Hierophant to command Magister to drop specific instances.  Instances created by Magister will appear with the tag `magister` on the Vast.ai instance tab.  Instances can be deleted from the Vast.ai frontend and Magister will detect this and allocate new instances.

*IMPORTANT NOTE*: To allow for inspection, instances aren't deallocated up when Magister is shut down.  You will have to go into the Vast.ai instance manager and manually destroy the instances after Magister is stopped.  This behaviour can be changed.

## Running

### Requirements

- Vast.ai API key with an account balance

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

Magister HTTP is on port `8555` by default.

## Useful endpoints

- `GET /summary` High level overview of instances as well as total USD cost per hour.

```bash
# paste this command on the same machine running Magister
curl --request GET --url http://127.0.0.1:8555/summary
```

- `GET /instances` Verbose information on all Vast.ai instances this Magister is managing.

```bash
# paste this command on the same machine running Magister
curl --request GET --url http://127.0.0.1:8555/instances
```

## Building docker image

Make sure to build the binary before building the docker image:

```bash
cargo build --release;
docker build -t magister .
```
