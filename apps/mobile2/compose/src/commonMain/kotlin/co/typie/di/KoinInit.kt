package co.typie.di

import co.typie.platform.PlatformServiceModule
import co.typie.storage.StorageModule
import org.koin.core.annotation.ComponentScan
import org.koin.core.annotation.KoinApplication
import org.koin.core.annotation.Module
import org.koin.dsl.KoinAppDeclaration
import org.koin.dsl.includes
import org.koin.plugin.module.dsl.startKoin

@Module(includes = [StorageModule::class, PlatformServiceModule::class])
@ComponentScan("co.typie")
class AppModule

@KoinApplication(modules = [AppModule::class])
object Application

fun initKoin(config: KoinAppDeclaration? = null) {
  startKoin<Application> {
    includes(config)
    modules(platformModule())
  }
}