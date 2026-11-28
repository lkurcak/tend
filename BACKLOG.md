# Backlog

## Now

## Later

- Run as a daemon, and as a service on Windows, where
  [windows-service](https://github.com/mullvad/windows-service-rs) might help (was #47).
- An interactive mode, to start and stop jobs while tend runs (was #48).
- Templates that update the jobs made from them. Jobs re-apply theirs on load, but creating one
  also saves its hooks into the job, so old ones stay and each save adds them again (was #69).
- Health checks, to restart a job that hangs while looking alive, as `kubectl port-forward`
  does until a request times out on a dead connection (was #77).
- A Nix flake (was #78).
- A TUI (was #79).

## Done
