# Examples

## Detecting errors and restarting jobs

Detect errors from job command's output. When an error is detected, restart it.

### Create a faulty command

Create a command `count` that prints "error" to `stderr` when the number is 5:

#### sh (Linux)

```sh
tend create --overwrite "count" -- sh -c 'for i in $(seq 1 10); do if [ $i -eq 5 ]; then echo "error" >&2; else echo "hello $i"; fi; sleep 1; done'
```

#### cmd (Windows)

```sh
tend create --overwrite "count" -- cmd /C "for /L %i in (1,1,10) do ((if %i==5 (echo error >&2) else (echo hello %i)) & timeout /t 1 /nobreak >nul)"
```

### Detect errors

Create a hook with name `error-hook` that detects the substring `error` in the `stderr` output of the command `count-err` and restarts the command when the substring is detected:

```sh
tend edit "count" hook create "error-hook" detected-substring "error" restart stderr
```

## Stopping jobs

Stop a job when a certain condition is met.

### Create a command

```sh
tend create --overwrite ping-1111 1.1.1.1
```

### Detect a condition

Create a hook with name `stop-hook` that detects the substring `from 1.1.1.1` in the `stdout` output of the command `ping` and stops the command when the substring is detected:

```sh
tend edit "ping-1111" hook create "stop-hook" detected-substring "from 1.1.1.1" stop stdout
```
