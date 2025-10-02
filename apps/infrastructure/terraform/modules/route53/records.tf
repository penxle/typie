resource "aws_route53_record" "typie_co_txt" {
  zone_id = aws_route53_zone.typie_co.zone_id
  name    = "typie.co"
  type    = "TXT"
  ttl     = 300

  records = [
    "google-site-verification=Q-1ETLmF6p7XkzQM0wpDyF0wCBQREsjK1aZdxR-4ggQ",
    "google-site-verification=hZdtWP44my1tA-wUAvYlOKAAPSp2vHT6M5omQXCRt6o",
    "facebook-domain-verification=fduiqboyntm5jz4x19bf0pau0ii960",
  ]
}

resource "aws_route53_record" "typie_co_mx" {
  zone_id = aws_route53_zone.typie_co.zone_id
  name    = "typie.co"
  type    = "MX"
  ttl     = 300

  records = [
    "1 smtp.google.com",
  ]
}

resource "aws_route53_record" "talos_k8s_typie_io" {
  zone_id = aws_route53_zone.typie_io.zone_id
  name    = "talos.k8s.typie.io"
  type    = "A"
  ttl     = 300

  records = [
    "10.0.10.3",
  ]
}

resource "aws_route53_record" "controlplane_k8s_typie_io" {
  zone_id = aws_route53_zone.typie_io.zone_id
  name    = "controlplane.k8s.typie.io"
  type    = "A"
  ttl     = 300

  records = [
    "115.68.42.145",
  ]
}

resource "aws_route53_record" "ingress_k8s_typie_io" {
  zone_id = aws_route53_zone.typie_io.zone_id
  name    = "ingress.k8s.typie.io"
  type    = "A"
  ttl     = 300

  records = [
    "115.68.42.155",
  ]
}
