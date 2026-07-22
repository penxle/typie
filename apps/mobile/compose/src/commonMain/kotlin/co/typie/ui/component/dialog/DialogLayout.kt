package co.typie.ui.component.dialog

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.verticalScroll
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
internal fun DialogLayout(
  title: String,
  message: String,
  icon: (@Composable () -> Unit)? = null,
  actions: @Composable RowScope.() -> Unit,
) {
  DialogLayout(
    header = {
      Column(
        modifier = Modifier.padding(start = 28.dp, end = 28.dp, top = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        if (icon != null) {
          icon()
          Spacer(Modifier.height(16.dp))
        }
        Text(title, style = AppTheme.typography.title)
      }
    },
    body = {
      Text(
        message,
        modifier = Modifier.padding(start = 28.dp, end = 28.dp, top = 6.dp, bottom = 28.dp),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textMuted,
      )
    },
    actions = actions,
  )
}

@Composable
internal fun DialogLayout(
  header: @Composable () -> Unit,
  body: @Composable ColumnScope.() -> Unit,
  actions: @Composable RowScope.() -> Unit,
) {
  val bodyScrollState = rememberScrollState()

  Column(
    modifier =
      Modifier.clip(AppShapes.rounded(AppShapes.lg)).background(AppTheme.colors.surfaceDefault),
    horizontalAlignment = Alignment.CenterHorizontally,
  ) {
    header()

    Column(
      modifier = Modifier.fillMaxWidth().weight(1f, fill = false).verticalScroll(bodyScrollState),
      horizontalAlignment = Alignment.CenterHorizontally,
      content = body,
    )

    Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))

    Row(
      modifier = Modifier.fillMaxWidth(),
      verticalAlignment = Alignment.CenterVertically,
      content = actions,
    )
  }
}

@Composable
context(rowScope: RowScope)
internal fun DialogActionButton(
  text: String,
  color: Color = AppTheme.colors.textDefault,
  onClick: () -> Unit,
) {
  Box(
    modifier = with(rowScope) { Modifier.weight(1f) }.clickable(onClick).padding(vertical = 14.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text = text, style = AppTheme.typography.action, color = color)
  }
}

@Composable
internal fun DialogActionDivider() {
  Box(modifier = Modifier.width(1.dp).height(48.dp).background(AppTheme.colors.borderHairline))
}
