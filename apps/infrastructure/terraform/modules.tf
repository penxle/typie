module "cloudfront" {
  source = "./modules/cloudfront"

  route53_zone_typie_net_zone_id = module.route53.zone_typie_net_zone_id

  providers = {
    aws           = aws
    aws.us_east_1 = aws.us_east_1
  }
}

module "ecr" {
  source = "./modules/ecr"
}

module "iam" {
  source = "./modules/iam"
}

module "route53" {
  source = "./modules/route53"
}

module "s3" {
  source = "./modules/s3"
}

module "ses" {
  source = "./modules/ses"

  route53_zone_typie_co_zone_id = module.route53.zone_typie_co_zone_id
}
