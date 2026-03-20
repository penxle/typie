package co.typie.di

import org.koin.core.annotation.Provided
import org.koin.core.module.Module

@Provided
expect class PlatformContext

@Provided
enum class Platform { Android, iOS, Jvm }

expect fun platformModule(): Module
