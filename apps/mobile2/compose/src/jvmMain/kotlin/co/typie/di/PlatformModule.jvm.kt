package co.typie.di

import org.koin.core.annotation.Provided
import org.koin.dsl.module

@Provided
actual class PlatformContext

actual fun platformModule() = module {
  single { Platform.Jvm }
  single { PlatformContext() }
}
