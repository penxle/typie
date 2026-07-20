package co.typie

import cocoapods.Sentry.experimental
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.cinterop.ExperimentalForeignApi
import platform.Foundation.NSBundle

@OptIn(ExperimentalForeignApi::class)
fun doInitSentry() {
  val bundle = NSBundle.mainBundle
  val version = bundle.objectForInfoDictionaryKey("CFBundleShortVersionString")
  val build = bundle.objectForInfoDictionaryKey("CFBundleVersion")

  Sentry.initWithPlatformOptions { options ->
    options.dsn = Konfig.SENTRY_DSN
    options.sendDefaultPii = true
    options.attachScreenshot = true
    options.releaseName = "${bundle.bundleIdentifier}@$version+$build"
    options.maxBreadcrumbs = 300uL
    options.enableAppHangTrackingV2 = true
    options.enableSigtermReporting = true
    options.enableMetricKit = true
    options.experimental().setEnableUnhandledCPPExceptionsV2(true)
  }
}
