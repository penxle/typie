package co.typie.screen.system.maintenance

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.datetime.format
import co.typie.domain.preflight.PreflightService
import co.typie.domain.preflight.PreflightState
import co.typie.generated.resources.Res
import co.typie.icons.Lucide
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private const val SUPPORT_URL = "https://penxle.channel.io/home"

@Composable
fun MaintenanceScreen() {
  val state = PreflightService.state
  if (state !is PreflightState.UnderMaintenance) return

  val uriHandler = LocalUriHandler.current

  ProvideTopBar(enabled = false)

  Screen(background = AppTheme.colors.surfaceDefault) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().padding(contentPadding)) {
      Column(
        modifier = Modifier.weight(1f).fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
      ) {
        Img(
          url = Res.getUri("files/logos/full.svg"),
          modifier = Modifier.height(32.dp),
          contentScale = ContentScale.FillHeight,
          color = AppTheme.colors.textDefault,
        )

        Spacer(Modifier.height(28.dp))

        Box(
          modifier =
            Modifier.size(56.dp).background(AppTheme.colors.surfaceInset, AppShapes.circle),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = Lucide.Construction,
            modifier = Modifier.size(24.dp),
            tint = AppTheme.colors.textMuted,
          )
        }

        Spacer(Modifier.height(20.dp))

        Text(
          text = state.title,
          style = AppTheme.typography.heading,
          textAlign = TextAlign.Center,
          modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.height(8.dp))

        Text(
          text = state.message.replace("\\n", "\n"),
          style = AppTheme.typography.body,
          color = AppTheme.colors.textMuted,
          textAlign = TextAlign.Center,
          modifier = Modifier.fillMaxWidth(),
        )

        if (state.until != null) {
          Spacer(Modifier.height(16.dp))

          Box(
            modifier =
              Modifier.background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.full))
                .padding(horizontal = 12.dp, vertical = 8.dp)
          ) {
            Text(
              text = "예상 종료: ${state.until.format("M월 d일 HH시 mm분")}",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
              textAlign = TextAlign.Center,
            )
          }
        }
      }

      Button(
        text = "고객센터",
        variant = ButtonVariant.Secondary,
        onClick = { uriHandler.openUri(SUPPORT_URL) },
        leading = { color ->
          Icon(icon = Lucide.Headphones, modifier = Modifier.size(16.dp), tint = color)
        },
      )

      Spacer(Modifier.height(24.dp))
    }
  }
}
