# Examples

## Detecting patterns in output

Hooks watch job output and run an action when matching text appears. They can restart a job, restart it with a shorter backoff, or stop it.

### Restarting jobs

Restart a job when its output contains an error.

#### Create a job that reports an error

Create a job named `count` that prints `error` to `stderr` when it reaches 5:

##### sh (Linux)

```sh
tend create --overwrite count sh -- -c 'for i in $(seq 1 10); do if [ $i -eq 5 ]; then echo "error" >&2; else echo "hello $i"; fi; sleep 1; done'
```

##### cmd (Windows)

```sh
tend create --overwrite count cmd -- /C "@echo off & for /L %i in (1,1,10) do ((if %i==5 (echo error >&2) else (echo hello %i)) & timeout /t 1 /nobreak >nul)"
```

#### Detect errors

Create a hook named `error-hook` that watches the `stderr` output of `count`. When it sees `error`, it restarts the job:

```sh
tend edit count hook create error-hook detect-substring --stream stderr "error" restart
```

### Stopping jobs

Stop a job after the first successful ping response.

#### Create a ping job

```sh
tend create --overwrite ping-1111 ping 1.1.1.1
```

#### Detect first response and stop the job

Create a hook named `stop-hook` that watches the `stdout` output of `ping-1111`. When it sees `from 1.1.1.1`, it stops the job:

```sh
tend edit ping-1111 hook create stop-hook detect-substring --stream stdout "from 1.1.1.1" stop
```
