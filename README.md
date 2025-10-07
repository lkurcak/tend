# tend

[![Build status](https://github.com/lkurcak/tend/workflows/release/badge.svg)](https://github.com/lkurcak/tend/actions)
[![Crates.io](https://img.shields.io/crates/v/tend.svg)](https://crates.io/crates/tend)
[![Snapcraft](https://snapcraft.io/tend/badge.svg)](https://snapcraft.io/tend)
[![WinGet Package Version](https://img.shields.io/winget/v/lkurcak.tend)](https://github.com/microsoft/winget-pkgs/tree/master/manifests/l/lkurcak/tend)


### Installation

**[Download binaries](https://github.com/lkurcak/tend/releases)** if you are using **Windows**, **macOS** or **Linux**.

You can install `tend` using `snap`:

```sh
sudo snap install tend
```

Or with `winget`:

```sh
winget install lkurcak.tend
```

Or using `cargo`:

```sh
cargo install tend
```

Or build from source:

```sh
git clone https://github.com/lkurcak/tend
cd tend
cargo build --release
```

### Usage

#### Basic
Create a new job called `hello`:
```sh
tend create "hello" ping 8.8.8.8
tend run hello
```

Press `Ctrl-C` to stop all jobs and exit the program.

To view jobs enter:
```sh
tend list
```

#### Available Programs
Based on your platform and configuration you will have access to different programs and shells. Make sure the programs are accessible from your current working directory.

For example, you could write this on Linux:
```sh
tend create "time" sh -- -c 'echo Time: $(date)'
```
to achieve something similar as this on Windows:
```sh
tend create "time" cmd -- /C 'echo Time: %TIME%'
```

#### Groups

You can create a job as a part of a group:
```sh
tend create "postgres" --group="dev" kubectl port-forward svc/postgres 5432:5432
```

Start all jobs from a specific group:
```sh
tend run --group "dev"
```

