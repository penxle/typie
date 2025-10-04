resource "aws_s3_bucket" "usercontents" {
  bucket = "typie-usercontents"
}

resource "aws_s3_bucket_policy" "usercontents" {
  bucket = aws_s3_bucket.usercontents.bucket

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = ["s3:GetObject"]
        Resource  = ["${aws_s3_bucket.usercontents.arn}/*"]
      }
    ]
  })
}

resource "aws_s3_bucket_lifecycle_configuration" "usercontents" {
  bucket = aws_s3_bucket.usercontents.bucket

  rule {
    id     = "transition-to-intelligent-tiering"
    status = "Enabled"

    filter {}

    transition {
      days          = 0
      storage_class = "INTELLIGENT_TIERING"
    }
  }
}
