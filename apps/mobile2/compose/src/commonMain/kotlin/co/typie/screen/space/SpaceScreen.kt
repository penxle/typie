package co.typie.screen.space

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.safeBottomPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.SpaceScreen_Query
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.EntityListCard
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.formatSpaceSummary
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun SpaceScreen() {
  val nav = Nav.current
  val model = koinViewModel<SpaceViewModel>()
  val scrollState = rememberScrollState("space")
  val bottomBarState = LocalBottomBarState.current

  LaunchedEffect(Unit) {
    bottomBarState.visible = true
  }

  val site = (model.query.state as? QueryState.Success)?.data?.site
  val items = site?.entities?.mapNotNull { it.toListItem() }.orEmpty()

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = {
      Text(
        site?.name ?: "스페이스",
        style = AppTheme.typography.title,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    contentPadding = PaddingValues(0.dp),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
      Column(
        modifier = Modifier
          .fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .safeBottomPadding(),
      ) {
        SpaceHeader(
          title = site?.name.orEmpty(),
          summary = formatSpaceSummary(
            folderCount = site?.folderCount ?: 0,
            documentCount = site?.documentCount ?: 0,
          ),
        )

        EntityListCard(
          items = items,
          emptyMessage = "문서와 폴더가 여기 나타나요",
          modifier = Modifier.padding(horizontal = 16.dp),
          onDocumentClick = { slug -> nav.navigate(Route.Editor(slug)) },
          onFolderClick = { entityId -> nav.navigate(Route.Folder(entityId)) },
        )

        Spacer(Modifier.height(140.dp))
      }
    },
  )
}

@Composable
private fun SpaceHeader(
  title: String,
  summary: String,
) {
  Column(
    modifier = Modifier
      .fillMaxWidth()
      .padding(horizontal = 16.dp)
      .padding(top = 4.dp, bottom = 24.dp),
  ) {
    Text(
      if (title.isBlank()) " " else title,
      style = AppTheme.typography.display,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    Spacer(Modifier.height(8.dp))

    Text(
      summary,
      style = AppTheme.typography.body,
      color = AppTheme.colors.textTertiary,
    )
  }
}

private fun SpaceScreen_Query.Entity.toListItem(): EntityListItem? {
  val folder = node.onFolder
  if (folder != null) {
    return EntityListItem.Folder(
      id = id,
      iconName = icon,
      iconColor = iconColor,
      name = folder.name,
      folderCount = folder.folderCount,
      documentCount = folder.documentCount,
    )
  }

  val document = node.onDocument
  if (document != null) {
    return EntityListItem.Document(
      id = id,
      iconName = icon,
      iconColor = iconColor,
      slug = slug,
      title = document.title,
      subtitle = document.subtitle,
      excerpt = document.excerpt,
      updatedAt = document.updatedAt,
    )
  }

  return null
}
