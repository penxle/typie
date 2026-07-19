package co.typie.screen.editor.editor

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.editor.sync.ChangesetDeltaStore
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.safeDrawingHorizontalPadding
import co.typie.ui.component.Text
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect

@Composable
fun BoxScope.EditorSubscriptionBanner(
  documentId: String?,
  hazeState: HazeState,
  backdropColor: Color,
) {
  if (SubscriptionService.entitlement !is Entitlement.Expired) return

  var hasLocalStash by remember(documentId) { mutableStateOf(false) }
  LaunchedEffect(documentId) {
    hasLocalStash = documentId != null && ChangesetDeltaStore.load(documentId).isNotEmpty()
  }

  val surface =
    when (AppTheme.themeMode) {
      ResolvedThemeMode.Light -> AppColor.light.gray.s600
      ResolvedThemeMode.Dark -> AppColor.dark.gray.s500
    }

  Column(
    modifier =
      Modifier.align(Alignment.BottomCenter)
        .navigationBarsPadding()
        .safeDrawingHorizontalPadding()
        .padding(horizontal = 16.dp)
        .padding(bottom = 12.dp)
        .fillMaxWidth()
        .clip(AppShapes.rounded(AppShapes.lg))
        .hazeEffect(hazeState) {
          blurEffect {
            backgroundColor = backdropColor
            blurRadius = 20.dp
          }
        }
        .background(surface.copy(alpha = .5f))
        .clickable { SubscriptionService.requestSubscribeSheet(GatedAction.Generic) }
        .padding(horizontal = 24.dp, vertical = 16.dp),
    verticalArrangement = Arrangement.spacedBy(3.dp),
  ) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.SpaceBetween,
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(
        text = "구독이 만료되어 읽기 전용이에요",
        style = AppTheme.typography.caption,
        color = AppColor.white,
      )
      Text(
        text = "구독하기",
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.SemiBold),
        color = AppColor.white,
      )
    }
    if (hasLocalStash) {
      Text(
        text = "이 기기의 변경사항은 구독을 재개하면 자동 저장돼요",
        style = AppTheme.typography.micro,
        color = AppColor.white.copy(alpha = .7f),
      )
    }
  }
}
