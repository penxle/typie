package co.typie.screen.home.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.ext.pressScale
import co.typie.ext.safeBottomPadding
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.screen.space.entity.EntityCreateViewModel
import co.typie.shell.MainBottomBarActionButton
import co.typie.shell.MainBottomBarPill
import co.typie.storage.Preference.siteId
import co.typie.ui.component.ResponsiveContainer
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.resolveResponsiveContainerMetrics
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun HomeScreen() {
  val model = viewModel { HomeViewModel() }
  val createActionModel = viewModel(key = "home-create-actions") { EntityCreateViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val dialog = LocalDialog.current
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
          if (createActionModel.isCreating) return@MainBottomBarActionButton
          val resolvedSiteId = siteId ?: return@MainBottomBarActionButton
          scope.launch {
            createActionModel
              .createDocument(siteId = resolvedSiteId)
              .withDefaultExceptionHandler(toast)
              .onOk { createdSlug ->
                model.refetch()
                nav.navigate(Route.Editor(createdSlug))
              }
          }
        }
      )
    },
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success) { contentPadding ->
    Column(
      Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding).safeBottomPadding()
    ) {
      HomeFramedSection {
        Skeleton.Keep {
          Text(
            "홈",
            style = AppTheme.typography.display,
            modifier = Modifier.padding(horizontal = 16.dp),
          )

          SearchBar(
            placeholder = resolveHomeSearchPlaceholder(model.query.data.site.name),
            onClick = { nav.navigate(Route.HomeSearch) },
          )
        }
      }

      RecentFolders(data = model.query.data)

      HomeFramedSection { RecentDocuments(data = model.query.data) }

      Spacer(Modifier.height(140.dp))
    }
  }
}

@Composable
private fun HomeFramedSection(content: @Composable ColumnScope.() -> Unit) {
  ResponsiveContainer(modifier = Modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth()) { content() }
  }
}

@Composable
private fun HomeFullBleedRail(
  scrollState: androidx.compose.foundation.ScrollState,
  itemSpacing: Dp = 16.dp,
  content: @Composable () -> Unit,
) {
  BoxWithConstraints(Modifier.fillMaxWidth()) {
    val metrics =
      resolveResponsiveContainerMetrics(
        screenWidth = maxWidth.value,
        maxWidth = ResponsiveContainerDefaults.MaxWidth.value,
        breakpoint = ResponsiveContainerDefaults.Breakpoint.value,
      )
    val edgePadding = metrics.gutterWidth.dp + 16.dp

    Row(
      modifier =
        Modifier.horizontalScroll(scrollState).padding(start = edgePadding, end = edgePadding),
      horizontalArrangement = Arrangement.spacedBy(itemSpacing),
    ) {
      content()
    }
  }
}

@Composable
private fun SearchBar(placeholder: String, onClick: suspend () -> Unit) {
  HomeSearchFieldFrame(
    modifier =
      Modifier.padding(horizontal = 16.dp).padding(top = 12.dp, bottom = 4.dp).fillMaxWidth(),
    onClick = onClick,
  ) {
    Icon(
      icon = Lucide.Search,
      modifier = Modifier.size(HomeSearchFieldDefaults.IconSize),
      tint = AppTheme.colors.textMuted,
    )

    Spacer(Modifier.width(HomeSearchFieldDefaults.IconGap))

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
  val folders = data.me.recentlyViewedEntities.mapNotNull { it.node.onFolder }

  Column {
    HomeFramedSection {
      Skeleton.Keep {
        Text(
          "최근 폴더",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.padding(horizontal = 16.dp).padding(top = 20.dp, bottom = 12.dp),
        )
      }
    }

    if (folders.isEmpty()) {
      HomeFramedSection {
        Box(
          modifier =
            Modifier.padding(horizontal = 16.dp)
              .fillMaxWidth()
              .height(110.dp)
              .clip(AppShapes.rounded(AppShapes.md))
              .background(AppTheme.colors.surfaceDefault),
          contentAlignment = Alignment.Center,
        ) {
          Text(
            "최근 사용한 폴더가 여기 나타나요",
            style = AppTheme.typography.action,
            color = AppTheme.colors.textTertiary,
          )
        }
      }
    } else {
      val scrollState = rememberScrollState("recent-folders")

      HomeFullBleedRail(scrollState = scrollState) {
        for (folder in folders) {
          InteractionScope {
            val entityIcon =
              resolveEntityIconAppearance(
                iconName = folder.entity.icon,
                iconColor = folder.entity.iconColor,
                fallbackIcon = Lucide.Folder,
                fallbackTint = AppTheme.colors.brand,
                colors = AppTheme.colors,
              )

            Column(
              modifier =
                Modifier.width(140.dp)
                  .clip(AppShapes.rounded(AppShapes.md))
                  .background(AppTheme.colors.surfaceDefault)
                  .clickable { nav.navigate(Route.Folder(folder.entity.id)) }
                  .pressScale()
                  .padding(16.dp)
            ) {
              Icon(icon = entityIcon.icon, modifier = Modifier.size(18.dp), tint = entityIcon.tint)

              Spacer(Modifier.height(6.dp))

              Text(
                folder.name,
                style = AppTheme.typography.label,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )

              Spacer(Modifier.height(2.dp))

              Text(
                if (folder.documentCount == 0) "빈 폴더" else "문서 ${folder.documentCount}개",
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
private fun RecentDocuments(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val documents = data.me.recentlyViewedEntities.mapNotNull { it.node.onDocument }

  Column {
    Skeleton.Keep {
      Text(
        "최근 문서",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(horizontal = 16.dp).padding(top = 24.dp, bottom = 12.dp),
      )
    }

    if (documents.isEmpty()) {
      Box(
        modifier =
          Modifier.padding(horizontal = 16.dp)
            .fillMaxWidth()
            .height(110.dp)
            .clip(AppShapes.rounded(AppShapes.md))
            .background(AppTheme.colors.surfaceDefault),
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
          Modifier.padding(horizontal = 16.dp)
            .clip(AppShapes.rounded(AppShapes.md))
            .background(AppTheme.colors.surfaceDefault)
      ) {
        documents.separated(
          separator = {
            Box(
              Modifier.fillMaxWidth()
                .height(1.dp)
                .padding(horizontal = 16.dp)
                .background(AppTheme.colors.borderSubtle)
            )
          }
        ) { document ->
          InteractionScope {
            val parentFolder = document.entity.parent?.node?.onFolder
            val folderName = parentFolder?.name
            val metaColor = AppTheme.colors.textMuted
            val entityIcon =
              resolveEntityIconAppearance(
                iconName = document.entity.icon,
                iconColor = document.entity.iconColor,
                fallbackIcon = Lucide.File,
                fallbackTint = metaColor,
                colors = AppTheme.colors,
              )
            val folderIcon =
              resolveEntityIconAppearance(
                iconName = parentFolder?.entity?.icon,
                iconColor = parentFolder?.entity?.iconColor,
                fallbackIcon = Lucide.Folder,
                fallbackTint = metaColor,
                colors = AppTheme.colors,
              )

            Column(
              modifier =
                Modifier.fillMaxWidth()
                  .clickable { nav.navigate(Route.Editor(document.entity.slug)) }
                  .pressScale()
                  .padding(horizontal = 16.dp, vertical = 12.dp)
            ) {
              if (folderName != null) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                  Icon(
                    icon = folderIcon.icon,
                    modifier = Modifier.size(12.dp),
                    tint = folderIcon.tint,
                  )

                  Spacer(Modifier.width(4.dp))

                  Text(
                    folderName,
                    style = AppTheme.typography.caption,
                    color = metaColor,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                }

                Spacer(Modifier.height(4.dp))
              }

              Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                  icon = entityIcon.icon,
                  modifier = Modifier.size(18.dp),
                  tint = entityIcon.tint,
                )

                Spacer(Modifier.width(12.dp))

                Column(modifier = Modifier.weight(1f)) {
                  val subtitle = document.subtitle?.takeIf { it.isNotBlank() }

                  Row(verticalAlignment = Alignment.CenterVertically) {
                    val titleText = buildAnnotatedString {
                      append(document.title)

                      if (subtitle != null) {
                        pushStyle(SpanStyle(color = metaColor))
                        append(" — ")
                        append(subtitle)
                        pop()
                      }
                    }

                    Text(
                      titleText,
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

                  Spacer(Modifier.height(4.dp))

                  Text(
                    if (document.excerpt.isNotEmpty()) document.excerpt else "(내용 없음)",
                    style = AppTheme.typography.caption,
                    color = metaColor,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                }
              }
            }
          }
        }
      }
    }
  }
}
