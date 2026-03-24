package co.typie.screen.home

import androidx.compose.foundation.background
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.graphql.HomeScreen_Search_Query
import co.typie.graphql.QueryState
import co.typie.graphql.type.DocumentType
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

@Composable
fun SearchContent(
  searchViewModel: SearchViewModel,
  contentPadding: PaddingValues,
) {
  val nav = Nav.current

  Column(
    modifier = Modifier
      .fillMaxWidth()
      .verticalScroll(rememberScrollState())
      .padding(contentPadding)
      .navigationBarsPadding(),
  ) {
    if (searchViewModel.activeQuery.isBlank()) {
      RecentSearchesList(
        recentSearches = searchViewModel.recentSearches,
        onSelect = { query ->
          searchViewModel.updateQuery(query)
          searchViewModel.submitQuery()
        },
        onRemove = { searchViewModel.removeRecentSearch(it) },
      )
    } else {
      when (val state = searchViewModel.searchResults.state) {
        is QueryState.Loading -> {
          Box(
            modifier = Modifier.fillMaxWidth().padding(top = 64.dp),
            contentAlignment = Alignment.Center,
          ) {
            Text(
              "검색 중...",
              style = AppTheme.typography.action,
              color = AppTheme.colors.textTertiary,
            )
          }
        }

        is QueryState.Success -> {
          if (state.data.search.hits.isEmpty()) {
            Box(
              modifier = Modifier.fillMaxWidth().padding(top = 64.dp),
              contentAlignment = Alignment.Center,
            ) {
              Text(
                "검색 결과가 없습니다",
                style = AppTheme.typography.action,
                color = AppTheme.colors.textTertiary,
              )
            }
          } else {
            SearchResultsList(
              data = state.data,
              queryText = searchViewModel.query,
              onDocumentClick = { slug, query ->
                searchViewModel.saveRecentSearch(query)
                nav.navigate(Route.Editor(slug))
              },
              onFolderClick = { entityId, query ->
                searchViewModel.saveRecentSearch(query)
                nav.navigate(Route.Folder(entityId))
              },
            )
          }
        }

        is QueryState.Error -> {
          Box(
            modifier = Modifier.fillMaxWidth().padding(top = 64.dp),
            contentAlignment = Alignment.Center,
          ) {
            Text(
              "검색 중 오류가 발생했습니다",
              style = AppTheme.typography.action,
              color = AppTheme.colors.textTertiary,
            )
          }
        }
      }
    }
  }
}

@Composable
private fun RecentSearchesList(
  recentSearches: List<String>,
  onSelect: (String) -> Unit,
  onRemove: (String) -> Unit,
) {
  Column {
    Text(
      "최근 검색",
      style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W700),
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.padding(horizontal = 16.dp).padding(top = 20.dp, bottom = 12.dp),
    )

    if (recentSearches.isEmpty()) {
      Box(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp).padding(top = 32.dp),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "최근 검색어가 없습니다",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      for (search in recentSearches) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          modifier = Modifier
            .fillMaxWidth()
            .clickable { onSelect(search) }
            .padding(horizontal = 16.dp, vertical = 14.dp),
        ) {
          Icon(
            icon = Lucide.Clock,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textTertiary,
          )
          Spacer(Modifier.width(12.dp))
          Text(
            search,
            style = AppTheme.typography.action,
            modifier = Modifier.weight(1f),
          )
          Icon(
            icon = Lucide.X,
            modifier = Modifier.size(16.dp).clickable { onRemove(search) },
            tint = AppTheme.colors.textMuted,
          )
        }
      }
    }
  }
}

@Composable
private fun SearchResultsList(
  data: HomeScreen_Search_Query.Data,
  queryText: String,
  onDocumentClick: suspend (slug: String, queryText: String) -> Unit,
  onFolderClick: suspend (entityId: String, queryText: String) -> Unit,
) {
  val highlightColor = AppTheme.colors.brand
  val hits = data.search.hits

  Column {
    for (hit in hits) {
      val onDocument = hit.onSearchHitDocument
      val onFolder = hit.onSearchHitFolder

      if (onDocument != null) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          modifier = Modifier
            .fillMaxWidth()
            .clickable { onDocumentClick(onDocument.document.entity.slug, queryText) }
            .padding(horizontal = 16.dp, vertical = 16.dp),
        ) {
          Icon(
            icon = when (onDocument.document.type) {
              DocumentType.NORMAL -> Lucide.File
              DocumentType.TEMPLATE -> Lucide.LayoutTemplate
              DocumentType.UNKNOWN__ -> Lucide.FileQuestionMark
            },
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textTertiary,
          )
          Spacer(Modifier.width(12.dp))
          Column(Modifier.weight(1f)) {
            val title = onDocument.title
            if (title != null) {
              Text(
                parseEmHighlight(title, highlightColor),
                style = AppTheme.typography.label,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            } else {
              Text(
                onDocument.document.title,
                style = AppTheme.typography.label,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }
            if (!onDocument.text.isNullOrEmpty()) {
              Spacer(Modifier.height(4.dp))
              Text(
                parseEmHighlight(onDocument.text, highlightColor),
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }
          }
        }
        Box(
          Modifier.fillMaxWidth().height(1.dp).padding(horizontal = 16.dp)
            .background(AppTheme.colors.borderSubtle)
        )
      }

      if (onFolder != null) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          modifier = Modifier
            .fillMaxWidth()
            .clickable { onFolderClick(onFolder.folder.entity.id, queryText) }
            .padding(horizontal = 16.dp, vertical = 16.dp),
        ) {
          Icon(
            icon = Lucide.Folder,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.brand,
          )
          Spacer(Modifier.width(12.dp))
          val folderName = onFolder.name
          if (folderName != null) {
            Text(
              parseEmHighlight(folderName, highlightColor),
              style = AppTheme.typography.label,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          } else {
            Text(
              onFolder.folder.name,
              style = AppTheme.typography.label,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          }
        }
        Box(
          Modifier.fillMaxWidth().height(1.dp).padding(horizontal = 16.dp)
            .background(AppTheme.colors.borderSubtle)
        )
      }
    }
  }
}

private fun parseEmHighlight(text: String, highlightColor: Color): AnnotatedString {
  return buildAnnotatedString {
    var remaining = text
    while (remaining.isNotEmpty()) {
      val startIdx = remaining.indexOf("<em>")
      if (startIdx == -1) {
        append(remaining)
        break
      }
      append(remaining.substring(0, startIdx))
      val endIdx = remaining.indexOf("</em>", startIdx)
      if (endIdx == -1) {
        append(remaining.substring(startIdx))
        break
      }
      val highlighted = remaining.substring(startIdx + 4, endIdx)
      pushStyle(SpanStyle(color = highlightColor))
      append(highlighted)
      pop()
      remaining = remaining.substring(endIdx + 5)
    }
  }
}
