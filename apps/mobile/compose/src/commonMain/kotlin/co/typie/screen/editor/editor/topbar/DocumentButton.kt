package co.typie.screen.editor.editor.topbar

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.UNTITLED_DOCUMENT_TEXT
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.fragment.EntityIcon_entity
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

@Composable
internal fun EditorDocumentButton(
  entityIcon: EntityIcon_entity,
  title: String,
  subtitle: String?,
  loading: Boolean,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
) {
  val resolvedTitle = if (loading) "" else title.ifBlank { UNTITLED_DOCUMENT_TEXT }
  val resolvedSubtitle = if (loading) null else subtitle?.takeIf(String::isNotBlank)

  InteractionScope {
    Skeleton.Passive(enabled = loading) {
      Box(
        modifier =
          modifier
            .fillMaxWidth()
            .height(TopBarDefaults.TitleHeight)
            .clickable(enabled = !loading, onClick = onClick)
            .padding(horizontal = 12.dp),
        contentAlignment = Alignment.CenterStart,
      ) {
        Row(
          modifier = Modifier.fillMaxWidth().pressScale(),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          EntityIcon(entity = entityIcon, modifier = Modifier.size(21.dp))

          Spacer(Modifier.width(12.dp))

          Column(
            modifier = Modifier.weight(1f, fill = false),
            verticalArrangement =
              if (resolvedSubtitle == null) Arrangement.Center else Arrangement.spacedBy(2.dp),
          ) {
            Text(
              text = resolvedTitle,
              style = AppTheme.typography.title.copy(fontSize = 16.sp),
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )

            if (resolvedSubtitle != null) {
              Text(
                text = resolvedSubtitle,
                style = AppTheme.typography.caption.copy(fontSize = 13.sp),
                color = AppTheme.colors.textMuted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }
          }

          Spacer(Modifier.width(6.dp))

          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(17.dp),
            tint = AppTheme.colors.textMuted,
          )
        }
      }
    }
  }
}
