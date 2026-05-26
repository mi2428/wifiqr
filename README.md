# wifiqr

A Rust CLI that generates Wi-Fi QR codes as PNG, SVG, or terminal text art
from Wi-Fi credentials or arbitrary QR payload text.

## Installation

Install Rust and Cargo first, then build and install the binary with `make install`.
By default, the binary is installed to `~/.local/bin/wifiqr`.
Set `INSTALL_BINDIR` if you want to install it somewhere else.

```console
$ git clone https://github.com/mi2428/wifiqr
$ make -C wifiqr install
```

## Usage

```console
$ wifiqr --help

Generate Wi-Fi QR codes as PNG, SVG, or terminal text art.

Usage: wifiqr [OPTIONS] [COMMAND]

Commands:
  raw   Generate a QR code from an arbitrary raw string
  help  Print this message or the help of the given subcommand(s)

Options:
  -o, --output <OUTPUT>
          Output file path for PNG or SVG
  -f, --format <FORMAT>
          Output format. Inferred from --output extension when omitted; defaults to ascii without --output [possible values: png, svg, ascii]
      --size <SIZE>
          PNG/SVG canvas size in pixels [default: 1024]
      --border <BORDER>
          Quiet-zone border in QR modules [default: 4]
  -e, --error-correction <ERROR_CORRECTION>
          QR error correction level [default: h] [possible values: l, m, q, h]
      --ascii-char <ASCII_CHAR>
          Text used for dark modules in terminal text output [default: ⬜]
  -s, --ssid <SSID>
          Wi-Fi SSID for the default Wi-Fi mode
  -p, --password <PASSWORD>
          Wi-Fi password for WPA/WEP networks
  -t, --security <SECURITY>
          Wi-Fi security type [default: wpa] [possible values: wpa, wep, nopass]
      --hidden
          Mark the Wi-Fi network as hidden in the QR payload
  -h, --help
          Print help
  -V, --version
          Print version
```

Use `-V` for the package version and `--version` for detailed build metadata.

The default mode generates a Wi-Fi QR payload.
When `--format` is omitted, the output format is inferred from the `--output` extension.
Without `--output`, the QR code is printed as terminal text art.
The default dark module text is `⬜`; use `--ascii-char '#'` for a hash-style terminal QR.

```console
$ wifiqr --ssid 2024shownet --password from-messe --output qrcode.png
$ wifiqr --ssid 2024shownet --password from-messe --format svg --output qrcode.svg
$ wifiqr --ssid 2024shownet --password from-messe --format ascii
$ wifiqr --ssid 2024shownet --password from-messe --format ascii --ascii-char '#'
```

The `raw` subcommand encodes arbitrary text instead of building a Wi-Fi payload.

```console
$ wifiqr raw --help

Generate a QR code from an arbitrary raw string

Usage: wifiqr raw [OPTIONS] <TEXT>

Arguments:
  <TEXT>  Raw text to encode

Options:
  -o, --output <OUTPUT>
          Output file path for PNG or SVG
  -f, --format <FORMAT>
          Output format. Inferred from --output extension when omitted; defaults to ascii without --output [possible values: png, svg, ascii]
      --size <SIZE>
          PNG/SVG canvas size in pixels [default: 1024]
      --border <BORDER>
          Quiet-zone border in QR modules [default: 4]
  -e, --error-correction <ERROR_CORRECTION>
          QR error correction level [default: h] [possible values: l, m, q, h]
      --ascii-char <ASCII_CHAR>
          Text used for dark modules in terminal text output [default: ⬜]
  -h, --help
          Print help
```

```console
$ wifiqr raw 'WIFI:S:2024shownet;T:WPA;P:from-messe;;' --output qrcode.png
```

## Development

`make release TAG=vX.Y.Z` builds four local release binaries, pushes the Git tag,
creates or updates the GitHub Release with generated release notes, and uploads
the release artifacts.
The default release matrix is macOS/Linux for amd64/arm64.
Before releasing, this repository must have a clean working tree.
Set `GH_REPO=owner/repo` if the GitHub repository cannot be inferred from `GIT_REMOTE`.

```console
$ make

Development
  build             Build the host binary into bin/
  install           Build and install the host binary into INSTALL_BINDIR
  fmt               Format Rust sources. Use CHECK_ONLY=1 to check without writing
  lint              Run clippy with warnings treated as errors
  doc               Build rustdoc with warnings treated as errors
  test              Run unit tests
  check             Run formatting, lint, rustdoc, and tests
  clean             Remove local build artifacts

Distribution
  release           Build 4 local dist binaries, push the tag, and publish a GitHub release. Requires TAG=vX.Y.Z
  dist              Build release binaries into dist/. Use OS=darwin,linux and ARCH=amd64,arm64
  dist-smoke        Smoke-test Linux dist binaries in a Debian container
  checksums         Write SHA-256 checksums for dist artifacts

Help
  help              Show this help message

Variables:
  TAG               Release tag for make release, for example v0.1.0
  GIT_REMOTE        Release git remote, defaults to origin
  OS                Release OS list for make dist, defaults to darwin,linux
  ARCH              Release arch list for make dist, defaults to amd64,arm64
  INSTALL_BINDIR    Install directory, defaults to /Users/teo/.local/bin

Examples:
  make fmt CHECK_ONLY=1                         # Check formatting without writing
  make check                                    # Run local quality gates
  make dist OS=darwin,linux ARCH=amd64,arm64    # Build release binaries and checksums
  make release TAG=v0.1.0                       # Publish a GitHub release with local artifacts
```

## License

MIT
