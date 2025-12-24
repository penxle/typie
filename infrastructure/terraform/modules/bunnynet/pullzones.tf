resource "bunnynet_pullzone" "typie_ingress" {
  name = "typie-ingress"

  origin {
    type                = "OriginUrl"
    url                 = "https://ingress.k8s.typie.io"
    forward_host_header = true
  }

  routing {
    tier = "Standard"
  }

  strip_cookies = false
  cache_vary    = ["hostname"]
}

resource "bunnynet_pullzone_hostname" "typie_co" {
  pullzone    = bunnynet_pullzone.typie_ingress.id
  name        = "typie.co"
  tls_enabled = true
  force_ssl   = true
}

resource "bunnynet_pullzone_hostname" "auth_typie_co" {
  pullzone    = bunnynet_pullzone.typie_ingress.id
  name        = "auth.typie.co"
  tls_enabled = true
  force_ssl   = true
}

resource "bunnynet_pullzone_hostname" "typie_me" {
  pullzone    = bunnynet_pullzone.typie_ingress.id
  name        = "typie.me"
  tls_enabled = true
  force_ssl   = true
}

resource "bunnynet_pullzone_hostname" "wildcard_typie_me" {
  pullzone    = bunnynet_pullzone.typie_ingress.id
  name        = "*.typie.me"
  tls_enabled = true
  force_ssl   = true
}
