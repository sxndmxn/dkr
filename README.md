# dkr

`dkr` monitors Docker containers from the terminal.
It provides an interactive view and status output in text or JSON format.

## Install

Install Rust and Cargo.
Run this command from the repository root:

```bash
cargo install --path . --locked
```

Docker must be running, and your account must have access to its socket.
The default connection is `unix:///var/run/docker.sock`.

## Use

```bash
dkr
dkr status
dkr status --json
dkr status container-name
dkr status --watch
dkr --help
```

In the interactive view, use the arrow keys to select a container.
Press `/` to filter containers.
Press `?` to show help.
Press `q` to quit.

## Configuration

```bash
dkr config path
dkr config show
```

The default file is `$XDG_CONFIG_HOME/dkr/config.toml`, or `~/.config/dkr/config.toml` when that variable is absent.
Use `--config PATH` to select another file.

The connection order is `--host`, `[connection].host`, `DOCKER_HOST`, then the default socket.

```toml
[display]
refresh_interval = 2000
show_stopped = true
```

The refresh interval uses milliseconds.
