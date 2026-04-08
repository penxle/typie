package co.typie.screen.home

import androidx.compose.foundation.background
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.border
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
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.HomeScreen_Search_Query
import co.typie.graphql.QueryState
import co.typie.graphql.type.DocumentType
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.DocumentThumbnailPreview
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

private val SearchScreenHorizontalPadding = 20.dp
private val SearchScreenTopFadeHeight = 24.dp

@Composable
fun SearchContent(
  modifier: Modifier = Modifier,
  searchViewModel: SearchViewModel,
  contentPadding: PaddingValues,
  scrollState: ScrollState,
) {
  val nav = Nav.current

  Box(
    modifier = modifier
      .fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier
        .fillMaxWidth()
        .verticalScroll(scrollState)
        .padding(contentPadding),
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

    SearchTopFade(
      modifier = Modifier
        .align(Alignment.TopCenter)
        .fillMaxWidth(),
    )
  }
}

@Composable
private fun SearchTopFade(modifier: Modifier = Modifier) {
  Box(
    modifier = modifier
      .height(SearchScreenTopFadeHeight)
      .background(
        Brush.verticalGradient(
          colors = listOf(
            AppTheme.colors.surfaceBase,
            AppTheme.colors.surfaceBase.copy(alpha = 0f),
          ),
        )
      ),
  )
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
      modifier = Modifier.padding(horizontal = SearchScreenHorizontalPadding).padding(top = 20.dp, bottom = 12.dp),
    )

    if (recentSearches.isEmpty()) {
      Box(
        modifier = Modifier.fillMaxWidth().padding(horizontal = SearchScreenHorizontalPadding).padding(top = 32.dp),
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
            modifier = Modifier
              .fillMaxWidth()
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
}

@Composable
private fun SearchResultsList(
  data: HomeScreen_Search_Query.Data,
  queryText: String,
  onDocumentClick: suspend (slug: String, queryText: String) -> Unit,
  onFolderClick: suspend (entityId: String, queryText: String) -> Unit,
) {
  val highlightColor = AppTheme.colors.brand
  val hits = data.search.hits.filter { hit ->
    hit.onSearchHitDocument != null || hit.onSearchHitFolder != null
  }

  Column(
    modifier = Modifier
      .padding(horizontal = SearchScreenHorizontalPadding)
      .padding(top = 16.dp, bottom = 12.dp),
  ) {
    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column(Modifier.fillMaxWidth()) {
        hits.forEachIndexed { index, hit ->
          val onDocument = hit.onSearchHitDocument
          val onFolder = hit.onSearchHitFolder

          when {
            onDocument != null -> {
              SearchDocumentResultRow(
                previewUrl = onDocument.document.previewUrl,
                documentType = onDocument.document.type,
                highlightedTitle = onDocument.title,
                fallbackTitle = onDocument.document.title,
                highlightedText = onDocument.text,
                highlightColor = highlightColor,
                onClick = { onDocumentClick(onDocument.document.entity.slug, queryText) },
              )
            }

            onFolder != null -> {
              SearchFolderResultRow(
                highlightedName = onFolder.name,
                fallbackName = onFolder.folder.name,
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
  previewUrl: String,
  documentType: DocumentType,
  highlightedTitle: String?,
  fallbackTitle: String,
  highlightedText: String?,
  highlightColor: Color,
  onClick: suspend () -> Unit,
) {
  CardRow(
    onClick = onClick,
    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 12.dp),
    spacing = 12.dp,
  ) {
    val shadowColor = AppTheme.colors.shadowAmbient

    DocumentThumbnailPreview(
      url = previewUrl,
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
      placeholderColor = AppTheme.colors.surfaceSunken,
    )

    Column(Modifier.weight(1f)) {
      Row(verticalAlignment = Alignment.CenterVertically) {
        if (documentType == DocumentType.TEMPLATE) {
          Icon(
            icon = Lucide.LayoutTemplate,
            modifier = Modifier.size(14.dp),
            tint = AppTheme.colors.textPrimary,
          )

          Spacer(Modifier.width(4.dp))
        }

        val titleModifier = Modifier.weight(1f)

        if (highlightedTitle != null) {
          Text(
            parseEmHighlight(highlightedTitle, highlightColor),
            style = AppTheme.typography.label,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = titleModifier,
          )
        } else {
          Text(
            fallbackTitle,
            style = AppTheme.typography.label,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = titleModifier,
          )
        }
      }

      if (!highlightedText.isNullOrEmpty()) {
        Spacer(Modifier.height(4.dp))
        Text(
          parseEmHighlight(highlightedText, highlightColor),
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          maxLines = 2,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }
  }
}

@Composable
private fun SearchFolderResultRow(
  highlightedName: String?,
  fallbackName: String,
  highlightColor: Color,
  onClick: suspend () -> Unit,
) {
  CardRow(
    onClick = onClick,
    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 12.dp),
    spacing = 12.dp,
  ) {
    Box(
      modifier = Modifier
        .width(35.dp)
        .height(49.dp)
        .clip(RoundedCornerShape(8.dp))
        .background(AppTheme.colors.surfaceSunken)
        .border(1.dp, AppTheme.colors.borderSubtle, RoundedCornerShape(8.dp)),
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = Lucide.Folder,
        modifier = Modifier.size(18.dp),
        tint = AppTheme.colors.brand,
      )
    }

    Column(Modifier.weight(1f)) {
      if (highlightedName != null) {
        Text(
          parseEmHighlight(highlightedName, highlightColor),
          style = AppTheme.typography.label,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      } else {
        Text(
          fallbackName,
          style = AppTheme.typography.label,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }

      Spacer(Modifier.height(4.dp))

      Text(
        "폴더",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
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
