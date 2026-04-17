package co.typie.screen.home.search

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityRow
import co.typie.domain.entity.buildSearchHighlightedText
import co.typie.domain.entity.document
import co.typie.domain.entity.folder
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entity.formatEntityExcerpt
import co.typie.domain.entity.formatFolderName
import co.typie.domain.entity.formatFolderRowSummary
import co.typie.domain.entity.parentFolderMeta
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.SearchScreen_Search_Query
import co.typie.graphql.fragment.SearchResultDocument_hit
import co.typie.graphql.fragment.SearchResultFolder_hit
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
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .padding(AppTheme.spacings.scrollBottomPadding)
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
      hit.searchResultDocument_hit != null || hit.searchResultFolder_hit != null
    }

  Column {
    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(Modifier.fillMaxWidth()) {
        hits.separated(separator = { CardDivider() }) { hit ->
          when {
            hit.searchResultDocument_hit != null -> {
              DocumentRow(hit = requireNotNull(hit.searchResultDocument_hit), onClick = onClick)
            }

            hit.searchResultFolder_hit != null -> {
              FolderRow(hit = requireNotNull(hit.searchResultFolder_hit), onClick = onClick)
            }
          }
        }
      }
    }
  }
}

@Composable
private fun DocumentRow(hit: SearchResultDocument_hit, onClick: suspend () -> Unit) {
  val nav = Nav.current

  val entity = hit.document.entity.entityRow_entity
  val document = entity.document ?: return
  val title = formatDocumentTitle(hit.title ?: document.title)
  val subtitle = hit.subtitle ?: document.subtitle
  val parentFolder = hit.document.entity.entityRowParent_entity.parentFolderMeta()
  val highlightedTitle = buildSearchHighlightedText(title, AppTheme.colors.brand)
  val highlightedSubtitle = subtitle?.let {
    buildSearchHighlightedText(it, AppTheme.colors.brand, AppTheme.colors.textMuted)
  }
  val previewText = hit.text ?: formatEntityExcerpt(document.excerpt)
  val highlightedPreview = buildSearchHighlightedText(previewText, AppTheme.colors.brand)

  EntityRow(
    entity = entity,
    onClick = {
      onClick()
      nav.navigate(Route.Editor(entity.id))
    },
  ) {
    parentMeta(parentFolder)
    title(
      title = highlightedTitle,
      subtitle = highlightedSubtitle,
      trailingText = document.updatedAt.timeAgo(),
    )
    supporting(text = highlightedPreview, maxLines = if (hit.text != null) 2 else 1)
  }
}

@Composable
private fun FolderRow(hit: SearchResultFolder_hit, onClick: suspend () -> Unit) {
  val nav = Nav.current
  val entity = hit.folder.entity.entityRow_entity
  val folder = entity.folder ?: return
  val title = formatFolderName(hit.name ?: folder.name)
  val parentFolder = hit.folder.entity.entityRowParent_entity.parentFolderMeta()
  val highlightedTitle = buildSearchHighlightedText(title, AppTheme.colors.brand)

  EntityRow(
    entity = entity,
    onClick = {
      onClick()
      nav.navigate(Route.Folder(entity.id))
    },
  ) {
    parentMeta(parentFolder)
    title(title = highlightedTitle)
    supporting(
      text =
        formatFolderRowSummary(
          folderCount = folder.folderCount,
          documentCount = folder.documentCount,
        )
    )
  }
}
