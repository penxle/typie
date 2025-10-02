resource "aws_cloudfront_cache_policy" "dynamic" {
  name        = "DynamicContents"
  comment     = "Cache policy for dynamic contents"
  min_ttl     = 0
  default_ttl = 0
  max_ttl     = 31536000

  parameters_in_cache_key_and_forwarded_to_origin {
    enable_accept_encoding_brotli = true
    enable_accept_encoding_gzip   = true

    cookies_config {
      cookie_behavior = "none"
    }

    headers_config {
      header_behavior = "none"
    }

    query_strings_config {
      query_string_behavior = "none"
    }
  }
}

resource "aws_cloudfront_origin_request_policy" "dynamic" {
  name    = "DynamicContents"
  comment = "Origin request policy for dynamic contents"

  cookies_config {
    cookie_behavior = "all"
  }

  headers_config {
    header_behavior = "allViewerAndWhitelistCloudFront"

    headers {
      items = [
        "CloudFront-Viewer-Address",
        "CloudFront-Viewer-Country-Name",
        "CloudFront-Viewer-Country-Region-Name",
        "CloudFront-Viewer-City",
      ]
    }
  }

  query_strings_config {
    query_string_behavior = "all"
  }
}

resource "aws_cloudfront_response_headers_policy" "dynamic" {
  name    = "DynamicContents"
  comment = "Response headers policy for dynamic contents"

  security_headers_config {
    strict_transport_security {
      override                   = true
      access_control_max_age_sec = 31536000
      include_subdomains         = true
      preload                    = true
    }
  }
}

resource "aws_cloudfront_cache_policy" "static" {
  name        = "StaticOrigin"
  comment     = "Cache policy for static contents"
  min_ttl     = 0
  default_ttl = 86400
  max_ttl     = 31536000

  parameters_in_cache_key_and_forwarded_to_origin {
    enable_accept_encoding_brotli = true
    enable_accept_encoding_gzip   = true

    cookies_config {
      cookie_behavior = "none"
    }

    headers_config {
      header_behavior = "none"
    }

    query_strings_config {
      query_string_behavior = "all"
    }
  }
}

resource "aws_cloudfront_origin_request_policy" "static" {
  name    = "StaticOrigin"
  comment = "Origin request policy for static origins"

  cookies_config {
    cookie_behavior = "none"
  }

  headers_config {
    header_behavior = "none"
  }

  query_strings_config {
    query_string_behavior = "all"
  }
}

resource "aws_cloudfront_response_headers_policy" "static" {
  name    = "StaticOrigin"
  comment = "Response headers policy for static origins"

  cors_config {
    access_control_allow_credentials = false
    origin_override                  = true

    access_control_allow_headers {
      items = ["*"]
    }

    access_control_allow_methods {
      items = ["GET"]
    }

    access_control_allow_origins {
      items = ["*"]
    }
  }

  custom_headers_config {
    items {
      header   = "Cache-Control"
      value    = "public, max-age=31536000, immutable"
      override = true
    }
  }

  security_headers_config {
    strict_transport_security {
      override                   = true
      access_control_max_age_sec = 31536000
      include_subdomains         = true
      preload                    = true
    }
  }
}
