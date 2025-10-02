resource "aws_s3_bucket" "app" {
  bucket = "typie-app"
}

resource "aws_s3_bucket_policy" "app" {
  bucket = aws_s3_bucket.app.bucket

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = ["s3:GetObject"]
        Resource  = ["${aws_s3_bucket.app.arn}/*"]
      }
    ]
  })
}
