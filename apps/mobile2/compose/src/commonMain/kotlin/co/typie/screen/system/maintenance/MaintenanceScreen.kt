package co.typie.screen.system.maintenance

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalUriHandler
import co.typie.datetime.format
import co.typie.icons.Lucide
import co.typie.screen.system.AppStateAction
import co.typie.screen.system.AppStateBadge
import co.typie.screen.system.AppStateScaffold
import co.typie.screen.system.SUPPORT_URL
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.topbar.ProvideTopBar
import kotlin.time.Instant

@Composable
fun MaintenanceScreen(title: String, message: String, until: Instant?) {
  val uriHandler = LocalUriHandler.current

  ProvideTopBar(enabled = false)

  AppStateScaffold(
    icon = Lucide.Wrench,
    title = title,
    message = message.replace("\\n", "\n"),
    detail = {
      if (until != null) {
        AppStateBadge("예상 종료: ${until.format("M월 d일 HH시 mm분")}")
      }
    },
    primaryAction =
      AppStateAction(
        label = "고객센터",
        variant = ButtonVariant.Secondary,
        leadingIcon = Lucide.Headphones,
        onClick = { uriHandler.openUri(SUPPORT_URL) },
      ),
  )
}
