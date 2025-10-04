resource "aws_iam_user" "external_secrets" {
  name = "external-secrets@k8s"
}

resource "aws_iam_user_policy" "external_secrets" {
  user = aws_iam_user.external_secrets.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = "ssm:GetParameter*"
        Resource = "*"
      }
    ]
  })
}

resource "aws_iam_access_key" "external_secrets" {
  user = aws_iam_user.external_secrets.name
}

resource "aws_iam_user" "external_dns" {
  name = "external-dns@k8s"
}

resource "aws_iam_user_policy" "external_dns" {
  user = aws_iam_user.external_dns.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "route53:ChangeResourceRecordSets",
          "route53:ListResourceRecordSets",
          "route53:ListHostedZones",
          "route53:ListTagsForResource"
        ]
        Resource = "*"
      }
    ]
  })
}

resource "aws_iam_access_key" "external_dns" {
  user = aws_iam_user.external_dns.name
}

resource "aws_iam_user" "cert_manager" {
  name = "cert-manager@k8s"
}

resource "aws_iam_user_policy" "cert_manager" {
  user = aws_iam_user.cert_manager.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "route53:GetChange",
          "route53:ListResourceRecordSets",
          "route53:ListHostedZonesByName",
          "route53:ChangeResourceRecordSets"
        ]
        Resource = "*"
      }
    ]
  })
}

resource "aws_iam_access_key" "cert_manager" {
  user = aws_iam_user.cert_manager.name
}

resource "aws_iam_user" "api" {
  name = "api@k8s"
}

resource "aws_iam_user_policy" "api" {
  user = aws_iam_user.api.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:GetObjectTagging",
          "s3:PutObjectTagging"
        ]
        Resource = [
          "arn:aws:s3:::typie-uploads",
          "arn:aws:s3:::typie-uploads/*",
          "arn:aws:s3:::typie-usercontents",
          "arn:aws:s3:::typie-usercontents/*",
          "arn:aws:s3:::typie-misc",
          "arn:aws:s3:::typie-misc/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "ses:SendEmail",
          "ce:GetCostAndUsage"
        ]
        Resource = "*"
      }
    ]
  })
}

resource "aws_iam_access_key" "api" {
  user = aws_iam_user.api.name
}
