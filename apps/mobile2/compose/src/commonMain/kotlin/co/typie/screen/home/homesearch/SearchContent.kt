package co.typie.screen.home.homesearch

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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.datetime.timeAgo
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.HomeScreen_Search_Query
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.SearchFolderRow
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppTheme

private val SearchScreenHorizontalPadding = 20.dp

@Composable
fun SearchContent(
  modifier: Modifier = Modifier,
  searchViewModel: SearchViewModel,
  headerHeight: Dp,
  onDocumentClick: suspend (slug: String, queryText: String) -> Unit,
  onFolderClick: suspend (entityId: String, queryText: String) -> Unit,
) {
  Column(modifier = modifier.fillMaxWidth()) {
    Spacer(modifier = Modifier.fillMaxWidth().height(headerHeight))

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
          SearchStateMessage("검색 중...")
        }

        is QueryState.Success -> {
          if (state.data.search.hits.isEmpty()) {
            SearchStateMessage("검색 결과가 없습니다")
          } else {
            SearchResultsList(
              data = state.data,
              queryText = searchViewModel.query,
              onDocumentClick = onDocumentClick,
              onFolderClick = onFolderClick,
            )
          }
        }

        is QueryState.Error -> {
          SearchStateMessage("검색 중 오류가 발생했습니다")
        }
      }
    }
  }
}

@Composable
private fun SearchStateMessage(text: String) {
  Box(
    modifier = Modifier.fillMaxWidth().padding(top = 64.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text, style = AppTheme.typography.action, color = AppTheme.colors.textTertiary)
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
      modifier =
        Modifier.padding(horizontal = SearchScreenHorizontalPadding)
          .padding(top = 20.dp, bottom = 12.dp),
    )

    if (recentSearches.isEmpty()) {
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .padding(horizontal = SearchScreenHorizontalPadding)
            .padding(top = 32.dp),
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
        InteractionScope {
          Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier =
              Modifier.fillMaxWidth()
                .clickable { onSelect(search) }
                .pressScale()
                .padding(horizontal = SearchScreenHorizontalPadding, vertical = 14.dp),
          ) {
            Icon(
              icon = Lucide.Clock,
              modifier = Modifier.size(16.dp),
              tint = AppTheme.colors.textTertiary,
            )
            Spacer(Modifier.width(12.dp))
            Text(search, style = AppTheme.typography.action, modifier = Modifier.weight(1f))
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
}

@Composable
private fun SearchResultsList(
  data: HomeScreen_Search_Query.Data,
  queryText: String,
  onDocumentClick: suspend (slug: String, queryText: String) -> Unit,
  onFolderClick: suspend (entityId: String, queryText: String) -> Unit,
) {
  val highlightColor = AppTheme.colors.brand
  val hits =
    data.search.hits.filter { hit ->
      hit.onSearchHitDocument != null || hit.onSearchHitFolder != null
    }

  Column(
    modifier = Modifier.padding(horizontal = SearchScreenHorizontalPadding).padding(top = 16.dp)
  ) {
    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(Modifier.fillMaxWidth()) {
        hits.forEachIndexed { index, hit ->
          val onDocument = hit.onSearchHitDocument
          val onFolder = hit.onSearchHitFolder

          when {
            onDocument != null -> {
              SearchDocumentResultRow(
                entityIconName = onDocument.document.entity.icon,
                entityIconColor = onDocument.document.entity.iconColor,
                highlightedTitle = onDocument.title,
                highlightedSubtitle = onDocument.subtitle,
                fallbackTitle = onDocument.document.title,
                fallbackSubtitle = onDocument.document.subtitle,
                folderName = onDocument.document.entity.parent?.node?.onFolder?.name,
                folderIconName = onDocument.document.entity.parent?.node?.onFolder?.entity?.icon,
                folderIconColor =
                  onDocument.document.entity.parent?.node?.onFolder?.entity?.iconColor,
                excerpt = onDocument.document.excerpt,
                updatedAt = onDocument.document.updatedAt,
                highlightedText = onDocument.text,
                highlightColor = highlightColor,
                onClick = { onDocumentClick(onDocument.document.entity.slug, queryText) },
              )
            }

            onFolder != null -> {
              SearchFolderResultRow(
                iconName = onFolder.folder.entity.icon,
                iconColor = onFolder.folder.entity.iconColor,
                highlightedName = onFolder.name,
                fallbackName = onFolder.folder.name,
                folderCount = onFolder.folder.folderCount,
                documentCount = onFolder.folder.documentCount,
                highlightColor = highlightColor,
                onClick = { onFolderClick(onFolder.folder.entity.id, queryText) },
              )
            }
          }

          if (index != hits.lastIndex) {
            CardDivider()
          }
        }
      }
    }
  }
}

@Composable
private fun SearchDocumentResultRow(
  entityIconName: String,
  entityIconColor: String,
  highlightedTitle: String?,
  highlightedSubtitle: String?,
  fallbackTitle: String,
  fallbackSubtitle: String?,
  folderName: String?,
  folderIconName: String?,
  folderIconColor: String?,
  excerpt: String,
  updatedAt: kotlin.time.Instant,
  highlightedText: String?,
  highlightColor: Color,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    val metaColor = AppTheme.colors.textMuted
    val entityIcon =
      resolveEntityIconAppearance(
        iconName = entityIconName,
        iconColor = entityIconColor,
        fallbackIcon = Lucide.File,
        fallbackTint = metaColor,
        colors = AppTheme.colors,
      )
    val folderIcon =
      resolveEntityIconAppearance(
        iconName = folderIconName,
        iconColor = folderIconColor,
        fallbackIcon = Lucide.Folder,
        fallbackTint = metaColor,
        colors = AppTheme.colors,
      )
    val titleText =
      buildSearchDocumentTitleText(
        highlightedTitle = highlightedTitle,
        highlightedSubtitle = highlightedSubtitle,
        fallbackTitle = fallbackTitle,
        fallbackSubtitle = fallbackSubtitle,
        highlightColor = highlightColor,
        subtitleColor = metaColor,
      )
    val detailText = highlightedText?.takeIf { it.isNotEmpty() }

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .clickable(onClick)
          .pressScale()
          .padding(horizontal = 16.dp, vertical = 12.dp)
    ) {
      if (folderName != null) {
        Row(verticalAlignment = Alignment.CenterVertically) {
          Icon(icon = folderIcon.icon, modifier = Modifier.size(12.dp), tint = folderIcon.tint)

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
        Icon(icon = entityIcon.icon, modifier = Modifier.size(18.dp), tint = entityIcon.tint)

        Spacer(Modifier.width(12.dp))

        Column(Modifier.weight(1f)) {
          Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
              titleText,
              style = AppTheme.typography.label,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
              modifier = Modifier.weight(1f),
            )

            Spacer(Modifier.width(8.dp))

            Text(updatedAt.timeAgo(), style = AppTheme.typography.caption, color = metaColor)
          }

          if (detailText != null) {
            Spacer(Modifier.height(4.dp))

            Text(
              parseEmHighlight(detailText, highlightColor),
              style = AppTheme.typography.caption,
              color = metaColor,
              maxLines = 2,
              overflow = TextOverflow.Ellipsis,
            )
          } else {
            Spacer(Modifier.height(4.dp))

            Text(
              if (excerpt.isNotEmpty()) excerpt else "(내용 없음)",
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

@Composable
private fun SearchFolderResultRow(
  iconName: String,
  iconColor: String,
  highlightedName: String?,
  fallbackName: String,
  folderCount: Int,
  documentCount: Int,
  highlightColor: Color,
  onClick: suspend () -> Unit,
) {
  val titleText =
    highlightedName?.let { parseEmHighlight(it, highlightColor) } ?: AnnotatedString(fallbackName)

  SearchFolderRow(
    title = titleText,
    iconName = iconName,
    iconColor = iconColor,
    folderCount = folderCount,
    documentCount = documentCount,
    onClick = onClick,
  )
}

private fun buildSearchDocumentTitleText(
  highlightedTitle: String?,
  highlightedSubtitle: String?,
  fallbackTitle: String,
  fallbackSubtitle: String?,
  highlightColor: Color,
  subtitleColor: Color,
): AnnotatedString {
  val subtitle = highlightedSubtitle ?: fallbackSubtitle?.takeIf { it.isNotBlank() }

  return buildAnnotatedString {
    append(parseEmHighlight(highlightedTitle ?: fallbackTitle, highlightColor))

    if (subtitle != null) {
      pushStyle(SpanStyle(color = subtitleColor))
      append(" — ")
      pop()
      append(parseEmHighlight(subtitle, highlightColor, subtitleColor))
    }
  }
}

private fun parseEmHighlight(
  text: String,
  highlightColor: Color,
  baseColor: Color? = null,
): AnnotatedString {
  return buildAnnotatedString {
    var remaining = text
    while (remaining.isNotEmpty()) {
      val startIdx = remaining.indexOf("<em>")
      if (startIdx == -1) {
        appendWithColor(remaining, baseColor)
        break
      }
      appendWithColor(remaining.substring(0, startIdx), baseColor)
      val endIdx = remaining.indexOf("</em>", startIdx)
      if (endIdx == -1) {
        appendWithColor(remaining.substring(startIdx), baseColor)
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

private fun AnnotatedString.Builder.appendWithColor(text: String, color: Color?) {
  if (text.isEmpty()) return

  if (color == null) {
    append(text)
    return
  }

  pushStyle(SpanStyle(color = color))
  append(text)
  pop()
}
