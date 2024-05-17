# tend

[![Build status](https://github.com/lubomirkurcak/tend/workflows/release/badge.svg)](https://github.com/lubomirkurcak/tend/actions)
[![Crates.io](https://img.shields.io/crates/v/tend.svg)](https://crates.io/crates/tend)

### Installation

Download pre-built **[binaries](https://github.com/lubomirkurcak/tend/releases)** if you are using **Windows**, **macOS** or **Linux**.

#### Snap

```sh
sudo snap install tend
```

#### Cargo

```sh
cargo install tend
```

### Usage

#### Basic
Create a new job called `hello`:
```sh
tend create hello ping 8.8.8.8
```

Run all jobs:
```sh
tend run
```

Press `Ctrl-C` to cancel all jobs and exit the program.

#### Management

List jobs:
```sh
tend list
```

```
+-------+---------+---------+-------------------+------------+---------+
| Job   | Program | Args    | Working Directory | Restart    | Group   |
+-------+---------+---------+-------------------+------------+---------+
| hello | ping    | 8.8.8.8 | /home/user        | on failure | default |
+-------+---------+---------+-------------------+------------+---------+
```

Reconfigure `hello` to always restart on completion (even successful):
```sh
tend create hello ping 8.8.8.8 --restart=always --overwrite
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

Create a job as a part of a group:
```sh
tend create "postgres" --group="dev" kubectl port-forward svc/postgres 5432:5432
```

Start all jobs from a specific group:
```sh
tend run --group "dev"
```

