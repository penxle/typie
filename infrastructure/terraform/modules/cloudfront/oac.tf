resource "aws_cloudfront_origin_access_control" "s3" {
  name                              = "s3"
  description                       = "Origin access control for S3 origins"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}
