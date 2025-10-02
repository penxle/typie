resource "aws_route53_record" "app" {
  zone_id = var.route53_zone_typie_net_zone_id
  name    = "app.typie.net"
  type    = "A"

  alias {
    name                   = aws_cloudfront_distribution.app.domain_name
    zone_id                = aws_cloudfront_distribution.app.hosted_zone_id
    evaluate_target_health = false
  }
}

resource "aws_route53_record" "cdn" {
  zone_id = var.route53_zone_typie_net_zone_id
  name    = "cdn.typie.net"
  type    = "A"

  alias {
    name                   = aws_cloudfront_distribution.cdn.domain_name
    zone_id                = aws_cloudfront_distribution.cdn.hosted_zone_id
    evaluate_target_health = false
  }
}

resource "aws_route53_record" "usercontents" {
  zone_id = var.route53_zone_typie_net_zone_id
  name    = "typie.net"
  type    = "A"

  alias {
    name                   = aws_cloudfront_distribution.usercontents.domain_name
    zone_id                = aws_cloudfront_distribution.usercontents.hosted_zone_id
    evaluate_target_health = false
  }
}
