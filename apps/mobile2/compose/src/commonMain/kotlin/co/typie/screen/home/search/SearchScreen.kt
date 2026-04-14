package co.typie.screen.home.search

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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityIcon
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.SearchScreen_Search_Query
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun SearchScreen() {
  val model = viewModel { SearchViewModel() }
  val scrollState = rememberScrollState()
  val nav = Nav.current

  ProvideTopBar()

  Screen { contentPadding ->
    Box(Modifier.fillMaxSize()) {
      Column(
        modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding)
      ) {
        if (model.inputKeyword.isBlank()) {
          RecentSearches(
            onSelect = {
              model.setKeyword(it)
              model.flush()
            },
            onRemove = { model.removeRecent(it) },
          )
        } else {
          when (val state = model.searchQuery.state) {
            is QueryState.Loading -> {
              Text(
                "검색 중...",
                style = AppTheme.typography.action,
                color = AppTheme.colors.textTertiary,
                modifier = Modifier.align(Alignment.CenterHorizontally),
              )
            }

            is QueryState.Success -> {
              if (state.data.search.hits.isEmpty()) {
                Text(
                  "검색 결과가 없어요",
                  style = AppTheme.typography.action,
                  color = AppTheme.colors.textTertiary,
                  modifier = Modifier.align(Alignment.CenterHorizontally),
                )
              } else {
                SearchResults(data = state.data, onClick = { model.addRecent() })
              }
            }

            is QueryState.Error -> {
              Text(
                "검색 중 오류가 발생했어요",
                style = AppTheme.typography.action,
                color = AppTheme.colors.textTertiary,
                modifier = Modifier.align(Alignment.CenterHorizontally),
              )
            }
          }
        }
      }
    }
  }
}

@Composable
private fun RecentSearches(onSelect: (String) -> Unit, onRemove: (String) -> Unit) {
  Column {
    Text(
      "최근 검색",
      style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W700),
      color = AppTheme.colors.textTertiary,
    )

    if (Preference.recentSearches.isEmpty()) {
      Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        Text(
          "최근 검색어가 없어요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
        )
      }
    } else {
      for (search in Preference.recentSearches) {
        InteractionScope {
          Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier =
              Modifier.fillMaxWidth()
                .pressScale()
                .clickable { onSelect(search) }
                .padding(vertical = 12.dp),
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
private fun SearchResults(data: SearchScreen_Search_Query.Data, onClick: () -> Unit = {}) {
  val hits =
    data.search.hits.filter { hit ->
      hit.onSearchHitDocument != null || hit.onSearchHitFolder != null
    }

  Column {
    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(Modifier.fillMaxWidth()) {
        hits.separated(separator = { CardDivider() }) { hit ->
          when {
            hit.onSearchHitDocument != null -> {
              DocumentRow(hit = hit.onSearchHitDocument, onClick = onClick)
            }

            hit.onSearchHitFolder != null -> {
              FolderRow(folder = hit.onSearchHitFolder, onClick = onClick)
            }
          }
        }
      }
    }
  }
}

@Composable
private fun DocumentRow(
  hit: SearchScreen_Search_Query.OnSearchHitDocument,
  onClick: suspend () -> Unit,
) {
  val nav = Nav.current

  val title = hit.title ?: hit.document.title
  val subtitle = hit.subtitle ?: hit.document.subtitle
  val parentFolder = hit.document.entity.parent?.node?.onFolder

  InteractionScope {
    Column(
      modifier =
        Modifier.fillMaxWidth()
          .pressScale()
          .clickable(onClick)
          .clickable { nav.navigate(Route.Editor(hit.document.entity.id)) }
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

        Spacer(Modifier.height(4.dp))
      }

      Row(verticalAlignment = Alignment.CenterVertically) {
        EntityIcon(hit.document.entity.entityIcon_entity, modifier = Modifier.size(18.dp))

        Spacer(Modifier.width(12.dp))

        Column(Modifier.weight(1f)) {
          Row(verticalAlignment = Alignment.CenterVertically) {
            val text = buildAnnotatedString {
              append(buildEmHighlightedAnnotatedString(title, AppTheme.colors.brand))

              if (subtitle != null) {
                appendWithColor(" — ", AppTheme.colors.textMuted)
                append(
                  buildEmHighlightedAnnotatedString(
                    subtitle,
                    AppTheme.colors.brand,
                    AppTheme.colors.textMuted,
                  )
                )
              }
            }

            Text(
              text,
              style = AppTheme.typography.label,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
              modifier = Modifier.weight(1f),
            )

            Spacer(Modifier.width(8.dp))

            Text(
              hit.document.updatedAt.timeAgo(),
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
            )
          }

          if (hit.text != null) {
            Spacer(Modifier.height(4.dp))

            Text(
              buildEmHighlightedAnnotatedString(hit.text, AppTheme.colors.brand),
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
              maxLines = 2,
              overflow = TextOverflow.Ellipsis,
            )
          } else {
            Spacer(Modifier.height(4.dp))

            Text(
              hit.document.excerpt.ifEmpty { "(내용 없음)" },
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

@Composable
private fun FolderRow(
  folder: SearchScreen_Search_Query.OnSearchHitFolder,
  onClick: suspend () -> Unit,
) {}

private fun buildEmHighlightedAnnotatedString(
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
