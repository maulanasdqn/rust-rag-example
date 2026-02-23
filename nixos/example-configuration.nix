# Example NixOS configuration for deploying RAG Server
# Add this to your NixOS configuration or import it

{ config, pkgs, ... }:

{
  imports = [
    # Import the RAG server module from the flake
    # In your flake.nix inputs, add:
    #   rag-app.url = "github:YOUR_USERNAME/rust-rag-example";
    # Then in your outputs:
    #   imports = [ rag-app.nixosModules.default ];
  ];

  # RAG Server configuration
  services.rag-server = {
    enable = true;

    # Domain for nginx (comment out to disable nginx)
    domain = "rag.example.com";

    # Email for Let's Encrypt SSL certificates
    acmeEmail = "admin@example.com";

    # API server settings
    host = "127.0.0.1";
    port = 8080;

    # Data directory (will be created automatically)
    dataDir = "/var/lib/rag-server";

    # OpenAI settings
    openaiApiBase = "https://api.openai.com/v1";
    openaiModel = "gpt-4o-mini";
    embeddingModel = "text-embedding-3-small";

    # IMPORTANT: Create this file with your API key
    # echo "OPENAI_API_KEY=sk-..." > /etc/rag-server/env
    # chmod 600 /etc/rag-server/env
    environmentFile = "/etc/rag-server/env";
  };
}
