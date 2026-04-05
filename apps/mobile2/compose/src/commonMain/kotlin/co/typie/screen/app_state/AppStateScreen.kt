package co.typie.screen.app_state

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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import co.typie.datetime.format
import co.typie.ext.navigationBarsPadding
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
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme
import kotlin.time.Instant

private const val SUPPORT_URL = "https://penxle.channel.io/home"

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

@Composable
fun MaintenanceScreen(
  title: String,
  message: String,
  until: Instant?,
) {
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
    primaryAction = AppStateAction(
      label = "고객센터",
      variant = ButtonVariant.Secondary,
      leadingIcon = Lucide.Headphones,
      onClick = { uriHandler.openUri(SUPPORT_URL) },
    ),
  )
}

@Composable
fun UpdateRequiredScreen(
  storeUrl: String,
  currentVersion: String,
  requiredVersion: String,
) {
  val uriHandler = LocalUriHandler.current

  ProvideTopBar(enabled = false)

  AppStateScaffold(
    icon = Lucide.Download,
    title = "업데이트가 필요해요",
    message = "새로운 버전이 출시되었어요.\n스토어에서 업데이트를 진행해주세요.",
    detail = {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 14.dp, vertical = 12.dp),
          verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
          AppStateVersionRow(label = "현재 버전", value = currentVersion)
          AppStateVersionRow(label = "필요 버전", value = requiredVersion)
        }
      }
    },
    secondaryAction = AppStateAction(
      label = "고객센터",
      variant = ButtonVariant.Secondary,
      leadingIcon = Lucide.Headphones,
      onClick = { uriHandler.openUri(SUPPORT_URL) },
    ),
    primaryAction = AppStateAction(
      label = "업데이트하고 접속하기",
      onClick = { uriHandler.openUri(storeUrl) },
    ),
  )
}

private data class AppStateAction(
  val label: String,
  val variant: ButtonVariant = ButtonVariant.Primary,
  val leadingIcon: IconData? = null,
  val onClick: suspend () -> Unit,
)

@Composable
private fun AppStateScaffold(
  icon: IconData,
  title: String,
  message: String,
  detail: (@Composable () -> Unit)? = null,
  secondaryAction: AppStateAction? = null,
  primaryAction: AppStateAction? = null,
) {
  Screen(
    background = AppTheme.colors.surfaceDefault,
    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
    responsiveMaxWidth = 480.dp,
    body = { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .padding(contentPadding)
        .navigationBarsPadding(),
    ) {
      Column(
        modifier = Modifier
          .weight(1f)
          .fillMaxWidth(),
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
          modifier = Modifier
            .size(56.dp)
            .background(AppTheme.colors.surfaceSunken, CircleShape),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = icon,
            modifier = Modifier.size(24.dp),
            tint = AppTheme.colors.textTertiary,
          )
        }

        Spacer(Modifier.height(20.dp))

        Text(
          text = title,
          style = AppTheme.typography.heading.copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center),
          modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.height(8.dp))

        Text(
          text = message,
          style = AppTheme.typography.body.copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center),
          color = AppTheme.colors.textSecondary,
          modifier = Modifier.fillMaxWidth(),
        )

        if (detail != null) {
          Spacer(Modifier.height(16.dp))
          detail()
        }
      }

      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        if (secondaryAction != null) {
          AppStateButton(secondaryAction)
        }
        if (primaryAction != null) {
          AppStateButton(primaryAction)
        }
      }

      Spacer(Modifier.height(24.dp))
    }
  })
}

@Composable
private fun AppStateButton(
  action: AppStateAction,
) {
  Button(
    text = action.label,
    variant = action.variant,
    onClick = action.onClick,
    leading = action.leadingIcon?.let { icon ->
      { tint ->
        Icon(
          icon = icon,
          modifier = Modifier.size(16.dp),
          tint = tint,
        )
      }
    },
  )
}

@Composable
private fun AppStateBadge(
  text: String,
) {
  Box(
    modifier = Modifier
      .background(AppTheme.colors.surfaceSunken, RoundedCornerShape(999.dp))
      .padding(horizontal = 12.dp, vertical = 8.dp),
  ) {
    Text(
      text = text,
      style = AppTheme.typography.caption.copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center),
      color = AppTheme.colors.textTertiary,
    )
  }
}

@Composable
private fun AppStateVersionRow(
  label: String,
  value: String,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.SpaceBetween,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Text(
      text = label,
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )
    Text(
      text = value,
      style = AppTheme.typography.caption,
    )
  }
}
