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
  refresh_token_lifetime = 1800

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
