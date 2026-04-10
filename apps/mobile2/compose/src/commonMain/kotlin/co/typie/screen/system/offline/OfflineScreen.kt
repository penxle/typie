package co.typie.screen.system.offline

import androidx.compose.runtime.Composable
import co.typie.icons.Lucide
import co.typie.screen.system.AppStateAction
import co.typie.screen.system.AppStateScaffold
import co.typie.ui.component.topbar.ProvideTopBar

@Composable
fun OfflineScreen(
  onRetry: suspend () -> Unit,
) {
  ProvideTopBar(enabled = false)

  AppStateScaffold(
    icon = Lucide.CloudOff,
    title = "앗! 문제가 발생했어요",
    message = "네트워크 연결을 확인하거나 잠시 후 다시 시도해주세요.",
    primaryAction = AppStateAction(
      label = "다시 시도",
      onClick = onRetry,
    ),
  )
}
