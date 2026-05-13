# tend

[![Build status](https://github.com/lkurcak/tend/workflows/release/badge.svg)](https://github.com/lkurcak/tend/actions)
[![Crates.io](https://img.shields.io/crates/v/tend.svg?color=blue)](https://crates.io/crates/tend)
[![WinGet Package Version](https://img.shields.io/winget/v/lkurcak.tend?color=blue)](https://github.com/microsoft/winget-pkgs/tree/master/manifests/l/lkurcak/tend)
[![Snapcraft](https://snapcraft.io/tend/badge.svg)](https://snapcraft.io/tend)

`tend` is a command-line process manager for commands you run often. Save a command as a job, run one job or a group of jobs, and let `tend` restart them when they exit or when output hooks match.

It is useful for local development services, port forwards, and other long-running commands you want to start and supervise together.

## Installation

**Homebrew (macOS / Linux)**:
```sh
brew install lkurcak/tap/tend
```

**Winget (Windows):**
```sh
winget install lkurcak.tend
```

**Snapcraft:**
```sh
sudo snap install tend
```

**Binary:**
[Download](https://github.com/lkurcak/tend/releases)

**Cargo:**
```sh
cargo install tend --locked
```

## Quick Start

```sh
# Create a job named "hello"
tend create hello ping 8.8.8.8

# Run the job
tend run hello

# List saved jobs
tend list
```

Press `Ctrl+C` to stop running jobs.

Jobs are stored in `~/.tend/jobs` as JSON files.

## Examples

**Run jobs by group:**
```sh
tend create --group dev postgres kubectl port-forward svc/postgres 5432:5432
tend run --group dev
```

**Run a shell command:**
```sh
# Linux
tend create time sh -- -c 'echo "Time: $(date)"'

# Windows
tend create time cmd -- /C "echo Time: %TIME%"
```

**Restart only after failures:**
```sh
tend create --restart on-failure api cargo run
```

**Use the port-forward template:**
```sh
tend create --template port-forward postgres kubectl port-forward svc/postgres 5432:5432
```

See [EXAMPLES.md](EXAMPLES.md) for hooks that restart or stop jobs based on command output.

## License

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org).
