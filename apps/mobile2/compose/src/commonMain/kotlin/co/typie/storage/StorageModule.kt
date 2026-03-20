package co.typie.storage

import co.typie.di.PlatformContext
import eu.anifantakis.lib.ksafe.KSafe
import org.koin.core.annotation.Module
import org.koin.core.annotation.Named
import org.koin.core.annotation.Single

@Module
expect class StorageModule() {
  @Single
  @Named("ksafe.prefs")
  fun providesKSafePrefs(ctx: PlatformContext): KSafe

  @Single
  @Named("ksafe.vault")
  fun providesKSafeVault(ctx: PlatformContext): KSafe
}
