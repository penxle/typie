resource "aws_iam_role" "bmo_lambda" {
  name = "bmo@lambda"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "bmo_basic_execution" {
  role       = aws_iam_role.bmo_lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy" "bmo_invoke_worker" {
  name = "bmo-invoke-worker"
  role = aws_iam_role.bmo_lambda.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "lambda:InvokeFunction"
      Resource = aws_lambda_function.bmo_worker.arn
    }]
  })
}

resource "aws_iam_role_policy" "bmo_dynamodb" {
  name = "bmo-dynamodb"
  role = aws_iam_role.bmo_lambda.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Action = [
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:DeleteItem",
      ]
      Resource = aws_dynamodb_table.bmo_sessions.arn
    }]
  })
}

resource "aws_iam_role_policy" "bmo_ssm" {
  name = "bmo-ssm"
  role = aws_iam_role.bmo_lambda.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "ssm:GetParameter"
      Resource = "arn:aws:ssm:ap-northeast-2:*:parameter/bmo/*"
    }]
  })
}

resource "aws_ecr_repository" "bmo_worker" {
  name                 = "bmo-worker"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = false
  }
}

resource "aws_ecr_lifecycle_policy" "bmo_worker" {
  repository = aws_ecr_repository.bmo_worker.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last 5 images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = 5
      }
      action = {
        type = "expire"
      }
    }]
  })
}

resource "aws_lambda_function" "bmo_webhook" {
  function_name = "bmo-webhook"
  role          = aws_iam_role.bmo_lambda.arn

  architectures = ["arm64"]
  memory_size   = 256
  timeout       = 10

  runtime  = "nodejs24.x"
  handler  = "handler.handler"
  filename = "${path.module}/../../../../apps/bmo/dist/webhook.zip"
  source_code_hash = filebase64sha256("${path.module}/../../../../apps/bmo/dist/webhook.zip")

  environment {
    variables = {
      WORKER_FUNCTION_NAME = aws_lambda_function.bmo_worker.function_name
    }
  }
}

resource "aws_lambda_function_url" "bmo_webhook" {
  function_name      = aws_lambda_function.bmo_webhook.function_name
  authorization_type = "NONE"
}

resource "aws_lambda_permission" "bmo_webhook_public_url" {
  function_name           = aws_lambda_function.bmo_webhook.function_name
  action                  = "lambda:InvokeFunctionUrl"
  principal               = "*"
  function_url_auth_type  = "NONE"
}

resource "aws_lambda_permission" "bmo_webhook_public_invoke" {
  function_name = aws_lambda_function.bmo_webhook.function_name
  action        = "lambda:InvokeFunction"
  principal     = "*"
}

resource "aws_lambda_function" "bmo_worker" {
  function_name = "bmo-worker"
  role          = aws_iam_role.bmo_lambda.arn

  architectures = ["arm64"]
  memory_size   = 10240
  timeout       = 900

  package_type = "Image"
  image_uri    = "${aws_ecr_repository.bmo_worker.repository_url}:latest"

  environment {
    variables = {}
  }
}

resource "aws_lambda_function_event_invoke_config" "bmo_worker" {
  function_name                = aws_lambda_function.bmo_worker.function_name
  maximum_retry_attempts       = 0
  maximum_event_age_in_seconds = 960
}

resource "aws_iam_role_policy" "bmo_s3" {
  name = "bmo-s3"
  role = aws_iam_role.bmo_lambda.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect   = "Allow"
        Action   = "s3:ListBucket"
        Resource = "arn:aws:s3:::typie-misc"
        Condition = {
          StringLike = {
            "s3:prefix" = ["bmo/sessions/*"]
          }
        }
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
        ]
        Resource = "arn:aws:s3:::typie-misc/bmo/sessions/*"
      },
    ]
  })
}

resource "aws_dynamodb_table" "bmo_sessions" {
  name         = "bmo-sessions"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "threadKey"

  attribute {
    name = "threadKey"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }
}
