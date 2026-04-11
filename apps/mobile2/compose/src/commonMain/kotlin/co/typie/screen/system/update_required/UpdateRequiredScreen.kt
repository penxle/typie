package co.typie.screen.system.update_required

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.preflight.PreflightService
import co.typie.preflight.PreflightState
import co.typie.screen.system.AppStateAction
import co.typie.screen.system.AppStateScaffold
import co.typie.screen.system.AppStateVersionRow
import co.typie.screen.system.SUPPORT_URL
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardSurface
import co.typie.ui.component.topbar.ProvideTopBar

@Composable
fun UpdateRequiredScreen() {
  val state = PreflightService.state
  if (state !is PreflightState.UpdateRequired) return

  val uriHandler = LocalUriHandler.current

  ProvideTopBar(enabled = false)

  AppStateScaffold(
    icon = Lucide.Download,
    title = "업데이트가 필요해요",
    message = "새로운 버전이 출시되었어요.\n스토어에서 업데이트를 진행해주세요.",
    detail = {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 12.dp),
          verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
          AppStateVersionRow(label = "현재 버전", value = state.currentVersion)
          AppStateVersionRow(label = "필요 버전", value = state.requiredVersion)
        }
      }
    },
    secondaryAction =
      AppStateAction(
        label = "고객센터",
        variant = ButtonVariant.Secondary,
        leadingIcon = Lucide.Headphones,
        onClick = { uriHandler.openUri(SUPPORT_URL) },
      ),
    primaryAction =
      AppStateAction(label = "업데이트하고 접속하기", onClick = { uriHandler.openUri(state.storeUrl) }),
  )
}
