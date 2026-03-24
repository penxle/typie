package co.typie.screen.home

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.InlineTextContent
import androidx.compose.foundation.text.appendInlineContent
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.Placeholder
import androidx.compose.ui.text.PlaceholderVerticalAlign
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.datetime.timeAgo
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.navigation.PlatformBackHandler
import co.typie.route.Route
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.EntityPreview
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.viewmodel.koinViewModel
import kotlin.math.roundToInt

@Composable
fun HomeScreen() {
  val model = koinViewModel<HomeViewModel>()
  val searchModel = koinViewModel<SearchViewModel>()
  val scrollState = rememberScrollState()
  val bottomBarState = LocalBottomBarState.current

  LaunchedEffect(model.searching) {
    bottomBarState.visible = !model.searching
  }

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("홈", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
    custom = if (model.searching) {
      {
        SearchTopBar(
          query = searchModel.query,
          onQueryChange = { searchModel.updateQuery(it) },
          onSubmit = { searchModel.submitQuery() },
          onCancel = {
            model.searching = false
          },
        )
      }
    } else null,
    customKey = SearchTopBarKey,
  )

  PlatformBackHandler(enabled = model.searching) {
    model.searching = false
  }

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    contentPadding = PaddingValues(0.dp)
  ) { contentPadding ->
    Crossfade(targetState = model.searching) { isSearching ->
      if (isSearching) {
        SearchContent(
          searchViewModel = searchModel,
          contentPadding = contentPadding,
        )
      } else {
        Column(
          Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .navigationBarsPadding()
        ) {
          Skeleton.Keep {
            Text(
              "홈",
              style = AppTheme.typography.display,
              modifier = Modifier.padding(horizontal = 16.dp)
            )

            SearchBar(onClick = {
              searchModel.reset()
              model.searching = true
            })
          }

          RecentDocuments(data = model.query.data)

          RecentFolders(data = model.query.data)

          MoreDocuments(data = model.query.data)

          Spacer(Modifier.height(140.dp))
        }
      }
    }
  }
}

@Composable
private fun SearchBar(onClick: () -> Unit) {
  Box(
    modifier = Modifier
      .padding(horizontal = 16.dp)
      .padding(top = 12.dp, bottom = 4.dp)
      .fillMaxWidth()
      .height(44.dp)
      .clip(RoundedCornerShape(10.dp))
      .background(AppTheme.colors.surfaceDefault)
      .clickable(onClick = onClick)
      .padding(horizontal = 14.dp),
    contentAlignment = Alignment.CenterStart,
  ) {
    Row(verticalAlignment = Alignment.CenterVertically) {
      Icon(
        icon = Lucide.Search,
        modifier = Modifier.size(16.dp),
        tint = AppTheme.colors.textMuted,
      )

      Spacer(Modifier.width(10.dp))

      Text(
        "문서 검색...",
        style = AppTheme.typography.action,
        color = AppTheme.colors.textMuted,
      )
    }
  }
}

@Composable
private fun RecentDocuments(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val documents = data.me.recentlyViewedEntities.mapNotNull { it.node.onDocument }.take(5)

  Column {
    Skeleton.Keep {
      Text(
        "자주 찾은 글",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(horizontal = 16.dp).padding(top = 24.dp, bottom = 12.dp),
      )
    }

    if (documents.isEmpty()) {
      Box(
        modifier = Modifier
          .padding(horizontal = 16.dp)
          .fillMaxWidth()
          .height(110.dp)
          .clip(RoundedCornerShape(12.dp))
          .background(AppTheme.colors.surfaceDefault),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "자주 찾은 글이 여기 나타나요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      val scrollState = rememberScrollState("recent-documents")

      Row(
        modifier = Modifier.horizontalScroll(scrollState).padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        for (document in documents) {
          InteractionScope {
            Row(
              modifier = Modifier
                .width(139.dp)
                .clickable { nav.navigate(Route.Editor(document.entity.id)) }
                .pressScale(),
              horizontalArrangement = Arrangement.spacedBy(12.dp),
              verticalAlignment = Alignment.CenterVertically,
            ) {
              val shadowColor = AppTheme.colors.shadowAmbient

              EntityPreview(
                url = document.previewUrl,
                modifier = Modifier
                  .fillMaxWidth()
                  .dropShadow(RoundedCornerShape(12.dp)) {
                    color = shadowColor
                    radius = 16f
                    spread = 8f
                  }
                  .clip(RoundedCornerShape(12.dp))
                  .border(1.dp, AppTheme.colors.borderSubtle, RoundedCornerShape(12.dp)),
                placeholderColor = AppTheme.colors.surfaceDefault
              )
            }
          }
        }
      }
    }
  }
}

@Composable
private fun RecentFolders(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val folders = data.me.recentlyViewedEntities.mapNotNull { it.node.onFolder }

  Column {
    Skeleton.Keep {
      Text(
        "최근 폴더",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(horizontal = 16.dp).padding(top = 20.dp, bottom = 12.dp),
      )
    }

    if (folders.isEmpty()) {
      Box(
        modifier = Modifier
          .padding(horizontal = 16.dp)
          .fillMaxWidth()
          .height(110.dp)
          .clip(RoundedCornerShape(12.dp))
          .background(AppTheme.colors.surfaceDefault),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "최근 사용한 폴더가 여기 나타나요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      val scrollState = rememberScrollState("recent-folders")

      Row(
        modifier = Modifier.horizontalScroll(scrollState).padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        for (folder in folders) {
          InteractionScope {
            Column(
              modifier = Modifier
                .width(140.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(AppTheme.colors.surfaceDefault)
                .clickable { nav.navigate(Route.Folder(folder.entity.id)) }
                .pressScale()
                .padding(16.dp),
            ) {
              Icon(
                icon = Lucide.Folder,
                modifier = Modifier.size(18.dp),
                tint = AppTheme.colors.brand,
              )

              Spacer(Modifier.height(6.dp))

              Text(
                folder.name,
                style = AppTheme.typography.label,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )

              Spacer(Modifier.height(2.dp))

              Text(
                "문서 ${folder.documentCount}개",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textMuted,
              )
            }
          }
        }
      }
    }
  }
}

@Composable
private fun MoreDocuments(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val documents = data.me.recentlyViewedEntities.mapNotNull { it.node.onDocument }.drop(5)

  Column {
    Skeleton.Keep {
      Text(
        "더 많은 최근 문서",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(horizontal = 16.dp).padding(top = 24.dp, bottom = 12.dp),
      )
    }

    if (documents.isEmpty()) {
      Box(
        modifier = Modifier
          .padding(horizontal = 16.dp)
          .fillMaxWidth()
          .height(110.dp)
          .clip(RoundedCornerShape(12.dp))
          .background(AppTheme.colors.surfaceDefault),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "최근 사용한 문서가 여기 나타나요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      Column(
        modifier = Modifier
          .padding(horizontal = 16.dp)
          .clip(RoundedCornerShape(12.dp))
          .background(AppTheme.colors.surfaceDefault),
      ) {
        documents.separated(
          separator = {
            Box(
              Modifier
                .fillMaxWidth()
                .height(1.dp)
                .padding(horizontal = 16.dp)
                .background(AppTheme.colors.borderSubtle)
            )
          },
        ) { document ->
          InteractionScope {
            Row(
              modifier = Modifier
                .fillMaxWidth()
                .clickable { nav.navigate(Route.Editor(document.entity.slug)) }
                .pressScale()
                .padding(horizontal = 16.dp, vertical = 12.dp),
              verticalAlignment = Alignment.CenterVertically,
            ) {
              val shadowColor = AppTheme.colors.shadowAmbient
              val density = LocalDensity.current

              Img(
                url = "${document.previewUrl}&w=${(35 * density.density).roundToInt()}&theme=${AppTheme.themeMode.name.lowercase()}",
                modifier = Modifier
                  .width(35.dp)
                  .height(49.dp)
                  .dropShadow(RoundedCornerShape(2.dp)) {
                    color = shadowColor
                    radius = 8f
                    spread = 4f
                  }
                  .clip(RoundedCornerShape(2.dp))
                  .border(1.dp, AppTheme.colors.borderSubtle, RoundedCornerShape(2.dp)),
                placeholderColor = AppTheme.colors.surfaceSunken
              )

              Spacer(Modifier.width(12.dp))

              Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                  Text(
                    document.title,
                    style = AppTheme.typography.label,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                  )

                  Spacer(Modifier.width(8.dp))

                  Text(
                    document.updatedAt.timeAgo(),
                    style = AppTheme.typography.caption,
                    color = AppTheme.colors.textMuted,
                  )
                }

                val folderName = document.entity.parent?.node?.onFolder?.name

                Spacer(Modifier.height(4.dp))

                val text = buildAnnotatedString {
                  if (folderName != null) {
                    appendInlineContent("folder")
                    append(" $folderName")

                    if (document.excerpt.isNotEmpty()) {
                      appendInlineContent("dot")
                    }
                  }

                  if (document.excerpt.isNotEmpty()) {
                    append(document.excerpt)
                  }
                }

                val color = AppTheme.colors.textMuted
                val iconSize = AppTheme.typography.caption.fontSize

                Text(
                  text = text,
                  style = AppTheme.typography.caption,
                  color = color,
                  maxLines = 1,
                  overflow = TextOverflow.Ellipsis,
                  inlineContent = mapOf(
                    "folder" to InlineTextContent(
                      Placeholder(iconSize, iconSize, PlaceholderVerticalAlign.TextCenter),
                    ) {
                      Icon(
                        icon = Lucide.Folder,
                        modifier = Modifier.fillMaxSize(),
                        tint = color,
                      )
                    },
                    "dot" to InlineTextContent(
                      Placeholder(iconSize, iconSize, PlaceholderVerticalAlign.TextCenter),
                    ) {
                      Icon(
                        icon = Lucide.Dot,
                        modifier = Modifier.fillMaxSize(),
                        tint = color,
                      )
                    },
                  ),
                )
              }
            }
          }
        }
      }
    }
  }
}
