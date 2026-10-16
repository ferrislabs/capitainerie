# The realm and OIDC clients of the local demo organization.
#
# One realm per organization: this one is the demo organization `capitainerie`.
# Token exchange policies are not covered by provider v0.1.0; the bootstrap
# script creates them through the admin API.

terraform {
  required_version = ">= 1.5"

  required_providers {
    ferriskey = {
      source  = "ferriskey/ferriskey"
      version = "~> 0.1"
    }
  }
}

# Password grant against the master realm's admin-cli client, with the admin
# account a local FerrisKey ships with. A shared instance uses a dedicated
# service account instead.
provider "ferriskey" {
  url       = var.ferriskey_url
  realm     = "master"
  client_id = "admin-cli"
  username  = var.admin_username
  password  = var.admin_password
}

resource "ferriskey_realm" "organization" {
  name = var.realm
}

resource "ferriskey_realm_settings" "organization" {
  realm = ferriskey_realm.organization.name

  access_token_lifetime  = var.access_token_lifetime
  # Bounds the user session (`sid`). Every token of the delegation chain is
  # refused once that session ends, whatever its own `exp`: 8 hours lets a
  # local run finish without logging in again (docs/ferriskey-needs.md, need 5).
  refresh_token_lifetime = var.session_lifetime

  # FerrisKey has no admin API to set another user's password, so a local
  # stack is only reachable through self-registration. Off for anything shared.
  user_registration_enabled = var.allow_self_registration
}

# Public: a browser cannot keep a secret. Authorization Code + PKCE.
resource "ferriskey_client" "console" {
  realm         = ferriskey_realm.organization.name
  client_id     = "capitainerie-console"
  name          = "Capitainerie Console"
  client_type   = "public"
  public_client = true

  # Lets integration scripts obtain a user token without a browser.
  direct_access_grants_enabled = var.allow_password_grant

  # FerrisKey compares redirect URIs as exact strings: register each form.
  redirect_uris = toset(flatten([
    for origin in var.console_origins : [origin, "${origin}/", "${origin}/callback"]
  ]))
}

# Owns the API resource (RFC 8707), so it is a party to console tokens and may
# exchange them for a token addressed to an agent when a run is queued.
resource "ferriskey_client" "api" {
  realm       = ferriskey_realm.organization.name
  client_id   = "capitainerie-api"
  name        = "Capitainerie API"
  client_type = "confidential"

  public_client                = false
  service_account_enabled      = false
  direct_access_grants_enabled = false
}

# The Kanban MCP server: the audience of every delegated agent token.
resource "ferriskey_client" "mcp" {
  realm       = ferriskey_realm.organization.name
  client_id   = "capitainerie-mcp"
  name        = "Capitainerie Kanban MCP"
  client_type = "confidential"

  public_client                = false
  service_account_enabled      = false
  direct_access_grants_enabled = false
}

# One client per agent: its own identity, shown as `act` in delegated tokens.
resource "ferriskey_client" "agent" {
  for_each = toset(var.agents)

  realm       = ferriskey_realm.organization.name
  client_id   = "capitainerie-agent-${each.key}"
  name        = "Capitainerie agent ${each.key}"
  client_type = "confidential"

  public_client                = false
  service_account_enabled      = true
  direct_access_grants_enabled = false
}
