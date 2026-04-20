package co.typie.screen.editor.editor

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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.UNTITLED_DOCUMENT_TEXT
import co.typie.editor.LocalEditorState
import co.typie.editor.compose.EditorView
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.EntityIcon_entity
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

@Composable
fun EditorScreen(entityId: String) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val model = viewModel { EditorViewModel() }
  val loading = model.query.state !is QueryState.Success
  val entity = model.query.data.entity
  val document = entity.node.onDocument

  LaunchedEffect(entityId) { model.entityId = entityId }
  LaunchedEffect(loading, document) {
    if (!loading && document == null) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = {
      document?.let { document ->
        Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
          EditorTopBarDocumentButton(
            entityIcon = entity.entityIcon_entity,
            title = document.title.ifBlank { UNTITLED_DOCUMENT_TEXT },
            subtitle = document.subtitle,
            loading = loading,
            onClick = { nav.navigate(Route.Document(entityId)) },
            modifier = Modifier.fillMaxWidth().widthIn(max = ResponsiveContainerDefaults.MaxWidth),
          )
        }
      }
    },
  )

  Screen(loadable = model.query, background = Color.White) { contentPadding ->
    document ?: return@Screen

    CompositionLocalProvider(LocalEditorState provides model.editorState) {
      Box(Modifier.fillMaxSize().padding(contentPadding)) {
        EditorView(doc = model.doc, selection = model.selection)
      }
    }
  }
}

@Composable
private fun EditorTopBarDocumentButton(
  entityIcon: EntityIcon_entity,
  title: String,
  subtitle: String?,
  loading: Boolean,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
) {
  val resolvedSubtitle = subtitle?.takeIf(String::isNotBlank)

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
              text = title,
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
