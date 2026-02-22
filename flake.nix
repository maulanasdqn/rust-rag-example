{
  description = "Rust RAG Example with PostgreSQL + pgvector";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # PostgreSQL with pgvector extension
        postgresWithPgvector = pkgs.postgresql_16.withPackages (ps: [ ps.pgvector ]);

        # Data directory for PostgreSQL
        pgDataDir = "./.pg_data";

        # Scripts for managing PostgreSQL
        startPostgres = pkgs.writeShellScriptBin "pg-start" ''
          set -e
          export PGDATA="$PWD/.pg_data"
          export PGPORT="5432"

          if [ ! -d "$PGDATA" ]; then
            echo "Initializing PostgreSQL database..."
            ${postgresWithPgvector}/bin/initdb -D "$PGDATA" --auth=trust --no-locale --encoding=UTF8

            # Configure PostgreSQL - use absolute path for socket
            echo "unix_socket_directories = '$PGDATA'" >> "$PGDATA/postgresql.conf"
            echo "listen_addresses = 'localhost'" >> "$PGDATA/postgresql.conf"
            echo "port = 5432" >> "$PGDATA/postgresql.conf"
          fi

          if ${postgresWithPgvector}/bin/pg_ctl -D "$PGDATA" status > /dev/null 2>&1; then
            echo "PostgreSQL is already running"
          else
            echo "Starting PostgreSQL..."
            ${postgresWithPgvector}/bin/pg_ctl -D "$PGDATA" -l "$PGDATA/postgres.log" start

            # Wait for PostgreSQL to be ready
            sleep 2

            # Create database and enable pgvector if not exists
            ${postgresWithPgvector}/bin/psql -h localhost -p 5432 -d postgres -c "SELECT 1 FROM pg_database WHERE datname = 'rag_db'" | grep -q 1 || \
              ${postgresWithPgvector}/bin/createdb -h localhost -p 5432 rag_db

            ${postgresWithPgvector}/bin/psql -h localhost -p 5432 -d rag_db -c "CREATE EXTENSION IF NOT EXISTS vector;"

            echo "PostgreSQL started with pgvector extension enabled"
          fi

          echo ""
          echo "Connection URL: postgresql://localhost:5432/rag_db"
        '';

        stopPostgres = pkgs.writeShellScriptBin "pg-stop" ''
          export PGDATA="$PWD/.pg_data"
          if [ -d "$PGDATA" ]; then
            ${postgresWithPgvector}/bin/pg_ctl -D "$PGDATA" stop 2>/dev/null || echo "PostgreSQL is not running"
          fi
        '';

        pgStatus = pkgs.writeShellScriptBin "pg-status" ''
          export PGDATA="$PWD/.pg_data"
          ${postgresWithPgvector}/bin/pg_ctl -D "$PGDATA" status
        '';

        pgPsql = pkgs.writeShellScriptBin "pg-psql" ''
          ${postgresWithPgvector}/bin/psql -h localhost -p 5432 -d rag_db "$@"
        '';

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
              targets = [ "wasm32-unknown-unknown" ];
            })

            # PostgreSQL with pgvector
            postgresWithPgvector

            # Database management scripts
            startPostgres
            stopPostgres
            pgStatus
            pgPsql

            # Build dependencies
            pkg-config
            openssl

            # Frontend tools
            trunk
            nodejs_20
            nodePackages.tailwindcss

            # Useful tools
            cargo-watch
            process-compose
          ];

          shellHook = ''
            echo ""
            echo "🦀 Rust RAG Development Environment"
            echo "===================================="
            echo ""
            echo "Quick start:"
            echo "  dev        - Start everything (DB + backend + frontend)"
            echo ""
            echo "Manual commands:"
            echo "  pg-start   - Start PostgreSQL with pgvector"
            echo "  pg-stop    - Stop PostgreSQL"
            echo "  pg-psql    - Connect to database"
            echo ""

            alias dev="process-compose up"
          '';

          # Environment variables
          DATABASE__URL = "postgresql://localhost:5432/rag_db";
        };
      });
}
