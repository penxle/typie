package co.typie

import io.sentry.kotlin.multiplatform.Sentry

fun doInitSentry() {
  Sentry.init { options ->
    options.dsn = Konfig.SENTRY_DSN
    options.sendDefaultPii = true
    options.attachScreenshot = true
  }
}
