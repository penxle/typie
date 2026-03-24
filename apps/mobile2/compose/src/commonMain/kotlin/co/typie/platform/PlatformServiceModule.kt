package co.typie.platform

import co.typie.di.PlatformContext
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
expect class PlatformServiceModule() {
  @Single fun clipboard(ctx: PlatformContext): Clipboard
  @Single fun fileSystem(ctx: PlatformContext): FileSystem
  @Single fun share(ctx: PlatformContext): Share
}
