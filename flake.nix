{
  description = "Rust RAG Example with SurrealDB";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Crane for building Rust packages
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common source filtering
        src = craneLib.cleanCargoSource ./.;

        # Common build arguments
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            clang
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };

        # Build dependencies (cached)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the server binary
        rag-server = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p rag-server";

          meta = {
            description = "RAG Server API";
            mainProgram = "rag-server";
          };
        });

        # Build the frontend
        rag-ui = pkgs.stdenv.mkDerivation {
          pname = "rag-ui";
          version = "0.1.0";
          src = ./rag-ui;

          nativeBuildInputs = with pkgs; [
            rustToolchain
            trunk
            wasm-bindgen-cli
            nodePackages.tailwindcss
            nodejs_20
            binaryen
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          buildPhase = ''
            export HOME=$(mktemp -d)
            export XDG_CACHE_HOME=$(mktemp -d)
            npm install
            trunk build --release
          '';

          installPhase = ''
            mkdir -p $out
            cp -r dist/* $out/
          '';
        };

        # Dev scripts
        devBackend = pkgs.writeShellScriptBin "dev-backend" ''
          cargo watch -x "run -p rag-server"
        '';

        devFrontend = pkgs.writeShellScriptBin "dev-frontend" ''
          cd rag-ui
          npm install --silent
          trunk serve --open
        '';

        dev = pkgs.writeShellScriptBin "dev" ''
          trap 'kill $(jobs -p) 2>/dev/null' EXIT

          echo "Starting RAG development environment..."
          echo ""

          # Start backend with cargo-watch
          echo "Starting backend on http://localhost:8080"
          cargo watch -x "run -p rag-server" &
          BACKEND_PID=$!

          # Wait for backend to compile and start
          echo "Waiting for backend to start..."
          sleep 5

          # Start frontend
          echo "Starting frontend on http://localhost:8081"
          cd rag-ui && npm install --silent && trunk serve --open &
          FRONTEND_PID=$!

          echo ""
          echo "Development servers running:"
          echo "  Backend:  http://localhost:8080"
          echo "  Frontend: http://localhost:8081"
          echo ""
          echo "Press Ctrl+C to stop all services"

          wait
        '';

        buildRelease = pkgs.writeShellScriptBin "build-release" ''
          echo "Building release..."
          cargo build --release -p rag-server
          cd rag-ui && trunk build --release
          echo "Done! Binary at target/release/rag-server"
        '';

      in
      {
        packages = {
          default = rag-server;
          server = rag-server;
          ui = rag-ui;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Build dependencies for RocksDB
            clang
            llvmPackages.libclang
            pkg-config
            openssl

            # Frontend tools
            trunk
            nodejs_20
            nodePackages.tailwindcss

            # Dev tools
            cargo-watch

            # Dev scripts
            dev
            devBackend
            devFrontend
            buildRelease
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          shellHook = ''
            echo ""
            echo "🦀 Rust RAG Development Environment"
            echo "===================================="
            echo ""
            echo "Commands:"
            echo "  dev           - Start backend + frontend (with hot reload)"
            echo "  dev-backend   - Start only backend (cargo watch)"
            echo "  dev-frontend  - Start only frontend (trunk serve)"
            echo "  build-release - Build production binaries"
            echo ""
            echo "NixOS Deployment:"
            echo "  nix build .#server  - Build backend"
            echo "  nix build .#ui      - Build frontend"
            echo ""
            echo "Database: SurrealDB embedded (RocksDB)"
            echo "Data stored in: ./data/rag.db"
            echo ""
          '';
        };
      }) // {

    # NixOS module for deployment
    nixosModules.default = { config, lib, pkgs, ... }:
      with lib;
      let
        cfg = config.services.rag-server;
      in {
        options.services.rag-server = {
          enable = mkEnableOption "RAG Server - Retrieval Augmented Generation";

          package = mkOption {
            type = types.package;
            default = self.packages.${pkgs.system}.server;
            description = "The rag-server package to use";
          };

          uiPackage = mkOption {
            type = types.package;
            default = self.packages.${pkgs.system}.ui;
            description = "The rag-ui package to use";
          };

          host = mkOption {
            type = types.str;
            default = "127.0.0.1";
            description = "Host to bind the server to";
          };

          port = mkOption {
            type = types.port;
            default = 8080;
            description = "Port for the API server";
          };

          dataDir = mkOption {
            type = types.path;
            default = "/var/lib/rag-server";
            description = "Directory for storing database";
          };

          openaiApiBase = mkOption {
            type = types.str;
            default = "https://api.openai.com/v1";
            description = "OpenAI API base URL";
          };

          openaiModel = mkOption {
            type = types.str;
            default = "gpt-4o-mini";
            description = "OpenAI chat model";
          };

          embeddingModel = mkOption {
            type = types.str;
            default = "text-embedding-3-small";
            description = "OpenAI embedding model";
          };

          environmentFile = mkOption {
            type = types.nullOr types.path;
            default = null;
            description = "Environment file containing OPENAI_API_KEY";
          };

          domain = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Domain for nginx (enables nginx + SSL)";
          };

          acmeEmail = mkOption {
            type = types.str;
            default = "";
            description = "Email for Let's Encrypt certificates";
          };
        };

        config = mkIf cfg.enable {
          users.users.rag-server = {
            isSystemUser = true;
            group = "rag-server";
            home = cfg.dataDir;
            createHome = true;
          };
          users.groups.rag-server = {};

          systemd.services.rag-server = {
            description = "RAG Server";
            wantedBy = [ "multi-user.target" ];
            after = [ "network.target" ];

            environment = {
              RUST_LOG = "info";
              RAG__SERVER__HOST = cfg.host;
              RAG__SERVER__PORT = toString cfg.port;
              RAG__DATABASE__PATH = "${cfg.dataDir}/rag.db";
              RAG__OPENAI__API_BASE = cfg.openaiApiBase;
              RAG__OPENAI__MODEL = cfg.openaiModel;
              RAG__OPENAI__EMBEDDING_MODEL = cfg.embeddingModel;
            };

            serviceConfig = {
              Type = "simple";
              User = "rag-server";
              Group = "rag-server";
              WorkingDirectory = cfg.dataDir;
              ExecStart = "${cfg.package}/bin/rag-server";
              Restart = "on-failure";
              RestartSec = 5;

              # Security
              NoNewPrivileges = true;
              ProtectSystem = "strict";
              ProtectHome = true;
              PrivateTmp = true;
              ReadWritePaths = [ cfg.dataDir ];
            } // optionalAttrs (cfg.environmentFile != null) {
              EnvironmentFile = cfg.environmentFile;
            };
          };

          services.nginx = mkIf (cfg.domain != null) {
            enable = true;
            recommendedGzipSettings = true;
            recommendedOptimisation = true;
            recommendedProxySettings = true;
            recommendedTlsSettings = true;

            virtualHosts.${cfg.domain} = {
              enableACME = true;
              forceSSL = true;

              locations = {
                "/api/" = {
                  proxyPass = "http://${cfg.host}:${toString cfg.port}";
                  proxyWebsockets = true;
                  extraConfig = ''
                    proxy_buffering off;
                    proxy_read_timeout 300s;
                  '';
                };

                "/" = {
                  root = cfg.uiPackage;
                  tryFiles = "$uri $uri/ /index.html";
                };
              };
            };
          };

          security.acme = mkIf (cfg.domain != null) {
            acceptTerms = true;
            defaults.email = cfg.acmeEmail;
          };

          networking.firewall.allowedTCPPorts = mkIf (cfg.domain != null) [ 80 443 ];
        };
      };
  };
}
