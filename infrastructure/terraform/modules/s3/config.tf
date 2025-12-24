resource "aws_s3_bucket" "config" {
  bucket = "typie-config"
}

resource "aws_s3_bucket_policy" "config" {
  bucket = aws_s3_bucket.config.bucket

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = ["s3:GetObject"]
        Resource  = ["${aws_s3_bucket.config.arn}/*"]
      }
    ]
  })
}

locals {
  bootstrap_envs = ["local", "dev", "prod"]
  bootstrap_content = jsonencode({
    version   = 1
    updatedAt = "2024-01-01T00:00:00Z"
    maintenance = {
      enabled   = false
      title     = "서비스 점검 중"
      message   = "더 나은 서비스를 위해 점검 중이에요. 잠시 후 다시 이용해 주세요."
      until     = null
      platforms = []
    }
    minVersion = {
      ios = {
        version  = "1.0.0"
        storeUrl = "https://apps.apple.com/app/id6745595771"
      }
      android = {
        version  = "1.0.0"
        storeUrl = "https://play.google.com/store/apps/details?id=co.typie"
      }
    }
  })
}

resource "aws_s3_object" "bootstrap_json" {
  for_each = toset(local.bootstrap_envs)

  bucket       = aws_s3_bucket.config.bucket
  key          = "bootstrap/${each.value}.json"
  content_type = "application/json"
  content      = local.bootstrap_content

  lifecycle {
    ignore_changes = [content]
  }
}
