package co.typie.screen.stats

import androidx.compose.runtime.Composable

enum class StatsImageSaveResult {
  Success,
  PermissionDenied,
  Error,
}

interface StatsImageExporter {
  suspend fun copyPng(
    bytes: ByteArray,
    suggestedName: String,
  ): Boolean

  suspend fun savePng(
    bytes: ByteArray,
    suggestedName: String,
  ): StatsImageSaveResult
}

@Composable
expect fun rememberStatsImageExporter(): StatsImageExporter
