package co.typie.screen.system

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.ext.safeBottomPadding
import co.typie.generated.resources.Res
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

internal const val SUPPORT_URL = "https://penxle.channel.io/home"

internal data class AppStateAction(
  val label: String,
  val variant: ButtonVariant = ButtonVariant.Primary,
  val leadingIcon: IconData? = null,
  val onClick: suspend () -> Unit,
)

@Composable
internal fun AppStateScaffold(
  icon: IconData,
  title: String,
  message: String,
  detail: (@Composable () -> Unit)? = null,
  secondaryAction: AppStateAction? = null,
  primaryAction: AppStateAction? = null,
) {
  Screen(
    background = AppTheme.colors.surfaceDefault,
    contentPadding = PaddingValues(horizontal = 20.dp),
  ) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize().padding(contentPadding).widthIn(max = 480.dp).safeBottomPadding()
    ) {
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
          modifier = Modifier.size(56.dp).background(AppTheme.colors.surfaceSunken, CircleShape),
          contentAlignment = Alignment.Center,
        ) {
          Icon(icon = icon, modifier = Modifier.size(24.dp), tint = AppTheme.colors.textTertiary)
        }

        Spacer(Modifier.height(20.dp))

        Text(
          text = title,
          style = AppTheme.typography.heading.copy(textAlign = TextAlign.Center),
          modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.height(8.dp))

        Text(
          text = message,
          style = AppTheme.typography.body.copy(textAlign = TextAlign.Center),
          color = AppTheme.colors.textSecondary,
          modifier = Modifier.fillMaxWidth(),
        )

        if (detail != null) {
          Spacer(Modifier.height(16.dp))
          detail()
        }
      }

      Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (secondaryAction != null) {
          AppStateButton(secondaryAction)
        }
        if (primaryAction != null) {
          AppStateButton(primaryAction)
        }
      }

      Spacer(Modifier.height(24.dp))
    }
  }
}

@Composable
internal fun AppStateButton(action: AppStateAction) {
  Button(
    text = action.label,
    variant = action.variant,
    onClick = action.onClick,
    leading =
      action.leadingIcon?.let { icon ->
        { tint -> Icon(icon = icon, modifier = Modifier.size(16.dp), tint = tint) }
      },
  )
}

@Composable
internal fun AppStateBadge(text: String) {
  Box(
    modifier =
      Modifier.background(AppTheme.colors.surfaceSunken, RoundedCornerShape(999.dp))
        .padding(horizontal = 12.dp, vertical = 8.dp)
  ) {
    Text(
      text = text,
      style = AppTheme.typography.caption.copy(textAlign = TextAlign.Center),
      color = AppTheme.colors.textTertiary,
    )
  }
}

@Composable
internal fun AppStateVersionRow(label: String, value: String) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.SpaceBetween,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Text(text = label, style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
    Text(text = value, style = AppTheme.typography.caption)
  }
}
