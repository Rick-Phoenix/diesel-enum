set tempdir := "/tmp"

[working-directory('prelude')]
test:
    #!/usr/bin/env sh
    PG_BIN_DIR=$(find /usr/lib/postgresql/ -type d -name "bin" | head -n 1)

    if [ -z "$PG_BIN_DIR" ]; then
      echo "Error: Could not find PostgreSQL bin directory."
      exit 1
    fi

     PATH="$PG_BIN_DIR:$PATH"  cargo test --all-features  -- -q 
