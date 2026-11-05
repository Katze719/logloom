# Install it

With Rust 1.99 or newer:

```sh
cargo install logloom --locked
```

## CI builds

Open a successful [Rust workflow run](https://github.com/Katze719/logloom/actions/workflows/ci.yml)
and download the artifact for your machine. GitHub requires a signed-in account
for artifact downloads. Pushes, pull requests, and manual runs build:

| Artifact | Files |
| --- | --- |
| `logloom-x86_64-pc-windows-msvc` | `logloom.exe`, x64 `.msi` installer |
| `logloom-x86_64-unknown-linux-musl` | ELF binary, `.tar.gz`, amd64 `.deb`, x86_64 `.rpm` |
| `logloom-aarch64-unknown-linux-musl` | ELF binary, `.tar.gz`, arm64 `.deb`, aarch64 `.rpm` |

Each download includes the MIT license and `SHA256SUMS`. Package versions come
from `Cargo.toml`. These are builds of that commit; the version alone doesn't
tell you whether it's a published release.

## Windows

Run the MSI installer. It installs under `Program Files\logloom` and adds logloom
to the system PATH. Administrator approval is required. Open a fresh terminal:

```powershell
logloom --version
```

Newer versions upgrade the existing installation. To replace a CI build with
another build of the same version, uninstall the previous build first.

Or put `logloom.exe` in a directory on your PATH. The C runtime is linked in;
there's no separate Visual C++ redistributable to install. The executable and MSI
aren't code-signed.

## Linux

The Linux binaries are statically linked with musl for their CPU architecture.
No glibc or musl installation needed. A compatible 64-bit Linux kernel is still
part of the deal.

After extracting the CI artifact, choose one:

```sh
# Standalone binary. Artifact ZIPs don't preserve executable permissions.
chmod +x logloom
mkdir -p ~/.local/bin
install -m 755 logloom ~/.local/bin/logloom

# Debian / Ubuntu (use the filename you downloaded)
sudo apt install ./logloom_0.1.1-1_amd64.deb

# Fedora / RHEL (same idea)
sudo dnf install ./logloom-0.1.1-1.x86_64.rpm
```

DEB and RPM packages install `/usr/bin/logloom`; remove them with
`sudo apt remove logloom` or `sudo dnf remove logloom`. The `.tar.gz` keeps the
executable bit and includes the binary, license, and README.

To check a Linux download before installing:

```sh
sha256sum --check SHA256SUMS
```

## From a checkout

See [development](development.md#build-and-check) for local builds and checks.
