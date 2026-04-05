package co.typie.bootstrap

import co.typie.di.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.time.Instant

class BootstrapModelsTest {
  @Test
  fun `resolveBootstrapState returns maintenance when enabled for current platform`() {
    val state = resolveBootstrapState(
      bootstrap = sampleBootstrap(
        maintenance = BootstrapMaintenancePayload(
          enabled = true,
          title = "점검 중",
          message = "잠시 후 다시 시도해주세요.",
          until = "2099-04-03T00:00:00Z",
          platforms = listOf("android"),
        ),
      ),
      platform = Platform.Android,
      currentVersion = "1.0.0",
    )

    assertEquals(
      BootstrapState.Maintenance(
        title = "점검 중",
        message = "잠시 후 다시 시도해주세요.",
        until = Instant.parse("2099-04-03T00:00:00Z"),
      ),
      state,
    )
  }

  @Test
  fun `resolveBootstrapState returns update required when version is lower`() {
    val state = resolveBootstrapState(
      bootstrap = sampleBootstrap(
        minVersion = BootstrapMinVersionPayload(
          ios = BootstrapPlatformMinVersionPayload(
            version = "1.0.0",
            storeUrl = "https://apps.apple.com/app/id6745595771",
          ),
          android = BootstrapPlatformMinVersionPayload(
            version = "1.2.0",
            storeUrl = "https://play.google.com/store/apps/details?id=co.typie",
          ),
        ),
      ),
      platform = Platform.Android,
      currentVersion = "1.1.9 (33)",
    )

    assertEquals(
      BootstrapState.UpdateRequired(
        storeUrl = "https://play.google.com/store/apps/details?id=co.typie",
        currentVersion = "1.1.9",
        requiredVersion = "1.2.0",
      ),
      state,
    )
  }

  @Test
  fun `resolveBootstrapState returns ready for unsupported desktop platform`() {
    val state = resolveBootstrapState(
      bootstrap = sampleBootstrap(),
      platform = Platform.Desktop,
      currentVersion = "1.0.0",
    )

    assertEquals(BootstrapState.Ready, state)
  }

  private fun sampleBootstrap(
    maintenance: BootstrapMaintenancePayload = BootstrapMaintenancePayload(
      enabled = false,
      title = "점검 중",
      message = "잠시 후 다시 시도해주세요.",
      until = null,
      platforms = emptyList(),
    ),
    minVersion: BootstrapMinVersionPayload = BootstrapMinVersionPayload(
      ios = BootstrapPlatformMinVersionPayload(
        version = "1.0.0",
        storeUrl = "https://apps.apple.com/app/id6745595771",
      ),
      android = BootstrapPlatformMinVersionPayload(
        version = "1.0.0",
        storeUrl = "https://play.google.com/store/apps/details?id=co.typie",
      ),
    ),
  ): BootstrapPayload {
    return BootstrapPayload(
      version = 1,
      updatedAt = "2099-04-03T00:00:00Z",
      maintenance = maintenance,
      minVersion = minVersion,
    )
  }
}
