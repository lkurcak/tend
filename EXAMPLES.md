# Examples

## Detecting patterns in output

Detect patterns in the output of a command and take action when a pattern is detected.

### Restarting jobs

Restart a job when a certain condition is met.

#### Create a faulty command

Create a command `count` that faults (prints "error" to `stderr`) occasionally (when it counts to 5):

##### sh (Linux)

```sh
tend create --overwrite "count" -- sh -c 'for i in $(seq 1 10); do if [ $i -eq 5 ]; then echo "error" >&2; else echo "hello $i"; fi; sleep 1; done'
```

##### cmd (Windows)

```sh
tend create --overwrite "count" -- cmd /C "@echo off & for /L %i in (1,1,10) do ((if %i==5 (echo error >&2) else (echo hello %i)) & timeout /t 1 /nobreak >nul)"
```

#### Detect errors

Create a hook with name `error-hook` that detects the substring `error` in the `stderr` output of the command `count-err` and restarts the command when the substring is detected:

```sh
tend edit "count" hook create "error-hook" detect-substring --stream stderr "error" restart
```

### Stopping jobs

Stop a job when a certain condition is met.

#### Create a command

```sh
tend create --overwrite ping-1111 1.1.1.1
```

#### Detect first response and stop the job

Create a hook with name `stop-hook` that detects the substring `from 1.1.1.1` in the `stdout` output of the command `ping` and stops the command when the substring is detected:

```sh
tend edit "ping-1111" hook create "stop-hook" detect-substring --stream stdout "from 1.1.1.1" stop
```
