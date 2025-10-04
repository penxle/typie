resource "aws_route53_record" "dkim" {
  for_each = toset(
    try(aws_sesv2_email_identity.typie_co.dkim_signing_attributes[0].tokens, [])
  )

  zone_id = var.route53_zone_typie_co_zone_id
  name    = "${each.value}._domainkey.typie.co"
  type    = "CNAME"
  ttl     = 300
  records = ["${each.value}.dkim.amazonses.com"]
}
