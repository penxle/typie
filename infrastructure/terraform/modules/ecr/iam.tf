resource "aws_iam_user" "ecr_credential_provider" {
  name = "ecr-credential-provider@k8s"

  tags = {
    Name = "ecr-credential-provider@k8s"
  }
}

resource "aws_iam_user_policy_attachment" "ecr_credential_provider" {
  user       = aws_iam_user.ecr_credential_provider.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
}

resource "aws_iam_access_key" "ecr_credential_provider" {
  user = aws_iam_user.ecr_credential_provider.name
}
