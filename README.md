# Magister

[![CodeQL](https://github.com/unattended-backpack/magister/actions/workflows/codeql.yml/badge.svg)](https://github.com/unattended-backpack/magister/actions/workflows/codeql.yml) [![Create Release](https://github.com/unattended-backpack/magister/actions/workflows/release.yml/badge.svg)](https://github.com/unattended-backpack/magister/actions/workflows/release.yml)

> Let all things be done decently and in order.

Magister is a tool for managing a pool of [Vast](https://vast.ai/) instances. Magister was designed to be used alongside [`Hierophant`](https://github.com/unattended-backpack/hierophant/) to manage Contemplants, and as such supports specific integrations with Hierophant.

Magister attempts to keep a constant number of instances using a specific template running. Magister creates all instances on startup and periodically checks the instance count. If the count is below the desired target, more instances are requested. Magister tags all of its managed instances with the string `magister`. Instances can be deleted directly from the Vast frontend interface; Magister will detect this and allocate new instances.

*Note*: to support easier debug inspection, running instances are not destroyed when Magister is shut down. Instances must be manually destroyed through the Vast frontend interface.

## Integration with Hierophant

Magister integrates with Hierophant indirectly, using Contemplants as intermediaries.

1. Magister creates Contemplant instances and bootstraps them via environment variables constructed from its own configuration.
   - `MAGISTER_DROP_ENDPOINT` is constructed from Magister's `THIS_MAGISTER_ADDR` and `HTTP_PORT` configuration values, combined with the Vast offer ID (`http://magister:8555/drop/12345`).
   - `HIEROPHANT_WS_ADDRESS` is constructed from Magister's `HIEROPHANT_IP` and `HIEROPHANT_HTTP_PORT` configuration values (`ws://hierophant:9010/ws`).
   These environment variables are injected into the Vast instance's `onstart` script, making them available to the Contemplant when it starts.
2. When a Contemplant starts, it:
   - Connects to its Hierophant via WebSocket.
   - Sends a registration message to the Hierophant that includes its `MAGISTER_DROP_ENDPOINT`.
   - This makes the Hierophant aware of which Magister manages this instance.
3. After successful Hierophant registration, the Contemplant calls Magister's `GET /verify/:id` endpoint to signal it is operational.
4. Hierophant monitors Contemplant health using its own heartbeat, strike, and timeout criteria. When the Hierophant decides to drop a Contemplant, it sends an HTTP `DELETE` request to the `MAGISTER_DROP_ENDPOINT` it received during registration.
5. Magister receives the drop request and destroys the corresponding Vast instance.


```
Magister ──(creates)──> Contemplant
                            │
                (WebSocket + registration)
                            │
                            ▼
                        Hierophant
                            │
                (HTTP DELETE /drop/:id)
                            │
                            ▼
Magister <───────────────────
```

In summary, Magister never communicates directly with Hierophant. Hierophant learns about Magister through Contemplant registration messages.

# Quickstart

To get up and running quickly, we recommend visiting the [Scriptory](https://github.com/unattended-backpack/scriptory) to utilize our prepared setup for easily running a Hierophant, Magister, and a number of Contemplants.

# Running

Running Magister requires specifying a Vast API key to an account funded with some positive balance for instance rental. If you have not done so, create a [Vast](https://vast.ai/) account to retrieve this key.

## Standalone Magister

You can build a native version of Magister via `make build`. You can supply configuration to this Magister as either environment variables, or through a `magister.toml` created with `make init`. Please observe the available configuration in [`magister.example.toml`](./magister.example.toml). 

### Magister Endpoints

Magister exposes several HTTP endpoints for monitoring and management. They are all available on the HTTP port (default `8555`).

```bash
curl --request GET --url http://127.0.0.1:8555/summary
curl --request GET --url http://127.0.0.1:8555/instances
```

- `GET /summary`: returns a high-level overview of managed instances, including the total number of instances, total USD cost per hour, and basic information about each instance.
- `GET /instances`: returns verbose information on all Vast instances this Magister is managing, including full offer details and instance status.
- `GET /verify/:id`: called by Contemplants after successful Hierophant registration to signal they are operational. Not typically called manually.
- `DELETE /drop/:id`: called by the Hierophant to request that a specific instance be destroyed. Not typically called manually.

## Building Container Images

You can also build a container image of Magister using `make docker`, which uses a `BUILD_IMAGE` for building dependencies that are packaged to run in a `RUNTIME_IMAGE`. Configuration values in `.env.maintainer` may be overridden by specifying them as environment variables.
```bash
MAGISTER_NAME=magister make docker
BUILD_IMAGE=registry.digitalocean.com/sigil/petros:latest make docker
RUNTIME_IMAGE=debian:bookworm-slim@sha256:... make docker
```

You may also build a container image via `make ci` after building native binaries. Check the [Makefile](./Makefile) goals for more detailed information. Running the built container image is as simple as `make run`.

# Configuration

Magister supports configuration via both TOML files and environment variables, with environment variables taking precedence.

### Priority

Configuration is loaded with the following priority (highest to lowest):
1. Environment variables
2. TOML file (`magister.toml`)
3. Default values

The TOML file is optional if all required fields are provided via environment variables.

### Environment Variables

All configuration options can be set via environment variables:

**Basic Configuration:**
- `HTTP_PORT` - HTTP server port (default: 8555)
- `THIS_MAGISTER_ADDR` - Publicly accessible address where this Magister can be reached (required)
- `HIEROPHANT_IP` - Hierophant IP address (required)
- `HIEROPHANT_HTTP_PORT` - Hierophant HTTP port (required)

**Vast Configuration:**
- `VAST_API_KEY` - Vast API key (required)
- `VAST_API_CALL_BACKOFF_SECS` - Seconds between Vast API calls (default: 10)
- `TEMPLATE_HASH` - Vast template ID to use (required)
- `NUMBER_INSTANCES` - Number of instances to maintain (required)

**Query Configuration:**
- `VAST_QUERY_ALLOCATED_STORAGE` - Allocated storage in GB
- `VAST_QUERY_GPU_NAME` - GPU name (e.g., "RTX 4090")
- `VAST_QUERY_RELIABILITY` - Minimum reliability (0-1)
- `VAST_QUERY_MIN_CUDA_VERSION` - Minimum CUDA version
- `VAST_QUERY_GPU_RAM` - Minimum GPU RAM in GB
- `VAST_QUERY_DISK_SPACE` - Minimum disk space in GB
- `VAST_QUERY_DURATION` - Minimum duration
- `VAST_QUERY_COST_PER_HOUR` - Maximum cost per hour in USD

**Timing Configuration:**
- `TASK_POLLING_INTERVAL_SECS` - Task polling interval (default: 30)
- `CONTEMPLANT_VERIFICATION_TIMEOUT_SECS` - Contemplant verification timeout (default: 180)

**Machine Filtering (optional):**
- `BAD_HOSTS` - Comma-separated list of host IDs to avoid
- `BAD_MACHINES` - Comma-separated list of machine IDs to avoid
- `GOOD_HOSTS` - Comma-separated list of preferred host IDs
- `GOOD_MACHINES` - Comma-separated list of preferred machine IDs

### Example: Environment Variable Only Configuration

```bash
export THIS_MAGISTER_ADDR="http://my-magister.example.com:8555"
export HIEROPHANT_IP="hierophant.example.com"
export HIEROPHANT_HTTP_PORT="9010"
export VAST_API_KEY="your-api-key-here"
export TEMPLATE_HASH="your-template-hash"
export NUMBER_INSTANCES="5"
export VAST_QUERY_GPU_NAME="RTX 4090"
export VAST_QUERY_GPU_RAM="24"
export VAST_QUERY_COST_PER_HOUR="0.53"
# ... other configuration ...

RUST_LOG=info cargo run --release
```

## Release Configuration

Our configuration follows a zero-trust model where all sensitive configuration is stored on the self-hosted runner, not in GitHub. This section documents the configuration required for automated releases via GitHub Actions.

Running this project may require some sensitive configuration to be provided in `.env` and other files; you can generate the configuration files from the provided examples with `make init`. Review configuration files carefully and populate all required fields before proceeding.

### Runner-Local Secrets

All automated build secrets must be stored on the self-hosted runner at `/opt/github-runner/secrets/`. These files are mounted read-only into the release workflow container; they are never stored in git.

#### Required Secrets

**GitHub Access Tokens** (for creating releases and pushing to GHCR):
- `ci_gh_pat` - A GitHub fine-grained personal access token with repository permissions.
- `ci_gh_classic_pat` - A GitHub classic personal access token for GHCR authentication.

**Registry Access Tokens** (for pushing container images):
- `do_token` - A DigitalOcean API token with container registry write access.
- `dh_token` - A Docker Hub access token.

**GPG Signing Keys** (for signing release artifacts):
- `gpg_private_key` - A base64-encoded GPG private key for signing digests.
- `gpg_passphrase` - The passphrase for the GPG private key.
- `gpg_public_key` - The base64-encoded GPG public key (included in release notes).

**Registry Configuration** (`registry.env` file):

This file contains non-sensitive registry identifiers and build configuration:

```bash
# The Docker image to perform release builds with.
# If not set, defaults to unattended/petros:latest from Docker Hub.
# Examples:
#   BUILD_IMAGE=registry.digitalocean.com/sigil/petros:latest
#   BUILD_IMAGE=ghcr.io/your-org/petros:latest
#   BUILD_IMAGE=unattended/petros:latest
BUILD_IMAGE=unattended/petros:latest

# The runtime base image for the final container.
# If not set, uses the value from .env.maintainer.
# Example:
#   RUNTIME_IMAGE=debian:trixie-slim@sha256:66b37a5078a77098bfc80175fb5eb881a3196809242fd295b25502854e12cbec
RUNTIME_IMAGE=debian:trixie-slim@sha256:66b37a5078a77098bfc80175fb5eb881a3196809242fd295b25502854e12cbec

# The name of the DigitalOcean registry to publish the built image to.
DO_REGISTRY_NAME=

# The username of the Docker Hub account to publish the built image to.
DH_USERNAME=unattended
```

## Verifying Release Artifacts

All releases include GPG-signed artifacts for verification. Each release contains:

- `image-digests.txt` - A human-readable list of container image digests.
- `image-digests.txt.asc` - A GPG signature for the digest list.
- `ghcr-manifest.json` / `ghcr-manifest.json.asc` - A GitHub Container Registry OCI manifest and signature.
- `dh-manifest.json` / `dh-manifest.json.asc` - A Docker Hub OCI manifest and signature.
- `do-manifest.json` / `do-manifest.json.asc` - A DigitalOcean Container Registry OCI manifest and signature.

### Quick Verification

Download the artifacts and verify signatures:

```bash
# Import the GPG public key (base64-encoded in release notes).
echo "<GPG_PUBLIC_KEY>" | base64 -d | gpg --import

# Verify digest list.
gpg --verify image-digests.txt.asc image-digests.txt

# Verify image manifests.
gpg --verify ghcr-manifest.json.asc ghcr-manifest.json
gpg --verify dh-manifest.json.asc dh-manifest.json
gpg --verify do-manifest.json.asc do-manifest.json
```

### Manifest Verification

The manifest files contain the complete OCI image structure (layers, config, metadata). You can use these to verify that a registry hasn't tampered with an image.
```bash
# Pull the manifest from the registry.
docker manifest inspect ghcr.io/unattended-backpack/...@sha256:... \
  --verbose > registry-manifest.json

# Compare to the signed manifest.
diff ghcr-manifest.json registry-manifest.json
```

This provides cryptographic proof that the image structure (all layers and configuration) matches what was signed at release time.

### Cosign Verification

Images are also signed with [cosign](https://github.com/sigstore/cosign) using GitHub Actions OIDC for keyless signing. This provides automated verification and build provenance.

To verify with cosign:
```bash
# Verify image signature (proves it was built by our workflow).
cosign verify ghcr.io/unattended-backpack/...@sha256:... \
  --certificate-identity-regexp='^https://github.com/unattended-backpack/.+' \
  --certificate-oidc-issuer=https://token.actions.githubusercontent.com
```

Cosign verification provides:
- Automated verification (no manual GPG key management).
- Build provenance (proves image was built by the GitHub Actions workflow).
- Registry-native signatures (stored alongside images).

**Note**: Cosign depends on external infrastructure (GitHub OIDC, Rekor). For maximum trust independence, rely on the GPG-signed manifests as your ultimate root of trust.

## Local Testing

This repository is configured to support testing the release workflow locally using the `act` tool. There is a corresponding goal in the Makefile, and instructions for further management of secrets [here](./docs/WORKFLOW_TESTING.md). This local testing file also shows how to configure the required secrets for building.

# Security

If you discover any bug; flaw; issue; dæmonic incursion; or other malicious, negligent, or incompetent action that impacts the security of any of these projects please responsibly disclose them to us; instructions are available [here](./SECURITY.md).

# License

The [license](./LICENSE) for all of our original work is `LicenseRef-VPL WITH AGPL-3.0-only`. This includes every asset in this repository: code, documentation, images, branding, and more. You are licensed to use all of it so long as you maintain _maximum possible virality_ and our copyleft licenses.

Permissive open source licenses are tools for the corporate subversion of libre software; visible source licenses are an even more malignant scourge. All original works in this project are to be licensed under the most aggressive, virulently-contagious copyleft terms possible. To that end everything is licensed under the [Viral Public License](./licenses/LicenseRef-VPL) coupled with the [GNU Affero General Public License v3.0](./licenses/AGPL-3.0-only) for use in the event that some unaligned party attempts to weasel their way out of copyleft protections. In short: if you use or modify anything in this project for any reason, your project must be licensed under these same terms.

For art assets specifically, in case you want to further split hairs or attempt to weasel out of this virality, we explicitly license those under the viral and copyleft [Free Art License 1.3](./licenses/FreeArtLicense-1.3).
