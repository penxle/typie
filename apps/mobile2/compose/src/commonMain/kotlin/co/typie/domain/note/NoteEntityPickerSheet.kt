package co.typie.domain.note

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.EntityRow
import co.typie.domain.entity.buildSearchHighlightedText
import co.typie.domain.entity.displayPreviewText
import co.typie.domain.entity.displayTitle
import co.typie.domain.entity.document
import co.typie.domain.entity.folder
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entity.formatEntityExcerpt
import co.typie.domain.entity.formatFolderName
import co.typie.domain.entity.formatFolderRowSummary
import co.typie.domain.entity.parentFolderMeta
import co.typie.ext.verticalScroll
import co.typie.graphql.NoteEntityPicker_Search_Query
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.EntityParentMeta_folder
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.icons.Lucide
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.scrollFog
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.SheetStop
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

internal val NoteEntityPickerStops = listOf(SheetStop.Top(64.dp))
private val NoteEntityPickerListFadeHeight = 24.dp

@Composable
context(_: SheetScope<Unit>)
internal fun NoteEntityPickerSheet(
  linkedEntityIds: Set<String>,
  onAddEntity: suspend (String) -> Boolean,
  onRemoveEntity: suspend (String) -> Boolean,
) {
  if (Preference.siteId == null) return
  val model = viewModel { NoteEntityPickerViewModel() }
  val listScrollState = rememberScrollState()
  var updatingEntityId by remember { mutableStateOf<String?>(null) }
  var selectedEntityIds by remember(linkedEntityIds) { mutableStateOf(linkedEntityIds) }
  val actionScope = rememberCoroutineScope()
  val highlightColor = AppTheme.colors.textDefault
  val mutedTextColor = AppTheme.colors.textHint

  LaunchedEffect(model) { model.clearSearch() }

  val showRecentSkeleton =
    model.inputKeyword.isBlank() && model.recentQuery.state is QueryState.Loading
  val visibleEntities =
    if (model.inputKeyword.isBlank()) {
      resolveRecentNotePickerEntities(
          inputKeyword = model.inputKeyword,
          recentQueryState = model.recentQuery.state,
          settledEntities = model.recentEntities,
          placeholderEntities = model.recentPlaceholderEntities,
        )
        .map(::recentNotePickerItem)
    } else {
      model.searchHits
        .mapNotNull { searchNotePickerItem(it, highlightColor, mutedTextColor) }
        .distinctBy { it.id }
    }

  val emptyMessage =
    when {
      model.inputKeyword.isBlank() && model.recentQuery.state is QueryState.Loading -> "불러오는 중..."
      model.inputKeyword.isBlank() -> "최근 항목이 없어요."
      model.searchQuery.state is QueryState.Loading -> "검색 중..."
      model.searchQuery.state is QueryState.Error -> "검색 결과를 불러올 수 없어요."
      else -> "검색 결과가 없어요."
    }

  SheetLayout(
    modifier = Modifier.fillMaxHeight(),
    fillHeight = true,
    bodyScroll = false,
    padding =
      SheetPadding(
        header = PaddingValues(horizontal = 16.dp),
        body = PaddingValues(vertical = 0.dp),
      ),
    header = {
      SheetBar(
        leading = {
          SheetBarTextButton(
            text = "완료",
            color = AppTheme.colors.textDefault,
            enabled = updatingEntityId == null,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = "연결 추가",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        },
      )
    },
  ) {
    Column(
      modifier = Modifier.fillMaxWidth().weight(1f),
      verticalArrangement = Arrangement.spacedBy(0.dp),
    ) {
      Box(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp).padding(top = 12.dp)) {
        TextField(
          value = model.inputKeyword,
          onValueChange = { model.setKeyword(it) },
          label = "검색",
          labelPosition = LabelPosition.None,
          placeholder = "문서나 폴더 검색",
        )
      }

      CardSurface(
        modifier = Modifier.fillMaxWidth().weight(1f).padding(horizontal = 16.dp),
        color = AppTheme.colors.surfaceInset,
      ) {
        if (visibleEntities.isEmpty()) {
          Box(
            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 16.dp),
            contentAlignment = Alignment.TopCenter,
          ) {
            NotePickerEmptyState(message = emptyMessage)
          }
        } else {
          val listFogInsets = remember { PaddingValues(vertical = NoteEntityPickerListFadeHeight) }

          Box(
            modifier =
              Modifier.fillMaxSize()
                .scrollFog(padding = listFogInsets, color = AppTheme.colors.surfaceInset)
          ) {
            Skeleton(enabled = showRecentSkeleton) {
              Column(
                modifier =
                  Modifier.fillMaxSize().verticalScroll(listScrollState).padding(listFogInsets)
              ) {
                Column(Modifier.fillMaxWidth()) {
                  visibleEntities.forEachIndexed { index, item ->
                    if (index > 0) {
                      CardDivider(color = AppTheme.colors.borderDefault)
                    }

                    NotePickerRow(
                      item = item,
                      selected = item.id in selectedEntityIds,
                      updating = updatingEntityId == item.id,
                      enabled = updatingEntityId == null,
                      onClick = {
                        actionScope.launch {
                          if (updatingEntityId != null) {
                            return@launch
                          }

                          val selected = item.id in selectedEntityIds
                          updatingEntityId = item.id

                          try {
                            val didToggle =
                              if (selected) onRemoveEntity(item.id) else onAddEntity(item.id)
                            if (didToggle) {
                              selectedEntityIds =
                                if (selected) selectedEntityIds - item.id
                                else selectedEntityIds + item.id
                            }
                          } finally {
                            updatingEntityId = null
                          }
                        }
                      },
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
}

@Composable
private fun NotePickerEmptyState(message: String, modifier: Modifier = Modifier) {
  Box(modifier = modifier.fillMaxWidth().height(120.dp), contentAlignment = Alignment.Center) {
    Text(message, style = AppTheme.typography.action, color = AppTheme.colors.textMuted)
  }
}

@Composable
private fun NotePickerRow(
  item: NotePickerItem,
  selected: Boolean,
  updating: Boolean,
  enabled: Boolean,
  onClick: suspend () -> Unit,
) {
  val metaColor = AppTheme.colors.textHint

  EntityRow(
    entity = item.entity,
    interactive = enabled,
    trailing = {
      when {
        updating -> Text("...", style = AppTheme.typography.caption, color = metaColor)
        selected ->
          Icon(
            icon = Lucide.Check,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textDefault,
          )
        else -> Spacer(Modifier.size(16.dp))
      }
    },
    onClick = onClick,
  ) {
    parentMeta(item.parentFolder)
    title(title = item.title, subtitle = item.subtitle)
    item.previewText?.let { previewText ->
      supporting(text = previewText, maxLines = item.previewMaxLines)
    }
  }
}

private data class NotePickerItem(
  val entity: EntityRow_entity,
  val title: AnnotatedString,
  val subtitle: AnnotatedString? = null,
  val previewText: AnnotatedString? = null,
  val previewMaxLines: Int = 1,
  val parentFolder: EntityParentMeta_folder? = null,
) {
  val id: String
    get() = entity.id
}

internal fun <T> resolveRecentNotePickerEntities(
  inputKeyword: String,
  recentQueryState: QueryState<*>,
  settledEntities: List<T>,
  placeholderEntities: List<T>,
): List<T> {
  if (inputKeyword.isNotBlank()) {
    return emptyList()
  }

  return when (recentQueryState) {
    QueryState.Loading -> placeholderEntities
    is QueryState.Success<*> -> settledEntities
    is QueryState.Error -> emptyList()
  }
}

private fun recentNotePickerItem(entity: NoteEntityPicker_entity): NotePickerItem {
  val rowEntity = entity.entityRow_entity
  return NotePickerItem(
    entity = rowEntity,
    title = AnnotatedString(rowEntity.displayTitle()),
    previewText = rowEntity.displayPreviewText()?.let(::AnnotatedString),
    previewMaxLines = 2,
    parentFolder = entity.entityRowParent_entity.parentFolderMeta(),
  )
}

private fun searchNotePickerItem(
  hit: NoteEntityPicker_Search_Query.Hit,
  highlightColor: Color,
  mutedTextColor: Color,
): NotePickerItem? {
  hit.onSearchHitDocument?.let { documentHit ->
    val entity = documentHit.document.entity.noteEntityPicker_entity
    val rowEntity = entity.entityRow_entity
    val document = rowEntity.document ?: return null
    val title = formatDocumentTitle(documentHit.title ?: document.title)
    val subtitle = documentHit.subtitle ?: document.subtitle
    val previewText = documentHit.text ?: formatEntityExcerpt(document.excerpt)

    return NotePickerItem(
      entity = rowEntity,
      title = buildSearchHighlightedText(title, highlightColor),
      subtitle = subtitle?.let { buildSearchHighlightedText(it, highlightColor, mutedTextColor) },
      previewText = buildSearchHighlightedText(previewText, highlightColor),
      previewMaxLines = if (documentHit.text != null) 2 else 1,
      parentFolder = entity.entityRowParent_entity.parentFolderMeta(),
    )
  }

  hit.onSearchHitFolder?.let { folderHit ->
    val entity = folderHit.folder.entity.noteEntityPicker_entity
    val rowEntity = entity.entityRow_entity
    val folder = rowEntity.folder ?: return null
    val title = formatFolderName(folderHit.name ?: folder.name)

    return NotePickerItem(
      entity = rowEntity,
      title = buildSearchHighlightedText(title, highlightColor),
      previewText =
        AnnotatedString(
          formatFolderRowSummary(
            folderCount = folder.folderCount,
            documentCount = folder.documentCount,
          )
        ),
      parentFolder = entity.entityRowParent_entity.parentFolderMeta(),
    )
  }

  return null
}
