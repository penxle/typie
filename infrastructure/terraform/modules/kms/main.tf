resource "aws_kms_key" "sops" {
  description         = "SOPS encryption key"
  enable_key_rotation = true
}

resource "aws_kms_alias" "sops" {
  name          = "alias/sops"
  target_key_id = aws_kms_key.sops.key_id
}

output "sops_key_arn" {
  value = aws_kms_key.sops.arn
}
