resource "aws_s3_bucket" "cdn" {
  bucket = "typie-cdn"
}

resource "aws_s3_bucket_policy" "cdn" {
  bucket = aws_s3_bucket.cdn.bucket

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = ["s3:GetObject"]
        Resource  = ["${aws_s3_bucket.cdn.arn}/*"]
      }
    ]
  })
}
