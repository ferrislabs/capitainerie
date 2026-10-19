variable "ferriskey_url" {
  description = "Base URL of the FerrisKey instance."
  type        = string
  default     = "http://localhost:4110"
}

variable "realm" {
  description = "Realm of the demo organization."
  type        = string
  default     = "capitainerie"
}

variable "admin_username" {
  description = "Initial admin account, used for the bootstrap password grant."
  type        = string
  default     = "admin"
}

variable "admin_password" {
  description = "Password of the bootstrap admin account; pass TF_VAR_admin_password anywhere but a local stack."
  type        = string
  sensitive   = true
  default     = "admin"
}

variable "console_origins" {
  description = "Origins the console is served from."
  type        = set(string)
  default     = ["http://localhost:4080"]
}

variable "access_token_lifetime" {
  description = <<-DESC
    Access token lifetime, in seconds.

    30 minutes on the local stack only: a delegated token never outlives the
    user token it comes from, and FerrisKey has no durable delegation grant yet
    (docs/ferriskey-needs.md, need 2). Keep 300 anywhere else.
  DESC
  type        = number
  default     = 1800
}

variable "allow_self_registration" {
  description = "Whether anyone reaching the login page may create an account. Local stack only."
  type        = bool
  default     = true
}

variable "allow_password_grant" {
  description = "Whether the console client accepts the password grant. Local stack only."
  type        = bool
  default     = true
}

variable "agents" {
  description = "Agents of the organization; each becomes the client capitainerie-agent-<name>. The writer runs the meeting pipeline and the captain answers in the chat. The executor that runs Claude Code on a card has no client: it writes through the worker only."
  type        = list(string)
  default     = ["writer", "captain"]
}

variable "api_resource" {
  description = "Resource indicator (RFC 8707) the console requests tokens for."
  type        = string
  default     = "http://localhost:4100"
}

variable "session_lifetime" {
  description = "Refresh token lifetime, which bounds the user session, in seconds. Local stack only."
  type        = number
  default     = 28800
}
