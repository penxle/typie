output "dkim_tokens" {
  value = try(aws_sesv2_email_identity.typie_co.dkim_signing_attributes[0].tokens, [])
}
