resource "cloudflare_dns_record" "typie_co" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "typie.co"
  type    = "CNAME"
  content = "dvnk6johx4te9.cloudfront.net"
  ttl     = 1
}

resource "cloudflare_dns_record" "auth_typie_co" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "auth.typie.co"
  type    = "CNAME"
  content = "dvnk6johx4te9.cloudfront.net"
  ttl     = 1
}

resource "cloudflare_dns_record" "typie_co_txt_gsv1" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "typie.co"
  type    = "TXT"
  content = "\"google-site-verification=Q-1ETLmF6p7XkzQM0wpDyF0wCBQREsjK1aZdxR-4ggQ\""
  ttl     = 1
}

resource "cloudflare_dns_record" "typie_co_txt_gsv2" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "typie.co"
  type    = "TXT"
  content = "\"google-site-verification=hZdtWP44my1tA-wUAvYlOKAAPSp2vHT6M5omQXCRt6o\""
  ttl     = 1
}

resource "cloudflare_dns_record" "typie_co_txt_fdv" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "typie.co"
  type    = "TXT"
  content = "\"facebook-domain-verification=fduiqboyntm5jz4x19bf0pau0ii960\""
  ttl     = 1
}

resource "cloudflare_dns_record" "typie_co_mx" {
  zone_id  = cloudflare_zone.typie_co.id
  name     = "typie.co"
  type     = "MX"
  priority = 1
  content  = "smtp.google.com"
  ttl      = 1
}

resource "cloudflare_dns_record" "typie_me" {
  zone_id = cloudflare_zone.typie_me.id
  name    = "typie.me"
  type    = "CNAME"
  content = "d1r6antjbz3luy.cloudfront.net"
  ttl     = 1
}

resource "cloudflare_dns_record" "wildcard_typie_me" {
  zone_id = cloudflare_zone.typie_me.id
  name    = "*.typie.me"
  type    = "CNAME"
  content = "d1r6antjbz3luy.cloudfront.net"
  ttl     = 1
}

resource "cloudflare_dns_record" "mail_typie_co_mx" {
  zone_id  = cloudflare_zone.typie_co.id
  name     = "mail.typie.co"
  type     = "MX"
  priority = 10
  content  = "feedback-smtp.ap-northeast-2.amazonses.com"
  ttl      = 1
}

resource "cloudflare_dns_record" "mail_typie_co_txt" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "mail.typie.co"
  type    = "TXT"
  content = "\"v=spf1 include:amazonses.com ~all\""
  ttl     = 1
}

resource "cloudflare_dns_record" "dmarc_typie_co_txt" {
  zone_id = cloudflare_zone.typie_co.id
  name    = "_dmarc.typie.co"
  type    = "TXT"
  content = "\"v=DMARC1; p=none;\""
  ttl     = 1
}

resource "cloudflare_dns_record" "dkim_typie_co" {
  for_each = toset(var.ses_dkim_tokens)

  zone_id = cloudflare_zone.typie_co.id
  name    = "${each.value}._domainkey.typie.co"
  type    = "CNAME"
  content = "${each.value}.dkim.amazonses.com"
  ttl     = 1
}

resource "cloudflare_dns_record" "talos_k8s_typie_io" {
  zone_id = cloudflare_zone.typie_io.id
  name    = "talos.k8s.typie.io"
  type    = "A"
  content = "10.0.10.3"
  ttl     = 1
}

resource "cloudflare_dns_record" "controlplane_k8s_typie_io" {
  zone_id = cloudflare_zone.typie_io.id
  name    = "controlplane.k8s.typie.io"
  type    = "A"
  content = "115.68.42.145"
  ttl     = 1
}

resource "cloudflare_dns_record" "ingress_k8s_typie_io" {
  zone_id = cloudflare_zone.typie_io.id
  name    = "ingress.k8s.typie.io"
  type    = "A"
  content = "115.68.42.155"
  ttl     = 1
}

resource "cloudflare_dns_record" "typie_net" {
  zone_id = cloudflare_zone.typie_net.id
  name    = "typie.net"
  type    = "CNAME"
  content = "d2qdhlm1riz8yl.cloudfront.net"
  ttl     = 1
}

resource "cloudflare_dns_record" "cdn_typie_net" {
  zone_id = cloudflare_zone.typie_net.id
  name    = "cdn.typie.net"
  type    = "CNAME"
  content = "d3cukiokgj3htl.cloudfront.net"
  ttl     = 1
}
