# Install it

With Rust 1.99 or newer:

```sh
cargo install logloom --locked
```

## Binaries

Download binaries and installers from
[Releases](https://github.com/Katze719/logloom/releases). No Rust compiler needed.

| Platform | Downloads |
| --- | --- |
| Windows x64 | EXE, ZIP, MSI installer |
| Linux x64 / ARM64 | Standalone ELF, `.tar.gz`, `.deb`, `.rpm` |

For unreleased builds, download artifacts from a successful
[CI run](https://github.com/Katze719/logloom/actions/workflows/ci.yml).
GitHub sign-in is required for those downloads.

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

Linux binaries are statically linked with musl. No glibc or musl installation needed.

Choose one, using your downloaded version and architecture:

```sh
# Standalone tarball
tar -xzf logloom-1.1.1-x86_64-unknown-linux-musl.tar.gz
mkdir -p ~/.local/bin
install -m 755 logloom ~/.local/bin/logloom

# Debian / Ubuntu (use the filename you downloaded)
sudo apt install ./logloom_1.1.1-1_amd64.deb

# Fedora / RHEL (same idea)
sudo dnf install ./logloom-1.1.1-1.x86_64.rpm
```

DEB and RPM packages install `/usr/bin/logloom`; remove them with
`sudo apt remove logloom` or `sudo dnf remove logloom`. The `.tar.gz` keeps the
executable bit and includes the binary, license, and README.

To verify downloaded files, also download `SHA256SUMS`:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```
