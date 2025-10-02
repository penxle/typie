output "uploads_bucket_arn" {
  description = "ARN of the uploads bucket"
  value       = aws_s3_bucket.uploads.arn
}

output "usercontents_bucket_arn" {
  description = "ARN of the usercontents bucket"
  value       = aws_s3_bucket.usercontents.arn
}

output "misc_bucket_arn" {
  description = "ARN of the misc bucket"
  value       = aws_s3_bucket.misc.arn
}

output "app_bucket_regional_domain_name" {
  description = "Regional domain name of the app bucket"
  value       = aws_s3_bucket.app.bucket_regional_domain_name
}

output "cdn_bucket_regional_domain_name" {
  description = "Regional domain name of the cdn bucket"
  value       = aws_s3_bucket.cdn.bucket_regional_domain_name
}

output "usercontents_bucket_regional_domain_name" {
  description = "Regional domain name of the usercontents bucket"
  value       = aws_s3_bucket.usercontents.bucket_regional_domain_name
}
