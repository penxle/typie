resource "aws_s3_bucket" "uploads" {
  bucket = "typie-uploads"
}

resource "aws_s3_bucket_lifecycle_configuration" "uploads" {
  bucket = aws_s3_bucket.uploads.bucket

  rule {
    id     = "delete-after-1-day"
    status = "Enabled"

    filter {}

    expiration {
      days = 1
    }
  }
}

resource "aws_s3_bucket_cors_configuration" "uploads" {
  bucket = aws_s3_bucket.uploads.bucket

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["POST"]
    allowed_origins = [
      "https://typie.co",
      "https://typie.dev",
      "http://localhost:4100"
    ]
  }
}
