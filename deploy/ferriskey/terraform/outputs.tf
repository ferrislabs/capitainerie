output "issuer" {
  value = "${var.ferriskey_url}/realms/${ferriskey_realm.organization.name}"
}

output "realm" {
  value = ferriskey_realm.organization.name
}

output "console_client_id" {
  value = ferriskey_client.console.client_id
}
