package co.typie.storage

import co.typie.di.PlatformContext
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy
import org.koin.core.annotation.Module
import org.koin.core.annotation.Named
import org.koin.core.annotation.Single

@Module
actual class StorageModule {
  @Single
  @Named("ksafe.prefs")
  actual fun providesKSafePrefs(ctx: PlatformContext): KSafe =
    KSafe(fileName = "prefs", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)

  @Single
  @Named("ksafe.vault")
  actual fun providesKSafeVault(ctx: PlatformContext): KSafe =
    KSafe(fileName = "vault")
}
