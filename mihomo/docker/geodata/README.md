# Local Docker geodata

Place controlled local geodata files here before building the Docker image:

- `geoip.metadb`
- `geosite.dat`
- `geoip.dat`

These files are intentionally not downloaded by `Dockerfile` and are ignored by
Git to avoid committing large generated assets by accident.
