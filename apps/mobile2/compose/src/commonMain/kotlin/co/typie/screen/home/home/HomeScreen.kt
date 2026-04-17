package co.typie.screen.home.home

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.EntityRow
import co.typie.domain.entity.document
import co.typie.domain.entity.folder
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entity.formatEntityExcerpt
import co.typie.domain.entity.formatFolderName
import co.typie.domain.entity.formatFolderRowSummary
import co.typie.domain.entity.parentFolderMeta
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.ext.truncate
import co.typie.ext.verticalScroll
import co.typie.graphql.HomeScreen_Query
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.shell.MainBottomBarActionButton
import co.typie.shell.MainBottomBarPill
import co.typie.ui.component.Divider
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
fun HomeScreen() {
  val model = viewModel { HomeViewModel() }

  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("홈", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  ProvideBottomBar(
    pill = { MainBottomBarPill() },
    action = {
      MainBottomBarActionButton(
        onClick = {
          if (model.isCreatingDocument) return@MainBottomBarActionButton
          model.createDocument().withDefaultExceptionHandler(toast).onOk {
            nav.navigate(Route.Editor(it))
          }
        }
      )
    },
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      Modifier.fillMaxSize()
        .verticalScroll(scrollState)
        .padding(contentPadding)
        .padding(bottom = BottomBarDefaults.BarAreaHeight)
        .padding(AppTheme.spacings.scrollBottomPadding)
    ) {
      Skeleton.Keep { Text("홈", style = AppTheme.typography.display) }

      Spacer(Modifier.height(16.dp))

      Skeleton.Bone(
        modifier = Modifier.fillMaxWidth().height(48.dp),
        shape = AppShapes.rounded(AppShapes.md),
      ) {
        SearchBar(
          placeholder = "${model.query.data.site.name.truncate(10)}에서 검색...",
          onClick = { nav.navigate(Route.Search) },
        )
      }

      Spacer(Modifier.height(20.dp))

      RecentFolders(data = model.query.data)

      Spacer(Modifier.height(20.dp))

      RecentDocuments(data = model.query.data)
    }

    ToastAnchor(
      modifier =
        Modifier.align(Alignment.BottomCenter)
          .navigationBarsPadding()
          .padding(bottom = BottomBarDefaults.BarAreaHeight)
    )
  }
}

@Composable
private fun SearchBar(placeholder: String, onClick: suspend () -> Unit) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier =
      Modifier.height(48.dp)
        .border(1.dp, AppTheme.colors.borderSubtle, AppShapes.rounded(AppShapes.md))
        .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
        .clickable(onClick = onClick)
        .padding(horizontal = 16.dp),
  ) {
    Icon(icon = Lucide.Search, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textMuted)

    Spacer(Modifier.width(12.dp))

    Text(
      placeholder,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.body,
      color = AppTheme.colors.textMuted,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun RecentFolders(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val folders =
    data.me.recentlyViewedEntities.mapNotNull { it.node.onFolder?.homeRecentFolder_folder }

  Column {
    Skeleton.Keep { SectionTitle("최근 폴더") }

    Spacer(Modifier.height(16.dp))

    if (folders.isEmpty()) {
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .height(110.dp)
            .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md)),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "최근 사용한 폴더가 여기 나타나요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      for (recentFolder in folders) {
        val entity = recentFolder.entity.entityRow_entity
        val folder = entity.folder ?: continue

        InteractionScope {
          Column(
            modifier =
              Modifier.width(140.dp)
                .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
                .pressScale()
                .clickable { nav.navigate(Route.Folder(entity.id)) }
                .padding(16.dp)
          ) {
            EntityIcon(entity = entity.entityIcon_entity, modifier = Modifier.size(18.dp))

            Spacer(Modifier.height(6.dp))

            Text(
              formatFolderName(folder.name),
              style = AppTheme.typography.label,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )

            Spacer(Modifier.height(2.dp))

            Text(
              formatFolderRowSummary(folderCount = 0, documentCount = folder.documentCount),
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
            )
          }
        }
      }
    }
  }
}

@Composable
private fun RecentDocuments(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val documents =
    data.me.recentlyViewedEntities.mapNotNull { it.node.onDocument?.homeRecentDocument_document }

  Column {
    Skeleton.Keep { SectionTitle("최근 문서") }

    Spacer(Modifier.height(16.dp))

    if (documents.isEmpty()) {
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .height(110.dp)
            .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md)),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "최근 문서가 여기 나타나요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      Column(
        modifier =
          Modifier.background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
      ) {
        documents.separated(separator = { Divider(inset = 16.dp) }) { recentDocument ->
          val entity = recentDocument.entity.entityRow_entity
          val document = entity.document ?: return@separated
          val parentFolder = recentDocument.entity.entityRowParent_entity.parentFolderMeta()

          EntityRow(entity = entity, onClick = { nav.navigate(Route.Editor(entity.id)) }) {
            parentMeta(parentFolder)
            title(
              title = formatDocumentTitle(document.title),
              subtitle = document.subtitle,
              trailingText = document.updatedAt.timeAgo(),
            )
            supporting(formatEntityExcerpt(document.excerpt))
          }
        }
      }
    }
  }
}
