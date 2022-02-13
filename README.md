# skynki
SkyEng words integration with Anki

### Migration with diesel cli

    cargo install diesel_cli --no-default-features --features postgres

### Prepare for testing

    docker compose up -d && sleep 3 && export $(grep -v '^#' .env.test | xargs -0) && diesel migration run