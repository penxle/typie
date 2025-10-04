output "zone_typie_co_zone_id" {
  description = "Zone ID for typie.co"
  value       = aws_route53_zone.typie_co.zone_id
}

output "zone_typie_me_zone_id" {
  description = "Zone ID for typie.me"
  value       = aws_route53_zone.typie_me.zone_id
}

output "zone_typie_net_zone_id" {
  description = "Zone ID for typie.net"
  value       = aws_route53_zone.typie_net.zone_id
}
