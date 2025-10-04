resource "aws_s3_bucket" "misc" {
  bucket = "typie-misc"
}

resource "aws_s3_bucket" "logs" {
  bucket = "typie-logs"
}

resource "aws_s3_bucket" "backups" {
  bucket = "typie-backups"
}

resource "aws_s3_bucket" "postgres" {
  bucket = "typie-postgres"
}
