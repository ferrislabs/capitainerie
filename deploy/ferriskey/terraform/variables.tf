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
  description = "Agents of the meeting pipeline; each becomes the client capitainerie-agent-<name>."
  type        = list(string)
  default     = ["writer"]
}

variable "api_resource" {
  description = "Resource indicator (RFC 8707) the console requests tokens for."
  type        = string
  default     = "http://localhost:4100"
}
