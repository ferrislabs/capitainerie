output "issuer" {
  value = "${var.ferriskey_url}/realms/${ferriskey_realm.organization.name}"
}

output "realm" {
  value = ferriskey_realm.organization.name
}

output "console_client_id" {
  value = ferriskey_client.console.client_id
}

output "api_client_uuid" {
  value = ferriskey_client.api.client_uuid
}

output "mcp_client_id" {
  value = ferriskey_client.mcp.client_id
}

output "agent_clients" {
  description = "client_id -> client_uuid of each agent."
  value       = { for name, client in ferriskey_client.agent : client.client_id => client.client_uuid }
}

output "api_resource" {
  value = var.api_resource
}

output "admin_role_uuid" {
  description = "capitainerie-admin; the bootstrap script grants it to ADMIN_USERNAME."
  value       = ferriskey_role.admin.role_uuid
}
