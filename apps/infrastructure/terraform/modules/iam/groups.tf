resource "aws_iam_group" "team" {
  name = "team"
}

resource "aws_iam_group_policy" "team" {
  group = aws_iam_group.team.name
  name  = "team"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue",
          "secretsmanager:DescribeSecret"
        ]
        Resource = [
          "arn:aws:secretsmanager:*:*:secret:/apps/*/local-*",
          "arn:aws:secretsmanager:*:*:secret:/apps/*/dev-*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject"
        ]
        Resource = ["arn:aws:s3:::typie-uploads/*"]
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:GetObjectTagging",
          "s3:PutObjectTagging"
        ]
        Resource = ["arn:aws:s3:::typie-usercontents/*"]
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:GetObjectTagging",
          "s3:PutObjectTagging"
        ]
        Resource = ["arn:aws:s3:::typie-misc/*"]
      },
      {
        Effect   = "Allow"
        Action   = ["ses:SendEmail"]
        Resource = ["*"]
      },
      {
        Effect   = "Allow"
        Action   = ["ce:GetCostAndUsage"]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "iam:GetUser",
          "iam:ListAccessKeys",
          "iam:ListGroupsForUser",
          "iam:ChangePassword",
          "iam:CreateAccessKey",
          "iam:DeleteAccessKey",
          "iam:UpdateAccessKey",
          "iam:GetAccessKeyLastUsed",
          "iam:CreateVirtualMFADevice",
          "iam:DeleteVirtualMFADevice",
          "iam:EnableMFADevice",
          "iam:DeactivateMFADevice",
          "iam:ListMFADevices",
          "iam:ResyncMFADevice"
        ]
        Resource = "arn:aws:iam::*:user/$${aws:username}"
      }
    ]
  })
}
