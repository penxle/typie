package co.typie.screen.settings.text_replacements

import androidx.compose.animation.animateBounds
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.LookaheadScope
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.safeBottomPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.type.TextReplacementState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.result.onErr
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.SettingSwitch
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.HeaderTextAction
import co.typie.ui.component.sheet.LocalSheetHost
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private const val CUSTOM_ROW_DRAG_GUTTER_WIDTH_DP = 44

@Composable
fun TextReplacementsScreen() {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val sheetHost = LocalSheetHost.current
  val haptic = LocalHapticFeedback.current
  val model = viewModel { TextReplacementsViewModel() }
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  var isPersistingCustomReorder by remember { mutableStateOf(false) }
  val scrollState = rememberScrollState("text-replacements")

  val presetItems = model.normalizedPresetItems.sortedBy { it.order.orEmpty() }
  val smartQuoteItems = model.normalizedSmartQuoteItems
  val serverCustomItems = model.normalizedCustomItems.sortedBy { it.order.orEmpty() }
  val serverCustomItemIds =
    remember(serverCustomItems) { normalizedCustomItemIds(serverCustomItems) }

  fun openForm(editingItem: NormalizedTextReplacement? = null) {
    sheetHost.show(
      TextReplacementFormSheet(
        model = model,
        editingItem = editingItem,
        lastCustomOrder = serverCustomItems.lastOrNull()?.order,
      )
    )
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("텍스트 대치", style = AppTheme.typography.title) },
    trailing = { TopBarButton(icon = Lucide.Plus, onClick = { openForm() }) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    primaryScrollableState = scrollState,
    body = { contentPadding ->
      val reorderState =
        rememberReorderableListState(
          keys = serverCustomItemIds,
          verticalScrollableState = scrollState,
        )
      val displayCustomItems =
        remember(serverCustomItems, reorderState.displayedKeys) {
          displayCustomItems(serverCustomItems, reorderState.displayedKeys)
        }

      Box(
        modifier =
          Modifier.fillMaxSize()
            .reorderableListContainer(
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
              .safeBottomPadding(),
          verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          Text(
            text = "텍스트 대치",
            style = AppTheme.typography.display,
            modifier = Modifier.padding(top = 4.dp),
          )

          Text(
            text = "입력 중 특정 텍스트를 자동으로 변환해요.",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
          )

          TextReplacementSection(title = "기본 대치") {
            if (smartQuoteItems.isNotEmpty()) {
              TextReplacementToggleRow(
                title = "곧은따옴표를 둥근따옴표로",
                checked = model.isSmartQuoteEnabled,
                onClick = {
                  model
                    .toggleSmartQuotes(
                      items = model.normalizedItems,
                      enabled = !model.isSmartQuoteEnabled,
                    )
                    .withDefaultExceptionHandler(toast)
                },
                onCheckedChange = { next ->
                  scope.launch {
                    model
                      .toggleSmartQuotes(items = model.normalizedItems, enabled = next)
                      .withDefaultExceptionHandler(toast)
                  }
                },
              )

              if (presetItems.isNotEmpty()) {
                CardDivider()
              }
            }

            presetItems.forEachIndexed { index, item ->
              if (index > 0) {
                CardDivider()
              }

              TextReplacementPresetRow(
                item = item,
                checked = item.state == TextReplacementState.ACTIVE,
                onClick = { model.togglePreset(item).withDefaultExceptionHandler(toast) },
                onCheckedChange = {
                  scope.launch { model.togglePreset(item).withDefaultExceptionHandler(toast) }
                },
              )
            }
          }

          Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            SectionTitle(text = "사용자 대치", modifier = Modifier.padding(top = 4.dp))
            Text(
              text = "위에서부터 순서대로 먼저 매치되는 규칙이 적용돼요.",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textTertiary,
            )

            if (serverCustomItems.isEmpty()) {
              CardSurface(modifier = Modifier.fillMaxWidth()) { TextReplacementEmptyState() }
            } else {
              LookaheadScope {
                val boundsTransform = remember {
                  androidx.compose.animation.BoundsTransform { _, _ ->
                    spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium)
                  }
                }

                Column(modifier = Modifier.fillMaxWidth()) {
                  displayCustomItems.forEachIndexed { index, item ->
                    key(item.textReplacementId) {
                      val isDragging = reorderState.isDragging(item.textReplacementId)

                      TextReplacementCustomRow(
                        modifier =
                          Modifier.animateBounds(
                              lookaheadScope = this@LookaheadScope,
                              boundsTransform = boundsTransform,
                            )
                            .reorderableItem(state = reorderState, key = item.textReplacementId),
                        dragHandleModifier =
                          Modifier.reorderableDragHandle(
                            state = reorderState,
                            key = item.textReplacementId,
                            enabled = !isPersistingCustomReorder,
                            onDragStarted = {
                              haptic.performHapticFeedback(
                                HapticFeedbackType.GestureThresholdActivate
                              )
                            },
                            onDragMoved = {
                              haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
                            },
                            onDragStopped = { commit ->
                              haptic.performHapticFeedback(HapticFeedbackType.GestureEnd)
                              if (commit == null || commit.orderedKeys == serverCustomItemIds) {
                                return@reorderableDragHandle
                              }

                              val reorderOrders =
                                calculateCustomReorderOrdersFromOrderedKeys(
                                  items = serverCustomItems,
                                  orderedKeys = commit.orderedKeys,
                                  movedKey = commit.movedKey,
                                )
                                  ?: run {
                                    reorderState.resetToServerKeys(serverCustomItemIds)
                                    return@reorderableDragHandle
                                  }

                              isPersistingCustomReorder = true
                              scope.launch {
                                model
                                  .moveCustom(
                                    textReplacementId = commit.movedKey,
                                    lowerOrder = reorderOrders.lowerOrder,
                                    upperOrder = reorderOrders.upperOrder,
                                  )
                                  .withDefaultExceptionHandler(toast)
                                  .onException {
                                    reorderState.resetToServerKeys(serverCustomItemIds)
                                  }
                                isPersistingCustomReorder = false
                              }
                            },
                          ),
                        index = index,
                        item = item,
                        isDragging = isDragging,
                        isFirst = index == 0,
                        isLast = index == displayCustomItems.lastIndex,
                        onToggleChange = {
                          scope.launch {
                            model.toggleCustom(item).withDefaultExceptionHandler(toast)
                          }
                        },
                        onEditClick = { openForm(item) },
                      )
                    }
                  }
                }
              }
            }
          }

          Box(modifier = Modifier.height(72.dp))
        }
      }
    },
  )
}

@Composable
private fun TextReplacementSection(
  title: String,
  description: String? = null,
  content: @Composable ColumnScope.() -> Unit,
) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(text = title, modifier = Modifier.padding(top = 4.dp))

    if (description != null) {
      Text(
        text = description,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )
    }

    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(modifier = Modifier.fillMaxWidth(), content = content)
    }
  }
}

@Composable
private fun TextReplacementToggleRow(
  title: String,
  checked: Boolean,
  onClick: suspend () -> Unit,
  onCheckedChange: (Boolean) -> Unit,
) {
  CardRow(onClick = onClick) {
    Column(modifier = Modifier.weight(1f)) {
      Text(
        text = title,
        style = AppTheme.typography.label,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }

    SettingSwitch(checked = checked, onCheckedChange = onCheckedChange)
  }
}

@Composable
private fun TextReplacementPresetRow(
  item: NormalizedTextReplacement,
  checked: Boolean,
  onClick: suspend () -> Unit,
  onCheckedChange: (Boolean) -> Unit,
) {
  CardRow(onClick = onClick) {
    TextReplacementRuleLabel(item = item, modifier = Modifier.weight(1f))

    SettingSwitch(checked = checked, onCheckedChange = onCheckedChange)
  }
}

@Composable
private fun TextReplacementCustomRow(
  modifier: Modifier = Modifier,
  dragHandleModifier: Modifier = Modifier,
  index: Int,
  item: NormalizedTextReplacement,
  isDragging: Boolean,
  isFirst: Boolean,
  isLast: Boolean,
  onToggleChange: (Boolean) -> Unit,
  onEditClick: suspend () -> Unit,
) {
  val topStartRadius by
    animateDpAsState(
      targetValue = if (isFirst) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val topEndRadius by
    animateDpAsState(
      targetValue = if (isFirst) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val bottomStartRadius by
    animateDpAsState(
      targetValue = if (isLast) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val bottomEndRadius by
    animateDpAsState(
      targetValue = if (isLast) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val shape =
    RoundedCornerShape(
      topStart = topStartRadius,
      topEnd = topEndRadius,
      bottomStart = bottomStartRadius,
      bottomEnd = bottomEndRadius,
    )
  val density = LocalDensity.current
  val animatedScale by
    animateFloatAsState(
      targetValue = if (isDragging) 1.008f else 1f,
      animationSpec =
        if (isDragging) {
          tween(durationMillis = 120)
        } else {
          spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow)
        },
    )
  val animatedElevation by
    animateDpAsState(
      targetValue = if (isDragging) 3.dp else 0.dp,
      animationSpec =
        if (isDragging) {
          tween(durationMillis = 120)
        } else {
          spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow)
        },
    )

  CardSurface(
    modifier =
      modifier
        .fillMaxWidth()
        .graphicsLayer {
          scaleX = animatedScale
          scaleY = animatedScale
          shadowElevation = with(density) { animatedElevation.toPx() }
          this.shape = shape
          clip = false
        }
        .zIndex(if (isDragging) 1f else 0f),
    shape = shape,
    color = if (isDragging) AppTheme.colors.surfaceRaised else AppTheme.colors.surfaceDefault,
  ) {
    Column(modifier = Modifier.fillMaxWidth()) {
      if (!isFirst) {
        CardDivider(inset = 20.dp)
      }

      Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(
          modifier =
            dragHandleModifier.size(width = CUSTOM_ROW_DRAG_GUTTER_WIDTH_DP.dp, height = 56.dp),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = Lucide.GripVertical,
            modifier = Modifier.size(18.dp),
            tint = AppTheme.colors.textTertiary,
          )
        }

        InteractionScope {
          Row(
            modifier =
              Modifier.weight(1f)
                .clickable(onEditClick)
                .padding(top = 16.dp, end = 12.dp, bottom = 16.dp)
                .pressScale(0.98f),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            TextReplacementOrderBadge(order = index + 1)

            TextReplacementRuleLabel(item = item, modifier = Modifier.weight(1f))
          }
        }

        Box(modifier = Modifier.padding(start = 8.dp, end = 16.dp)) {
          SettingSwitch(
            checked = item.state == TextReplacementState.ACTIVE,
            onCheckedChange = onToggleChange,
          )
        }
      }
    }
  }
}

@Composable
private fun TextReplacementEmptyState() {
  Box(
    modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 24.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = "아직 사용자 대치 규칙이 없어요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )
  }
}

@Composable
private fun TextReplacementRegexBadge() {
  Icon(
    icon = Lucide.Regex,
    modifier = Modifier.size(16.dp),
    tint = AppTheme.colors.textOnBrandSubtle,
  )
}

@Composable
private fun TextReplacementOrderBadge(order: Int) {
  Box(
    modifier =
      Modifier.clip(RoundedCornerShape(4.dp))
        .background(AppTheme.colors.surfaceTinted)
        .padding(horizontal = 6.dp, vertical = 2.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = order.toString(),
      style = AppTheme.typography.caption.copy(fontFamily = FontFamily.Monospace),
      color = AppTheme.colors.textTertiary,
      maxLines = 1,
    )
  }
}

@Composable
private fun TextReplacementRuleLabel(
  item: NormalizedTextReplacement,
  modifier: Modifier = Modifier,
) {
  val note = item.note?.takeIf { it.isNotBlank() }

  SubcomposeLayout(modifier = modifier) { constraints ->
    val spacing = 6.dp.roundToPx()

    val trailingPlaceables =
      subcompose("trailing") {
          if (item.regex) {
            Row(
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
              TextReplacementRegexBadge()
            }
          }
        }
        .map { measurable -> measurable.measure(constraints.copy(minWidth = 0, minHeight = 0)) }

    val trailingWidth = trailingPlaceables.sumOf { it.width }
    val trailingSpacing =
      if (trailingPlaceables.size > 1) spacing * (trailingPlaceables.size - 1) else 0
    val trailingClusterWidth = trailingWidth + trailingSpacing
    val gapToTrailing = if (trailingPlaceables.isNotEmpty()) spacing else 0
    val contentMaxWidth =
      (constraints.maxWidth - trailingClusterWidth - gapToTrailing).coerceAtLeast(0)

    val contentPlaceable =
      subcompose("content") {
          if (note != null) {
            Text(
              text = note,
              style = AppTheme.typography.label,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          } else {
            Row(
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
              TextReplacementRuleToken(
                text = item.match,
                modifier = Modifier.weight(1f, fill = false),
              )
              Icon(
                icon = Lucide.ChevronRight,
                modifier = Modifier.size(14.dp),
                tint = AppTheme.colors.textTertiary,
              )
              TextReplacementRuleToken(
                text = item.substitute,
                modifier = Modifier.weight(1f, fill = false),
              )
            }
          }
        }
        .single()
        .measure(constraints.copy(minWidth = 0, minHeight = 0, maxWidth = contentMaxWidth))

    val width =
      (contentPlaceable.width + gapToTrailing + trailingClusterWidth).coerceIn(
        constraints.minWidth,
        constraints.maxWidth,
      )
    val height =
      maxOf(
          contentPlaceable.height,
          trailingPlaceables.maxOfOrNull { it.height } ?: 0,
          constraints.minHeight,
        )
        .coerceAtMost(constraints.maxHeight)

    layout(width, height) {
      val contentY = (height - contentPlaceable.height) / 2
      contentPlaceable.placeRelative(0, contentY)

      var trailingX = contentPlaceable.width + gapToTrailing
      trailingPlaceables.forEach { placeable ->
        val placeableY = (height - placeable.height) / 2
        placeable.placeRelative(trailingX, placeableY)
        trailingX += placeable.width + spacing
      }
    }
  }
}

@Composable
private fun TextReplacementRuleToken(text: String, modifier: Modifier = Modifier) {
  Box(
    modifier =
      modifier
        .clip(RoundedCornerShape(4.dp))
        .background(AppTheme.colors.surfaceTinted)
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

private fun TextReplacementFormSheet(
  model: TextReplacementsViewModel,
  editingItem: NormalizedTextReplacement?,
  lastCustomOrder: String?,
): SheetPresentation<Unit> = sheetPresentation {
  val isEditing = editingItem != null
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val form =
    remember(editingItem?.textReplacementId) {
      TextReplacementForm(scope = scope, editingItem = editingItem)
    }

  val dialog = LocalDialog.current
  var errorText by remember(editingItem?.textReplacementId) { mutableStateOf<String?>(null) }
  var isSaving by remember { mutableStateOf(false) }
  var isDeleting by remember { mutableStateOf(false) }

  LaunchedEffect(form.match.value, form.substitute.value, form.note.value, form.regex.value) {
    if (errorText != null) {
      errorText = null
    }
  }

  suspend fun submit() {
    val validationError =
      validateTextReplacementForm(
        match = form.match.value,
        substitute = form.substitute.value,
        regex = form.regex.value,
        regexValidator = model::validateRegex,
      )

    if (validationError != null) {
      errorText = validationError.message
      return
    }

    errorText = null
    isSaving = true

    model
      .saveCustomRule(
        editingItem = editingItem,
        match = form.match.value,
        substitute = form.substitute.value,
        regex = form.regex.value,
        note = form.note.value,
        lastOrder = lastCustomOrder,
      )
      .withDefaultExceptionHandler(toast)
      .onOk { complete(Unit) }
      .onErr { error ->
        when (error) {
          is SaveRuleError.ValidationFailed -> {
            errorText = error.message
          }
        }
      }

    isSaving = false
  }

  SheetLayout(
    header = {
      ActionHeader(
        title = if (isEditing) "대치 규칙 수정" else "대치 규칙 추가",
        leading = {
          HeaderTextAction(
            text = "취소",
            color = AppTheme.colors.brand,
            enabled = !isSaving && !isDeleting,
            onClick = { dismiss() },
          )
        },
        trailing = {
          HeaderTextAction(
            text = "저장",
            color = AppTheme.colors.brand,
            textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
            enabled = !isDeleting,
            loading = isSaving,
            onClick = { submit() },
          )
        },
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
      Column {
        TextField(
          field = form.match,
          label = "찾을 텍스트",
          labelPosition = LabelPosition.Internal,
          placeholder = "찾을 텍스트를 입력해 주세요",
        )

        TextField(
          field = form.substitute,
          label = "삽입할 텍스트",
          labelPosition = LabelPosition.Internal,
          placeholder = "삽입할 텍스트를 입력해 주세요",
        )

        TextField(
          field = form.note,
          label = "설명 (선택)",
          labelPosition = LabelPosition.Internal,
          placeholder = "설명 (선택)",
        )

        TextReplacementRegexRow(
          checked = form.regex.value,
          onClick = { form.regex.setValue(!form.regex.value) },
          onCheckedChange = { next -> form.regex.setValue(next) },
        )
      }

      if (errorText != null) {
        Text(
          text = errorText!!,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.danger,
          modifier = Modifier.padding(horizontal = 8.dp),
        )
      }

      if (isEditing) {
        TextReplacementFormTextAction(
          text = if (isDeleting) "삭제 중..." else "이 규칙 삭제하기",
          enabled = !isSaving && !isDeleting,
          color = AppTheme.colors.danger,
          onClick = {
            val result =
              dialog.confirm(
                title = "대치 규칙 삭제",
                message = "\"${replacementPreview(requireNotNull(editingItem))}\" 규칙을 삭제하시겠어요?",
                confirmText = "삭제",
                confirmIsDestructive = true,
              )
            if (result is DialogResult.Resolved) {
              isDeleting = true
              model
                .deleteCustom(requireNotNull(editingItem))
                .withDefaultExceptionHandler(toast)
                .onOk { complete(Unit) }
              isDeleting = false
            }
          },
        )
      }
    }
  }
}

@Composable
private fun TextReplacementFormTextAction(
  text: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  color: androidx.compose.ui.graphics.Color = AppTheme.colors.textPrimary,
) {
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)

  InteractionScope {
    Box(
      modifier =
        modifier
          .fillMaxWidth()
          .clickable(enabled = enabled, onClick = onClick)
          .pressScale(0.97f)
          .padding(horizontal = 4.dp, vertical = 8.dp)
          .alpha(alpha),
      contentAlignment = Alignment.Center,
    ) {
      Text(text = text, style = AppTheme.typography.action, color = color)
    }
  }
}

@Composable
private fun TextReplacementRegexRow(
  checked: Boolean,
  onClick: suspend () -> Unit,
  onCheckedChange: (Boolean) -> Unit,
  modifier: Modifier = Modifier,
) {
  InteractionScope {
    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .clip(RoundedCornerShape(12.dp))
          .clickable(onClick)
          .pressScale()
          .padding(horizontal = 16.dp, vertical = 4.dp),
      horizontalArrangement = Arrangement.spacedBy(12.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(
        icon = Lucide.Regex,
        modifier = Modifier.size(18.dp),
        tint = AppTheme.colors.textTertiary,
      )

      Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(text = "정규식", style = AppTheme.typography.label)
        Text(
          text = "찾을 텍스트를 정규식 패턴으로 해석합니다.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )
      }

      SettingSwitch(checked = checked, onCheckedChange = onCheckedChange)
    }
  }
}

private fun replacementPreview(item: NormalizedTextReplacement): String {
  return "${item.match} → ${item.substitute}"
}

private fun displayCustomItems(
  serverItems: List<NormalizedTextReplacement>,
  optimisticOrder: List<String>?,
): List<NormalizedTextReplacement> {
  val currentOptimisticOrder = optimisticOrder ?: return serverItems
  val itemsById = serverItems.associateBy { it.textReplacementId }
  val orderedItems = currentOptimisticOrder.mapNotNull(itemsById::get)
  if (orderedItems.size == serverItems.size) {
    return orderedItems
  }

  val orderedIds = orderedItems.mapTo(mutableSetOf()) { it.textReplacementId }
  return orderedItems + serverItems.filterNot { it.textReplacementId in orderedIds }
}
