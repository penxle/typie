output "lambda_object_access_point_literoom_domain" {
  description = "Domain name of the S3 Object Lambda Access Point for literoom"
  value       = "${aws_s3control_object_lambda_access_point.usercontents_literoom.alias}.s3.${aws_s3control_object_lambda_access_point.usercontents_literoom.region}.amazonaws.com"
}
