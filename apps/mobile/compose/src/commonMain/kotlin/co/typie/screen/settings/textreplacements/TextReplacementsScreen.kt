package co.typie.screen.settings.textreplacements

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.settings.SettingSwitch
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.domain.subscription.grantsAccess
import co.typie.ext.clickable
import co.typie.ext.plus
import co.typie.ext.separated
import co.typie.icons.Lucide
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.reorder.ReorderableLazyColumn
import co.typie.ui.component.reorder.ReorderableLazyColumnState
import co.typie.ui.component.reorder.rememberReorderableLazyColumnState
import co.typie.ui.component.reorder.reorderableAnimatedItem
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.reorder.reorderableViewport
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberLazyListState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private const val TextReplacementsTitleKey = "text-replacements-title"

private const val TextReplacementsDescriptionKey = "text-replacements-description"

private const val TextReplacementsPresetKey = "text-replacements-preset"

private const val TextReplacementsCustomHeaderKey = "text-replacements-custom-header"

private const val TextReplacementsEmptyKey = "text-replacements-empty"

@Composable
fun TextReplacementsScreen() {
  val model = viewModel { TextReplacementsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberLazyListState()

  val sheet = LocalSheet.current

  ProvideTopBar(
    center = { Text("텍스트 대치", style = AppTheme.typography.title) },
    trailing = {
      TopBarButton(
        icon = Lucide.Plus,
        onClick = {
          if (SubscriptionService.gate(sheet, GatedAction.TextReplacement)) {
            sheet.present { TextReplacementEditSheet(model = model, editing = null) }
          }
        },
      )
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { innerPadding ->
    val displayed = model.customs
    val keys = displayed.map { it.textReplacementId }
    val reorderState = rememberReorderableLazyColumnState(keys = keys, lazyListState = scrollState)
    val byId = remember(displayed) { displayed.associateBy { it.textReplacementId } }
    val ordered = reorderState.layoutKeys.mapNotNull(byId::get)
    val toast = LocalToast.current
    val reorderEnabled = SubscriptionService.entitlement.grantsAccess()

    ReorderableLazyColumn(
      state = reorderState,
      modifier =
        Modifier.fillMaxSize()
          .reorderableViewport(state = reorderState, viewportTopInset = topBarOcclusion),
      contentPadding = innerPadding + AppTheme.spacings.scrollBottomPadding,
    ) {
      item(key = TextReplacementsTitleKey) { Text("텍스트 대치", style = AppTheme.typography.display) }
      item(key = TextReplacementsDescriptionKey) {
        Text(
          "입력 중 특정 텍스트를 자동으로 변환해요.",
          modifier = Modifier.padding(top = 16.dp),
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
        )
      }
      item(key = TextReplacementsPresetKey) {
        Box(modifier = Modifier.padding(top = 16.dp)) {
          PresetSection(model = model, scope = scope)
        }
      }
      item(key = TextReplacementsCustomHeaderKey) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(top = 16.dp, bottom = 12.dp),
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          SectionTitle(text = "사용자 대치", modifier = Modifier.padding(top = 4.dp))
          Text(
            text = "위에서부터 순서대로 먼저 매치되는 규칙이 적용돼요.",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textMuted,
          )
        }
      }

      if (ordered.isEmpty()) {
        item(key = TextReplacementsEmptyKey) {
          CardSurface(modifier = Modifier.fillMaxWidth()) { EmptyStateMessage() }
        }
      } else {
        itemsIndexed(items = ordered, key = { _, entry -> entry.textReplacementId }) { _, entry ->
          val id = entry.textReplacementId
          val projectedIndex = reorderState.keys.indexOf(id)
          val shape =
            RoundedCornerShape(
              topStart = if (projectedIndex == 0) AppShapes.md else 0.dp,
              topEnd = if (projectedIndex == 0) AppShapes.md else 0.dp,
              bottomStart =
                if (projectedIndex == reorderState.keys.lastIndex) AppShapes.md else 0.dp,
              bottomEnd = if (projectedIndex == reorderState.keys.lastIndex) AppShapes.md else 0.dp,
            )
          CardSurface(
            modifier =
              reorderableAnimatedItem(state = reorderState, key = id)
                .reorderableItem(state = reorderState, key = id),
            shape = shape,
          ) {
            Column(modifier = Modifier.fillMaxWidth()) {
              if (projectedIndex > 0) CardDivider(inset = 20.dp)
              CustomRow(
                entry = entry,
                order = projectedIndex + 1,
                reorderState = reorderState,
                reorderEnabled = reorderEnabled,
                onEdit = {
                  if (SubscriptionService.gate(sheet, GatedAction.TextReplacement)) {
                    sheet.present { TextReplacementEditSheet(model = model, editing = entry) }
                  }
                },
                onToggle = {
                  if (SubscriptionService.gate(sheet, GatedAction.TextReplacement)) {
                    model
                      .updateTextReplacementState(entry.textReplacementId, entry.isActive)
                      .withDefaultExceptionHandler(toast)
                  }
                },
                onReorderCommit = { movedKey, orderedKeys ->
                  scope.launch {
                    model.reorderCustom(movedKey, orderedKeys).withDefaultExceptionHandler(toast)
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

@Composable
private fun PresetSection(model: TextReplacementsViewModel, scope: CoroutineScope) {
  val toast = LocalToast.current
  val sheet = LocalSheet.current

  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(text = "기본 대치", modifier = Modifier.padding(top = 4.dp))

    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(modifier = Modifier.fillMaxWidth()) {
        CardRow(
          onClick = {
            if (SubscriptionService.gate(sheet, GatedAction.TextReplacement)) {
              model
                .updateSmartQuotesTextReplacementState(model.smartQuotes.all { it.isActive })
                .withDefaultExceptionHandler(toast)
            }
          }
        ) {
          Text(
            text = "곧은따옴표를 둥근따옴표로",
            style = AppTheme.typography.label,
            modifier = Modifier.weight(1f),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
          SettingSwitch(
            checked = model.smartQuotes.all { it.isActive },
            onCheckedChange = { next ->
              scope.launch {
                if (!SubscriptionService.gate(sheet, GatedAction.TextReplacement)) return@launch
                model.updateSmartQuotesTextReplacementState(next).withDefaultExceptionHandler(toast)
              }
            },
          )
        }

        CardDivider()

        model.presets.separated(separator = { CardDivider() }) {
          PresetRow(
            entry = it,
            onClick = {
              if (SubscriptionService.gate(sheet, GatedAction.TextReplacement)) {
                model
                  .updateTextReplacementState(it.textReplacementId, !it.isActive)
                  .withDefaultExceptionHandler(toast)
              }
            },
            onCheckedChange = { next ->
              scope.launch {
                if (!SubscriptionService.gate(sheet, GatedAction.TextReplacement)) return@launch
                model
                  .updateTextReplacementState(it.textReplacementId, next)
                  .withDefaultExceptionHandler(toast)
              }
            },
          )
        }
      }
    }
  }
}

@Composable
private fun PresetRow(
  entry: TextReplacement,
  onClick: suspend () -> Unit,
  onCheckedChange: (Boolean) -> Unit,
) {
  CardRow(onClick = onClick) {
    RuleLabel(entry = entry, modifier = Modifier.weight(1f))
    SettingSwitch(checked = entry.isActive, onCheckedChange = onCheckedChange)
  }
}

@Composable
private fun CustomRow(
  entry: TextReplacement,
  order: Int,
  reorderState: ReorderableLazyColumnState<String>,
  reorderEnabled: Boolean,
  onEdit: suspend () -> Unit,
  onToggle: suspend () -> Unit,
  onReorderCommit: (movedKey: String, orderedKeys: List<String>) -> Unit,
) {
  val id = entry.textReplacementId
  val toggleScope = rememberCoroutineScope()

  Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
    Box(
      modifier =
        Modifier.reorderableDragHandle(
            state = reorderState,
            key = id,
            enabled = reorderEnabled,
            onDragStopped = { drop ->
              if (drop == null) return@reorderableDragHandle
              onReorderCommit(drop.movedKey, drop.orderedKeys)
            },
          )
          .size(width = 44.dp, height = 56.dp),
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = Lucide.GripVertical,
        modifier = Modifier.size(18.dp),
        tint = AppTheme.colors.textMuted,
      )
    }

    Row(
      modifier =
        Modifier.weight(1f)
          .clickable(onClick = onEdit)
          .padding(top = 16.dp, end = 12.dp, bottom = 16.dp),
      horizontalArrangement = Arrangement.spacedBy(8.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      OrderBadge(order = order)
      RuleLabel(entry = entry, modifier = Modifier.weight(1f))
    }

    Box(modifier = Modifier.padding(start = 8.dp, end = 16.dp)) {
      SettingSwitch(
        checked = entry.isActive,
        onCheckedChange = { toggleScope.launch { onToggle() } },
      )
    }
  }
}

@Composable
private fun EmptyStateMessage() {
  Box(
    modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 24.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = "아직 사용자 대치 규칙이 없어요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
    )
  }
}

@Composable
private fun OrderBadge(order: Int) {
  Box(
    modifier =
      Modifier.clip(AppShapes.rounded(AppShapes.sm))
        .background(AppTheme.colors.surfaceInset)
        .padding(horizontal = 6.dp, vertical = 2.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = order.toString(),
      style = AppTheme.typography.caption.copy(fontFamily = FontFamily.Monospace),
      color = AppTheme.colors.textMuted,
      maxLines = 1,
    )
  }
}

@Composable
private fun RuleLabel(entry: TextReplacement, modifier: Modifier = Modifier) {
  val note = entry.note?.takeIf { it.isNotBlank() }

  Row(
    modifier = modifier,
    horizontalArrangement = Arrangement.spacedBy(6.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    if (note != null) {
      Text(
        text = note,
        style = AppTheme.typography.label,
        modifier = Modifier.weight(1f, fill = false),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    } else {
      Token(text = entry.match, modifier = Modifier.weight(1f, fill = false))
      Icon(
        icon = Lucide.ChevronRight,
        modifier = Modifier.size(14.dp),
        tint = AppTheme.colors.textMuted,
      )
      Token(text = entry.substitute, modifier = Modifier.weight(1f, fill = false))
    }
    if (entry.regex) {
      Icon(icon = Lucide.Regex, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textDefault)
    }
  }
}

@Composable
private fun Token(text: String, modifier: Modifier = Modifier) {
  Box(
    modifier =
      modifier
        .clip(AppShapes.rounded(AppShapes.sm))
        .background(AppTheme.colors.surfaceInset)
        .padding(horizontal = 6.dp, vertical = 2.dp)
  ) {
    Text(
      text = text,
      style = AppTheme.typography.caption.copy(fontFamily = FontFamily.Monospace),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}
