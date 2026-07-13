package co.typie

import io.sentry.kotlin.multiplatform.Sentry
import platform.Foundation.NSBundle

fun doInitSentry() {
  val bundle = NSBundle.mainBundle
  val version = bundle.objectForInfoDictionaryKey("CFBundleShortVersionString")
  val build = bundle.objectForInfoDictionaryKey("CFBundleVersion")

  Sentry.init { options ->
    options.dsn = Konfig.SENTRY_DSN
    options.sendDefaultPii = true
    options.attachScreenshot = true
    options.release = "${bundle.bundleIdentifier}@$version+$build"
  }
}
