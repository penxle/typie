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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
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
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.ext.clickable
import co.typie.ext.separated
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.reorder.ReorderableColumn
import co.typie.ui.component.reorder.ReorderableColumnState
import co.typie.ui.component.reorder.rememberReorderableColumnState
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.reorder.reorderableViewport
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

@Composable
fun TextReplacementsScreen() {
  val model = viewModel { TextReplacementsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

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

  Screen(loadable = model.query) { contentPadding ->
    val displayed = model.customs
    val keys = displayed.map { it.textReplacementId }
    val reorderState =
      rememberReorderableColumnState(keys = keys, verticalScrollableState = scrollState)

    Box(
      modifier =
        Modifier.fillMaxSize()
          .reorderableViewport(
            state = reorderState,
            viewportTopInset =
              maxOf(
                0.dp,
                contentPadding.calculateTopPadding() -
                  TopBarDefaults.BlurFadeHeight -
                  TopBarDefaults.ContentTopSpacing,
              ),
          )
    ) {
      Column(
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .padding(AppTheme.spacings.scrollBottomPadding),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        Text("텍스트 대치", style = AppTheme.typography.display)
        Text(
          "입력 중 특정 텍스트를 자동으로 변환해요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
        )

        PresetSection(model = model, scope = scope)
        CustomSection(
          model = model,
          displayed = displayed,
          reorderState = reorderState,
          scope = scope,
        )
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
private fun CustomSection(
  model: TextReplacementsViewModel,
  displayed: List<TextReplacement>,
  reorderState: ReorderableColumnState<String>,
  scope: CoroutineScope,
) {
  val sheet = LocalSheet.current
  val toast = LocalToast.current
  val reorderEnabled = SubscriptionService.entitlement !is Entitlement.Expired

  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(text = "사용자 대치", modifier = Modifier.padding(top = 4.dp))
    Text(
      text = "위에서부터 순서대로 먼저 매치되는 규칙이 적용돼요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
    )

    if (displayed.isEmpty()) {
      CardSurface(modifier = Modifier.fillMaxWidth()) { EmptyStateMessage() }
    } else {
      val byId = remember(displayed) { displayed.associateBy { it.textReplacementId } }
      val ordered = reorderState.keys.mapNotNull(byId::get)
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        ReorderableColumn(state = reorderState, modifier = Modifier.fillMaxWidth()) {
          ordered.forEachIndexed { index, entry ->
            key(entry.textReplacementId) {
              if (index > 0) CardDivider(inset = 20.dp)
              CustomRow(
                entry = entry,
                order = index + 1,
                reorderState = reorderState,
                reorderEnabled = reorderEnabled,
                onEdit = {
                  if (SubscriptionService.gate(sheet, GatedAction.TextReplacement)) {
                    sheet.present {
                      TextReplacementEditSheet(model = model, editing = entry.onTextReplacement)
                    }
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
  reorderState: ReorderableColumnState<String>,
  reorderEnabled: Boolean,
  onEdit: suspend () -> Unit,
  onToggle: suspend () -> Unit,
  onReorderCommit: (movedKey: String, orderedKeys: List<String>) -> Unit,
) {
  val id = entry.textReplacementId
  val toggleScope = rememberCoroutineScope()

  Row(
    modifier = Modifier.fillMaxWidth().reorderableItem(state = reorderState, key = id),
    verticalAlignment = Alignment.CenterVertically,
  ) {
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
