resource "aws_ssm_parameter" "external_secrets_aws_access_key_id" {
  name  = "/external_secrets/aws_access_key_id"
  type  = "SecureString"
  value = aws_iam_access_key.external_secrets.id
}

resource "aws_ssm_parameter" "external_secrets_aws_secret_access_key" {
  name  = "/external_secrets/aws_secret_access_key"
  type  = "SecureString"
  value = aws_iam_access_key.external_secrets.secret
}

resource "aws_ssm_parameter" "external_dns_aws_access_key_id" {
  name  = "/external_dns/aws_access_key_id"
  type  = "SecureString"
  value = aws_iam_access_key.external_dns.id
}

resource "aws_ssm_parameter" "external_dns_aws_secret_access_key" {
  name  = "/external_dns/aws_secret_access_key"
  type  = "SecureString"
  value = aws_iam_access_key.external_dns.secret
}

resource "aws_ssm_parameter" "cert_manager_aws_access_key_id" {
  name  = "/cert_manager/aws_access_key_id"
  type  = "SecureString"
  value = aws_iam_access_key.cert_manager.id
}

resource "aws_ssm_parameter" "cert_manager_aws_secret_access_key" {
  name  = "/cert_manager/aws_secret_access_key"
  type  = "SecureString"
  value = aws_iam_access_key.cert_manager.secret
} 

resource "aws_ssm_parameter" "api_aws_access_key_id" {
  name  = "/api/aws_access_key_id"
  type  = "SecureString"
  value = aws_iam_access_key.api.id
}

resource "aws_ssm_parameter" "api_aws_secret_access_key" {
  name  = "/api/aws_secret_access_key"
  type  = "SecureString"
  value = aws_iam_access_key.api.secret
}
