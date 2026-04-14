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
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityIcon
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

  Screen(query = model.query) { contentPadding ->
    Box(Modifier.fillMaxSize()) {
      Column(Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding)) {
        Skeleton.Keep {
          Text("홈", style = AppTheme.typography.display)

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
  val folders = data.me.recentlyViewedEntities.mapNotNull { it.node.onFolder }

  Column {
    Skeleton.Keep {
      Text("최근 폴더", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
    }

    Spacer(Modifier.height(12.dp))

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
      for (folder in folders) {
        InteractionScope {
          Column(
            modifier =
              Modifier.width(140.dp)
                .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
                .pressScale()
                .clickable { nav.navigate(Route.Folder(folder.entity.id)) }
                .padding(16.dp)
          ) {
            EntityIcon(folder.entity.entityIcon_entity, modifier = Modifier.size(18.dp))

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

@Composable
private fun RecentDocuments(data: HomeScreen_Query.Data) {
  val nav = Nav.current
  val documents = data.me.recentlyViewedEntities.mapNotNull { it.node.onDocument }

  Column {
    Skeleton.Keep {
      Text("최근 문서", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
    }

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
        documents.separated(separator = { Divider(inset = 16.dp) }) { document ->
          InteractionScope {
            val parentFolder = document.entity.parent?.node?.onFolder

            Column(
              modifier =
                Modifier.fillMaxWidth()
                  .pressScale()
                  .clickable { nav.navigate(Route.Editor(document.entity.id)) }
                  .padding(horizontal = 16.dp, vertical = 12.dp)
            ) {
              if (parentFolder != null) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                  EntityIcon(parentFolder.entity.entityIcon_entity, modifier = Modifier.size(12.dp))

                  Spacer(Modifier.width(4.dp))

                  Text(
                    parentFolder.name,
                    style = AppTheme.typography.caption,
                    color = AppTheme.colors.textMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                }
              }

              Row(verticalAlignment = Alignment.CenterVertically) {
                EntityIcon(document.entity.entityIcon_entity, modifier = Modifier.size(18.dp))

                Spacer(Modifier.width(12.dp))

                Column(modifier = Modifier.weight(1f)) {
                  Row(verticalAlignment = Alignment.CenterVertically) {
                    val title = buildAnnotatedString {
                      append(document.title)

                      if (document.subtitle != null) {
                        pushStyle(SpanStyle(color = AppTheme.colors.textMuted))
                        append(" — ")
                        append(document.subtitle)
                        pop()
                      }
                    }

                    Text(
                      title,
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
                    document.excerpt.ifEmpty { "(내용 없음)" },
                    style = AppTheme.typography.caption,
                    color = AppTheme.colors.textMuted,
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
