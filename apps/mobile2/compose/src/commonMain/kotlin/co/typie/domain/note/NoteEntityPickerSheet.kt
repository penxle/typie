package co.typie.domain.note

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState as foundationRememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.icons.Lucide
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.SheetStop
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppTheme

internal val NoteEntityPickerStops = listOf(SheetStop.Top(64.dp))
private val NoteEntityPickerListFadeHeight = 24.dp

@Composable
context(_: SheetScope<Unit>)
internal fun NoteEntityPickerSheet(
  linkedEntityIds: Set<String>,
  onAddEntity: suspend (String) -> Boolean,
  onRemoveEntity: suspend (String) -> Boolean,
) {
  val currentSiteId = Preference.siteId ?: return
  val model = viewModel(key = "notes-entity-picker:$currentSiteId") { NoteEntityPickerViewModel() }
  val listScrollState = foundationRememberScrollState()
  var updatingEntityId by remember { mutableStateOf<String?>(null) }
  var selectedEntityIds by remember(linkedEntityIds) { mutableStateOf(linkedEntityIds) }

  LaunchedEffect(model) { model.clearSearch() }

  val visibleEntities =
    if (model.inputKeyword.isBlank()) {
      model.recentEntities
    } else {
      model.searchResults
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
            color = AppTheme.colors.brand,
            enabled = updatingEntityId == null,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = "연결 추가",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
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
        color = AppTheme.colors.surfaceSunken,
      ) {
        if (visibleEntities.isEmpty()) {
          Box(
            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 16.dp),
            contentAlignment = Alignment.TopCenter,
          ) {
            NotePickerEmptyState(message = emptyMessage)
          }
        } else {
          val showTopFade by
            remember(listScrollState) { derivedStateOf { listScrollState.value > 0 } }
          val showBottomFade by
            remember(listScrollState) {
              derivedStateOf { listScrollState.value < listScrollState.maxValue }
            }
          val topFadeAlpha by
            animateFloatAsState(
              targetValue = if (showTopFade) 1f else 0f,
              animationSpec = tween(250),
            )
          val bottomFadeAlpha by
            animateFloatAsState(
              targetValue = if (showBottomFade) 1f else 0f,
              animationSpec = tween(250),
            )

          Box(modifier = Modifier.fillMaxSize()) {
            Column(modifier = Modifier.fillMaxSize().verticalScroll(listScrollState)) {
              Column(Modifier.fillMaxWidth()) {
                visibleEntities.forEachIndexed { index, entity ->
                  if (index > 0) {
                    CardDivider()
                  }

                  NotePickerRow(
                    entity = entity,
                    selected = entity.id in selectedEntityIds,
                    updating = updatingEntityId == entity.id,
                    enabled = updatingEntityId == null,
                    onClick = {
                      val selected = entity.id in selectedEntityIds
                      updatingEntityId = entity.id
                      val didToggle =
                        if (selected) onRemoveEntity(entity.id) else onAddEntity(entity.id)
                      if (didToggle) {
                        selectedEntityIds =
                          if (selected) selectedEntityIds - entity.id
                          else selectedEntityIds + entity.id
                      }
                      updatingEntityId = null
                    },
                  )
                }
              }
            }

            Box(
              modifier =
                Modifier.align(Alignment.TopCenter)
                  .fillMaxWidth()
                  .height(NoteEntityPickerListFadeHeight)
            ) {
              Box(
                modifier =
                  Modifier.fillMaxSize()
                    .graphicsLayer { alpha = topFadeAlpha }
                    .background(
                      brush =
                        Brush.verticalGradient(
                          colorStops =
                            arrayOf(
                              0.3f to AppTheme.colors.surfaceSunken.copy(alpha = 0.92f),
                              1f to AppTheme.colors.surfaceSunken.copy(alpha = 0f),
                            )
                        )
                    )
              )
            }

            Box(
              modifier =
                Modifier.align(Alignment.BottomCenter)
                  .fillMaxWidth()
                  .height(NoteEntityPickerListFadeHeight)
            ) {
              Box(
                modifier =
                  Modifier.fillMaxSize()
                    .graphicsLayer { alpha = bottomFadeAlpha }
                    .background(
                      brush =
                        Brush.verticalGradient(
                          colorStops =
                            arrayOf(
                              0f to AppTheme.colors.surfaceSunken.copy(alpha = 0f),
                              0.7f to AppTheme.colors.surfaceSunken.copy(alpha = 0.92f),
                            )
                        )
                    )
              )
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
    Text(message, style = AppTheme.typography.action, color = AppTheme.colors.textTertiary)
  }
}

@Composable
private fun NotePickerRow(
  entity: NoteEntityPicker_entity,
  selected: Boolean,
  updating: Boolean,
  enabled: Boolean,
  onClick: suspend () -> Unit,
) {
  val iconAppearance = entity.iconAppearance()
  val metaColor = AppTheme.colors.textMuted
  val parentFolder = entity.parentFolder()
  val parentFolderIconAppearance =
    resolveEntityIconAppearance(
      iconName = parentFolder?.icon,
      iconColor = parentFolder?.iconColor,
      fallbackIcon = Lucide.Folder,
      fallbackTint = metaColor,
      colors = AppTheme.colors,
    )

  InteractionScope {
    Column(
      modifier =
        Modifier.fillMaxWidth()
          .clickable { if (enabled) onClick() }
          .pressScale()
          .padding(horizontal = 16.dp, vertical = 12.dp)
    ) {
      if (parentFolder != null) {
        Row(verticalAlignment = Alignment.CenterVertically) {
          Icon(
            icon = parentFolderIconAppearance.icon,
            modifier = Modifier.size(12.dp),
            tint = parentFolderIconAppearance.tint,
          )

          Spacer(Modifier.width(4.dp))

          Text(
            text = parentFolder.name,
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
          icon = iconAppearance.icon,
          modifier = Modifier.size(18.dp),
          tint = iconAppearance.tint,
        )

        Spacer(Modifier.width(12.dp))

        Column(modifier = Modifier.weight(1f)) {
          Text(
            text = entity.displayTitle(),
            style = AppTheme.typography.label,
            color = AppTheme.colors.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )

          entity.displayPreviewText()?.let { previewText ->
            Spacer(Modifier.height(4.dp))

            Text(
              text = previewText,
              style = AppTheme.typography.caption,
              color = metaColor,
              maxLines = 2,
              overflow = TextOverflow.Ellipsis,
            )
          }
        }

        Spacer(Modifier.width(8.dp))

        when {
          updating -> Text("...", style = AppTheme.typography.caption, color = metaColor)
          selected ->
            Icon(icon = Lucide.Check, modifier = Modifier.size(16.dp), tint = AppTheme.colors.brand)
          else -> Spacer(Modifier.size(16.dp))
        }
      }
    }
  }
}
