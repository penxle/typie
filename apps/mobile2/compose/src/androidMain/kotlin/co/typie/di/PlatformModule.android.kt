package co.typie.di

import android.content.Context
import org.koin.core.annotation.Provided
import org.koin.dsl.module

@Provided
actual class PlatformContext(val context: Context)

actual fun platformModule() = module {
  single { Platform.Android }
  single { PlatformContext(get()) }
}
