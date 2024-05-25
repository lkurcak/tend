# Examples

## Detecting errors from stdout
Creates command called `count-err` that greets numbers from 1 to 10 with a 1 second delay between each number.

Prints "error" to `stderr` when the number is 5.

### sh (Linux)

```sh
tend create --overwrite "count-err" -- sh -c 'for i in $(seq 1 10); do if [ $i -eq 5 ]; then echo "error" >&2; else echo "hello $i"; fi; sleep 1; done'
```

### cmd (Windows)

```sh
tend create --overwrite "count-err" -- cmd /C "for /L %i in (1,1,10) do ((if %i==5 (echo error >&2) else (echo hello %i)) & timeout /t 1 /nobreak >nul)"
```
