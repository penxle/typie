resource "aws_iam_role" "doppler" {
  name = "doppler"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { AWS = "arn:aws:iam::299900769157:root" }
        Action    = "sts:AssumeRole"
        Condition = {
          StringEquals = {
            "sts:ExternalId" = "a622bad8b546cbb9c2be"
          }
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "doppler" {
  role = aws_iam_role.doppler.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect   = "Allow"
        Action   = [
          "ssm:PutParameter",
          "ssm:LabelParameterVersion",
          "ssm:DeleteParameter",
          "ssm:RemoveTagsFromResource",
          "ssm:GetParameterHistory",
          "ssm:AddTagsToResource",
          "ssm:GetParametersByPath",
          "ssm:GetParameters",
          "ssm:GetParameter",
          "ssm:DeleteParameters"
        ]
        Resource = "*"
      }
    ]
  })
}
