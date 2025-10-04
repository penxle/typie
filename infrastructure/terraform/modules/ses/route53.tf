resource "aws_route53_record" "mail_typie_co_mx" {
  zone_id = var.route53_zone_typie_co_zone_id
  name    = "mail.typie.co"
  type    = "MX"
  ttl     = 300
  records = ["10 feedback-smtp.ap-northeast-2.amazonses.com"]
}

resource "aws_route53_record" "mail_typie_co_txt" {
  zone_id = var.route53_zone_typie_co_zone_id
  name    = "mail.typie.co"
  type    = "TXT"
  ttl     = 300
  records = ["v=spf1 include:amazonses.com ~all"]
}

resource "aws_route53_record" "dmarc_typie_co_txt" {
  zone_id = var.route53_zone_typie_co_zone_id
  name    = "_dmarc.typie.co"
  type    = "TXT"
  ttl     = 300
  records = ["v=DMARC1; p=none;"]
}
