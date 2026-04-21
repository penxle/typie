package co.typie

import co.touchlab.kermit.CommonWriter
import co.touchlab.kermit.Logger

fun doInitLogger() {
  Logger.addLogWriter(CommonWriter())
}
