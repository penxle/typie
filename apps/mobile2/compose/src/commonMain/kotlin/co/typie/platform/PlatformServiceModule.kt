package co.typie.platform

import co.typie.di.PlatformContext
import co.typie.migration.LegacyMigrationPlatformSource
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
expect class PlatformServiceModule() {
  @Single fun clipboard(ctx: PlatformContext): Clipboard
  @Single fun deviceInfo(ctx: PlatformContext): DeviceInfo
  @Single fun fileSystem(ctx: PlatformContext): FileSystem
  @Single fun legacyMigrationPlatformSource(ctx: PlatformContext): LegacyMigrationPlatformSource
  @Single fun purchaseService(ctx: PlatformContext): PurchaseService
  @Single fun share(ctx: PlatformContext): Share
}
