resource "aws_sesv2_configuration_set" "typie_co" {
  configuration_set_name = "typie_co"
}

resource "aws_sesv2_email_identity" "typie_co" {
  email_identity         = "typie.co"
  configuration_set_name = aws_sesv2_configuration_set.typie_co.configuration_set_name
}

resource "aws_sesv2_email_identity_mail_from_attributes" "typie_co" {
  email_identity   = aws_sesv2_email_identity.typie_co.email_identity
  mail_from_domain = "mail.typie.co"
}
