resource "aws_iam_role" "literoom_lambda" {
  name = "literoom@lambda"

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

resource "aws_iam_role_policy_attachment" "literoom_lambda_basic_execution" {
  role       = aws_iam_role.literoom_lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_lambda_layer_version" "sharp" {
  layer_name          = "sharp"
  compatible_runtimes = ["nodejs22.x"]
  compatible_architectures = ["arm64"]
  filename            = "${path.module}/../../../../apps/literoom/dist/layers/sharp.zip"
  source_code_hash    = filebase64sha256("${path.module}/../../../../apps/literoom/dist/layers/sharp.zip")
}

resource "aws_lambda_function" "literoom" {
  function_name = "literoom"
  role          = aws_iam_role.literoom_lambda.arn

  architectures = ["arm64"]
  memory_size   = 10240
  timeout       = 900

  runtime = "nodejs22.x"
  handler = "handler.handler"
  layers  = [aws_lambda_layer_version.sharp.arn]

  filename         = "${path.module}/../../../../apps/literoom/dist/function.zip"
  source_code_hash = filebase64sha256("${path.module}/../../../../apps/literoom/dist/function.zip")
}

resource "aws_lambda_permission" "literoom" {
  function_name = aws_lambda_function.literoom.function_name
  principal     = "cloudfront.amazonaws.com"
  action        = "lambda:InvokeFunction"
}

resource "aws_s3_access_point" "usercontents" {
  name   = "usercontents"
  bucket = "typie-usercontents"
}

resource "aws_s3control_access_point_policy" "usercontents" {
  access_point_arn = aws_s3_access_point.usercontents.arn

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "cloudfront.amazonaws.com"
      }
      Action   = "s3:*"
      Resource = [
        aws_s3_access_point.usercontents.arn,
        "${aws_s3_access_point.usercontents.arn}/object/*"
      ]
    }]
  })
}

resource "aws_s3control_object_lambda_access_point" "usercontents_literoom" {
  name = "usercontents-literoom"

  configuration {
    supporting_access_point = aws_s3_access_point.usercontents.arn

    transformation_configuration {
      actions = ["GetObject"]

      content_transformation {
        aws_lambda {
          function_arn = aws_lambda_function.literoom.arn
        }
      }
    }
  }
}

resource "aws_s3control_object_lambda_access_point_policy" "usercontents_literoom" {
  name = aws_s3control_object_lambda_access_point.usercontents_literoom.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "cloudfront.amazonaws.com"
      }
      Action   = "s3-object-lambda:Get*"
      Resource = aws_s3control_object_lambda_access_point.usercontents_literoom.arn
    }]
  })
}

resource "aws_iam_role_policy" "literoom_lambda" {
  name = "literoom@lambda"
  role = aws_iam_role.literoom_lambda.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Action = ["s3-object-lambda:WriteGetObjectResponse"]
      Resource = [aws_s3control_object_lambda_access_point.usercontents_literoom.arn]
    }]
  })
}
