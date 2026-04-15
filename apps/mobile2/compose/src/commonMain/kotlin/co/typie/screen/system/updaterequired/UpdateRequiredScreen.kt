package co.typie.screen.system.updaterequired

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import co.typie.domain.preflight.PreflightService
import co.typie.domain.preflight.PreflightState
import co.typie.generated.resources.Res
import co.typie.icons.Lucide
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private const val SUPPORT_URL = "https://penxle.channel.io/home"

@Composable
fun UpdateRequiredScreen() {
  val state = PreflightService.state
  if (state !is PreflightState.UpdateRequired) return

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
          color = AppTheme.colors.textPrimary,
        )

        Spacer(Modifier.height(28.dp))

        Box(
          modifier =
            Modifier.size(56.dp).background(AppTheme.colors.surfaceSunken, AppShapes.circle),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = Lucide.CircleFadingArrowUp,
            modifier = Modifier.size(24.dp),
            tint = AppTheme.colors.textTertiary,
          )
        }

        Spacer(Modifier.height(20.dp))

        Text(
          text = "업데이트가 필요해요",
          style = AppTheme.typography.heading,
          textAlign = TextAlign.Center,
          modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.height(8.dp))

        Text(
          text = "새로운 버전이 출시되었어요.\n스토어에서 업데이트를 진행해주세요.",
          style = AppTheme.typography.body,
          color = AppTheme.colors.textSecondary,
          textAlign = TextAlign.Center,
          modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.height(16.dp))

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
          ) {
            Row(
              modifier = Modifier.fillMaxWidth(),
              horizontalArrangement = Arrangement.SpaceBetween,
              verticalAlignment = Alignment.CenterVertically,
            ) {
              Text(
                text = "현재 버전",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )

              Text(text = state.currentVersion, style = AppTheme.typography.caption)
            }

            Row(
              modifier = Modifier.fillMaxWidth(),
              horizontalArrangement = Arrangement.SpaceBetween,
              verticalAlignment = Alignment.CenterVertically,
            ) {
              Text(
                text = "필요 버전",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )

              Text(text = state.requiredVersion, style = AppTheme.typography.caption)
            }
          }
        }
      }

      Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(
          text = "고객센터",
          variant = ButtonVariant.Secondary,
          onClick = { uriHandler.openUri(SUPPORT_URL) },
          leading = { tint ->
            Icon(icon = Lucide.Headphones, modifier = Modifier.size(16.dp), tint = tint)
          },
        )

        Button(text = "업데이트하고 접속하기", onClick = { uriHandler.openUri(state.storeUrl) })
      }

      Spacer(Modifier.height(24.dp))
    }
  }
}
