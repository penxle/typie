resource "aws_acm_certificate" "cf_typie_co" {
  provider = aws.us_east_1

  domain_name               = "typie.co"
  subject_alternative_names = ["*.typie.co"]
  validation_method         = "DNS"
}

resource "aws_acm_certificate" "cf_typie_dev" {
  provider = aws.us_east_1

  domain_name               = "typie.dev"
  subject_alternative_names = ["*.typie.dev"]
  validation_method         = "DNS"
}

resource "aws_acm_certificate" "cf_typie_me" {
  provider = aws.us_east_1

  domain_name               = "typie.me"
  subject_alternative_names = ["*.typie.me"]
  validation_method         = "DNS"
}

resource "aws_acm_certificate" "cf_typie_app" {
  provider = aws.us_east_1

  domain_name               = "typie.app"
  subject_alternative_names = ["*.typie.app"]
  validation_method         = "DNS"
}

resource "aws_acm_certificate" "cf_typie_net" {
  provider = aws.us_east_1

  domain_name               = "typie.net"
  subject_alternative_names = ["*.typie.net"]
  validation_method         = "DNS"
}

resource "aws_acm_certificate" "cf_typie_io" {
  provider = aws.us_east_1

  domain_name               = "typie.io"
  subject_alternative_names = ["*.typie.io"]
  validation_method         = "DNS"
}
