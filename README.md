# tend

[![Build status](https://github.com/lkurcak/tend/workflows/release/badge.svg)](https://github.com/lkurcak/tend/actions)
[![Crates.io](https://img.shields.io/crates/v/tend.svg?color=blue)](https://crates.io/crates/tend)
[![WinGet Package Version](https://img.shields.io/winget/v/lkurcak.tend?color=blue)](https://github.com/microsoft/winget-pkgs/tree/master/manifests/l/lkurcak/tend)
[![Snapcraft](https://snapcraft.io/tend/badge.svg)](https://snapcraft.io/tend)

Command-line tool for managing and running multiple processes.

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org).

## Installation

**Windows:**
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
# Create a job
tend create "hello" ping 8.8.8.8

# Run it
tend run hello

# View all jobs
tend list
```

Press `Ctrl-C` to stop all jobs.

## Examples

**Run jobs by group:**
```sh
tend create "postgres" --group="dev" kubectl port-forward svc/postgres 5432:5432
tend run --group "dev"
```

**Run any command available in your shell:**
```sh
# Linux
tend create "time" sh -- -c 'echo Time: $(date)'

# Windows
tend create "time" cmd -- /C "echo Time: %TIME%"
```
