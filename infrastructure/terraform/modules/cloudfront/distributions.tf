resource "aws_cloudfront_distribution" "app" {
  enabled      = true
  http_version = "http2and3"
  aliases      = ["app.typie.net"]

  origin {
    origin_id                = "s3"
    domain_name              = "typie-app.s3.ap-northeast-2.amazonaws.com"
    origin_access_control_id = aws_cloudfront_origin_access_control.s3.id
  }

  default_cache_behavior {
    target_origin_id = "s3"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]

    cache_policy_id            = aws_cloudfront_cache_policy.static.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.static.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.static.id
  }

  ordered_cache_behavior {
    target_origin_id = "s3"
    path_pattern     = "*.json"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]

    cache_policy_id            = aws_cloudfront_cache_policy.dynamic.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.static.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.dynamic.id
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.cf_typie_net.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  wait_for_deployment = false
}

resource "aws_cloudfront_distribution" "cdn" {
  enabled      = true
  http_version = "http2and3"
  aliases      = ["cdn.typie.net"]

  origin {
    origin_id                = "s3"
    domain_name              = "typie-cdn.s3.ap-northeast-2.amazonaws.com"
    origin_access_control_id = aws_cloudfront_origin_access_control.s3.id
  }

  default_cache_behavior {
    target_origin_id = "s3"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]

    cache_policy_id            = aws_cloudfront_cache_policy.static.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.static.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.static.id
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.cf_typie_net.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  wait_for_deployment = false
}

resource "aws_cloudfront_distribution" "usercontents" {
  enabled      = true
  http_version = "http2and3"
  aliases      = ["typie.net"]

  origin {
    origin_id                = "s3"
    domain_name              = "typie-usercontents.s3.ap-northeast-2.amazonaws.com"
    origin_access_control_id = aws_cloudfront_origin_access_control.s3.id
  }

  origin {
    origin_id                = "lambda"
    domain_name              = "usercontents-literoo-dsqhecmpgp5romu8x8rbkcmbapn2a--ol-s3.s3.ap-northeast-2.amazonaws.com"
    origin_access_control_id = aws_cloudfront_origin_access_control.s3.id

    origin_shield {
      enabled = true
      origin_shield_region = "ap-northeast-2"
    }
  }

  default_cache_behavior {
    target_origin_id = "s3"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]

    cache_policy_id            = aws_cloudfront_cache_policy.static.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.static.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.static.id
  }

  ordered_cache_behavior {
    target_origin_id = "lambda"
    path_pattern     = "images/*"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]
    cache_policy_id = aws_cloudfront_cache_policy.static.id

    origin_request_policy_id   = aws_cloudfront_origin_request_policy.static.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.static.id
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.cf_typie_net.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  wait_for_deployment = false
}

resource "aws_cloudfront_distribution" "typie_co" {
  enabled      = true
  http_version = "http2and3"
  aliases      = ["typie.co", "auth.typie.co"]

  origin {
    origin_id   = "k8s"
    domain_name = "ingress.k8s.typie.io"

    custom_origin_config {
      http_port                = 80
      https_port               = 443
      origin_protocol_policy   = "https-only"
      origin_ssl_protocols     = ["TLSv1.2"]
      origin_read_timeout      = 60
      origin_keepalive_timeout = 60
    }
  }

  default_cache_behavior {
    target_origin_id = "k8s"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]

    cache_policy_id            = aws_cloudfront_cache_policy.dynamic.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.dynamic.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.dynamic.id
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.cf_typie_co.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  wait_for_deployment = false
}

resource "aws_cloudfront_distribution" "typie_me" {
  enabled      = true
  http_version = "http2and3"
  aliases      = ["typie.me", "*.typie.me"]

  origin {
    origin_id   = "k8s"
    domain_name = "ingress.k8s.typie.io"

    custom_origin_config {
      http_port                = 80
      https_port               = 443
      origin_protocol_policy   = "https-only"
      origin_ssl_protocols     = ["TLSv1.2"]
      origin_read_timeout      = 60
      origin_keepalive_timeout = 60
    }
  }

  default_cache_behavior {
    target_origin_id = "k8s"

    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    allowed_methods = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods  = ["GET", "HEAD", "OPTIONS"]

    cache_policy_id            = aws_cloudfront_cache_policy.dynamic.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.dynamic.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.dynamic.id
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.cf_typie_me.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  wait_for_deployment = false
}
