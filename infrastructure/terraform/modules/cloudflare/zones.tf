resource "cloudflare_zone" "typie_co" {
  account = {
    id = var.account_id
  }

  name = "typie.co"
  type = "full"
}

resource "cloudflare_zone" "typie_me" {
  account = {
    id = var.account_id
  }

  name = "typie.me"
  type = "full"
}

resource "cloudflare_zone" "typie_dev" {
  account = {
    id = var.account_id
  }

  name = "typie.dev"
  type = "full"
}

resource "cloudflare_zone" "typie_app" {
  account = {
    id = var.account_id
  }

  name = "typie.app"
  type = "full"
}

resource "cloudflare_zone" "typie_net" {
  account = {
    id = var.account_id
  }

  name = "typie.net"
  type = "full"
}

resource "cloudflare_zone" "typie_io" {
  account = {
    id = var.account_id
  }

  name = "typie.io"
  type = "full"
}
