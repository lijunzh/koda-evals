# notes-service

A small API that serves notes.

## Running

```bash
python3 -m unittest discover -s tests
```

## Configuration

Settings live in `config.toml`. Every key is optional.

| Key | Default | What it does |
|-----|---------|--------------|
| `port` | `8080` | The port the service listens on |
| `log_level` | `"info"` | How much the service logs |
