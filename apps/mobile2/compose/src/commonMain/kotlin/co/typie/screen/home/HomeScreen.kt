package co.typie.screen.home

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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
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
import co.typie.graphql.type.DocumentType
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.ErrorDialog
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

@Composable
fun HomeScreen() {
  val model = koinViewModel<HomeViewModel>()
  val scrollState = rememberScrollState()

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("홈", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceSubtle,
    contentPadding = PaddingValues(0.dp)
  ) { contentPadding ->
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

        SearchBar()
      }

      RecentFolders(data = model.query.data)

      RecentDocuments(data = model.query.data)

      Spacer(Modifier.height(140.dp))
    }
  }
}

@Composable
private fun SearchBar() {
  Box(
    modifier = Modifier
      .padding(horizontal = 16.dp)
      .padding(top = 12.dp, bottom = 4.dp)
      .fillMaxWidth()
      .height(44.dp)
      .clip(RoundedCornerShape(10.dp))
      .background(AppTheme.colors.surfaceDefault)
      .padding(horizontal = 14.dp),
    contentAlignment = Alignment.CenterStart,
  ) {
    Row(verticalAlignment = Alignment.CenterVertically) {
      Icon(
        icon = Lucide.Search,
        modifier = Modifier.size(16.dp),
        tint = AppTheme.colors.textDisabled,
      )

      Spacer(Modifier.width(10.dp))

      Text(
        "문서 검색...",
        style = AppTheme.typography.action,
        color = AppTheme.colors.textDisabled,
      )
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
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W700),
        color = AppTheme.colors.textFaint,
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
          color = AppTheme.colors.textFaint,
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
                tint = AppTheme.colors.accentBrand,
              )

              Spacer(Modifier.height(6.dp))

              Text(
                folder.name,
                style = AppTheme.typography.title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )

              Spacer(Modifier.height(2.dp))

              Text(
                "문서 ${folder.documentCount}개",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textFaint,
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
  val documents = data.me.recentlyViewedEntities.mapNotNull { it.node.onDocument }

  Column {
    Skeleton.Keep {
      Text(
        "최근 문서",
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W700),
        color = AppTheme.colors.textFaint,
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
          color = AppTheme.colors.textFaint,
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
        ) { doc ->
          DocumentRow(doc)
        }
      }
    }
  }
}

@Composable
private fun DocumentRow(doc: HomeScreen_Query.OnDocument) {
  val nav = Nav.current

  InteractionScope {
    Column(
      modifier = Modifier
        .fillMaxWidth()
        .clickable { nav.navigate(Route.Editor(doc.entity.slug)) }
        .pressScale()
        .padding(horizontal = 16.dp, vertical = 14.dp),
    ) {
      Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(
          icon = when (doc.type) {
            DocumentType.NORMAL -> Lucide.File
            DocumentType.TEMPLATE -> Lucide.LayoutTemplate
            DocumentType.UNKNOWN__ -> Lucide.FileQuestionMark
          },
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textFaint,
        )

        Spacer(Modifier.width(12.dp))

        Text(
          doc.title,
          style = AppTheme.typography.title,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          modifier = Modifier.weight(1f),
        )

        Spacer(Modifier.width(8.dp))

        Text(
          doc.updatedAt.timeAgo(),
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textDisabled,
        )
      }

      if (doc.excerpt.isNotEmpty()) {
        Text(
          doc.excerpt,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textFaint,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          modifier = Modifier.padding(start = 28.dp),
        )
      }
    }
  }
}
