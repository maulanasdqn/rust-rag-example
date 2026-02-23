# NixOS Deployment Guide

Deploy the RAG application on your NixOS VPS with systemd service, nginx reverse proxy, and automatic SSL.

## Quick Start

### 1. Add the Flake Input

In your NixOS `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Add RAG app
    rag-app.url = "github:YOUR_USERNAME/rust-rag-example";
    # Or use local path for testing:
    # rag-app.url = "path:/path/to/rust-rag-example";
  };

  outputs = { self, nixpkgs, rag-app, ... }: {
    nixosConfigurations.your-server = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        rag-app.nixosModules.default
      ];
    };
  };
}
```

### 2. Configure the Service

In your `configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  services.rag-server = {
    enable = true;

    # Your domain (enables nginx + SSL)
    domain = "rag.yourdomain.com";
    acmeEmail = "you@email.com";

    # OpenAI settings
    openaiApiBase = "https://api.openai.com/v1";
    openaiModel = "gpt-4o-mini";
    embeddingModel = "text-embedding-3-small";

    # Secret file for API key
    environmentFile = "/etc/rag-server/env";
  };
}
```

### 3. Create the Secrets File

```bash
# On your VPS:
sudo mkdir -p /etc/rag-server
echo "OPENAI_API_KEY=sk-your-key-here" | sudo tee /etc/rag-server/env
sudo chmod 600 /etc/rag-server/env
```

### 4. Deploy

```bash
# From your local machine:
nixos-rebuild switch --flake .#your-server --target-host root@your-vps-ip

# Or on the VPS directly:
sudo nixos-rebuild switch --flake /path/to/config#your-server
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `enable` | `false` | Enable the RAG server |
| `domain` | `null` | Domain for nginx (null = no nginx) |
| `acmeEmail` | `""` | Email for Let's Encrypt |
| `host` | `"127.0.0.1"` | Server bind address |
| `port` | `8080` | Server port |
| `dataDir` | `"/var/lib/rag-server"` | Database directory |
| `openaiApiBase` | `"https://api.openai.com/v1"` | OpenAI API URL |
| `openaiModel` | `"gpt-4o-mini"` | Chat model |
| `embeddingModel` | `"text-embedding-3-small"` | Embedding model |
| `environmentFile` | `null` | File with `OPENAI_API_KEY` |

## Alternative: OpenRouter

To use OpenRouter instead of OpenAI:

```nix
services.rag-server = {
  enable = true;
  domain = "rag.yourdomain.com";
  acmeEmail = "you@email.com";

  # OpenRouter settings
  openaiApiBase = "https://openrouter.ai/api/v1";
  openaiModel = "anthropic/claude-3.5-sonnet";
  embeddingModel = "openai/text-embedding-3-small";

  environmentFile = "/etc/rag-server/env";
};
```

Environment file:
```bash
OPENAI_API_KEY=sk-or-your-openrouter-key
```

## Without Nginx (API Only)

For API-only deployment without nginx:

```nix
services.rag-server = {
  enable = true;
  host = "0.0.0.0";  # Listen on all interfaces
  port = 8080;
  # Don't set 'domain' - nginx won't be configured
  environmentFile = "/etc/rag-server/env";
};

# Open firewall manually
networking.firewall.allowedTCPPorts = [ 8080 ];
```

## Manual Service Management

```bash
# Check status
sudo systemctl status rag-server

# View logs
sudo journalctl -u rag-server -f

# Restart
sudo systemctl restart rag-server

# Check data directory
ls -la /var/lib/rag-server/
```

## Updating

```bash
# Update flake inputs
nix flake update rag-app

# Rebuild
sudo nixos-rebuild switch --flake .#your-server
```

## Backup

The database is stored in `dataDir` (default `/var/lib/rag-server/`):

```bash
# Backup
sudo tar -czvf rag-backup.tar.gz /var/lib/rag-server/

# Restore
sudo systemctl stop rag-server
sudo tar -xzvf rag-backup.tar.gz -C /
sudo systemctl start rag-server
```

## Troubleshooting

### Service won't start
```bash
sudo journalctl -u rag-server -n 50
```

### Permission denied on data directory
```bash
sudo chown -R rag-server:rag-server /var/lib/rag-server
```

### SSL certificate issues
```bash
sudo systemctl status acme-rag.yourdomain.com
sudo journalctl -u acme-rag.yourdomain.com
```

### Check nginx config
```bash
sudo nginx -t
sudo systemctl status nginx
```
