package co.typie.domain.note

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.BringIntoViewSpec
import androidx.compose.foundation.gestures.LocalBringIntoViewSpec
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.displayTitle
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.rememberTextInputState
import co.typie.ext.textInputFocusable
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.graphql.type.NoteStatus
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.close
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.abs

private val NoteCardShape = AppShapes.rounded(AppShapes.md)
private val NoteActionButtonSize = 24.dp
private val NoteStatusHitTargetSize = 28.dp
private val NoteGripHitTargetSize = 28.dp
private val NoteColorDotHitTargetWidth = 20.dp
private val NoteColorDotHitTargetHeight = 24.dp
private const val NoteExpandAnimationDurationMillis = 220

// NOTE: iOS에서 마지막 노트의 텍스트 필드에 포커스하면 scroll offset 0까지 천천히 스크롤되는 버그의 workaround
@OptIn(ExperimentalFoundationApi::class)
private val NoteEditorBringIntoViewSpec =
  object : BringIntoViewSpec {
    override fun calculateScrollDistance(offset: Float, size: Float, containerSize: Float): Float {
      val trailingEdge = offset + size
      val leadingEdge = offset
      val distance =
        when {
          leadingEdge >= 0f && trailingEdge <= containerSize -> 0f
          leadingEdge < 0f && trailingEdge > containerSize -> 0f
          abs(leadingEdge) < abs(trailingEdge - containerSize) -> leadingEdge
          else -> trailingEdge - containerSize
        }

      return distance.coerceAtLeast(0f)
    }
  }

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun NoteCard(
  note: NoteCard_note,
  expanded: Boolean,
  isDragging: Boolean,
  content: String,
  isSaving: Boolean,
  colorOption: NoteColorOption,
  dragHandleModifier: Modifier,
  contentEditable: Boolean = true,
  modifier: Modifier = Modifier,
  onExpand: () -> Unit,
  onCollapse: () -> Unit,
  onContentChange: (String) -> Unit,
  onBlur: () -> Unit,
  onToggleStatus: () -> Unit,
  onColorChange: (String) -> Unit,
  onAddEntity: () -> Unit,
  onEntityClick: (NoteLinkedEntity_entity) -> Unit,
  onDelete: () -> Unit,
  noteColorOptions: List<NoteColorOption>,
) {
  val containerColor by
    animateColorAsState(
      targetValue = if (expanded) AppTheme.colors.surfaceDefault else AppTheme.colors.surfaceInset,
      animationSpec = tween(durationMillis = NoteExpandAnimationDurationMillis),
    )
  val borderColor by
    animateColorAsState(
      targetValue = if (expanded) AppTheme.colors.borderDefault else AppTheme.colors.borderHairline,
      animationSpec = tween(durationMillis = NoteExpandAnimationDurationMillis),
    )
  val baseShadowElevation by
    animateDpAsState(
      targetValue = if (expanded) 8.dp else 3.dp,
      animationSpec = tween(durationMillis = NoteExpandAnimationDurationMillis),
    )
  val dragShadowElevation by
    animateDpAsState(
      targetValue = if (isDragging) 16.dp else 0.dp,
      animationSpec =
        if (isDragging) tween(durationMillis = 120)
        else spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow),
    )
  val dragScale by
    animateFloatAsState(
      targetValue = if (isDragging) 1.014f else 1f,
      animationSpec =
        if (isDragging) tween(durationMillis = 120)
        else spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow),
    )
  val dragRotation by
    animateFloatAsState(
      targetValue = if (isDragging) -1.5f else 0f,
      animationSpec =
        if (isDragging) tween(durationMillis = 120)
        else spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow),
    )
  val cardModifier =
    modifier
      .fillMaxWidth()
      .shadow(
        AppTheme.shadows.lg,
        NoteCardShape,
        alpha = { ((baseShadowElevation + dragShadowElevation).value / 24f).coerceIn(0f, 1f) },
      )
      .clip(NoteCardShape)
      .graphicsLayer {
        scaleX = dragScale
        scaleY = dragScale
        rotationZ = dragRotation
      }
      .background(containerColor, NoteCardShape)
      .border(1.dp, borderColor, NoteCardShape)

  Column(modifier = cardModifier) {
    AnimatedContent(
      targetState = expanded,
      modifier = Modifier.fillMaxWidth(),
      transitionSpec = {
        (fadeIn(animationSpec = tween(durationMillis = 180, delayMillis = 40)) togetherWith
            fadeOut(animationSpec = tween(durationMillis = 120)))
          .using(
            SizeTransform(clip = false) { _, _ ->
              tween(durationMillis = NoteExpandAnimationDurationMillis)
            }
          )
      },
      label = "note-card-content",
    ) { isExpanded ->
      if (isExpanded) {
        NoteExpandedContent(
          note = note,
          content = content,
          isSaving = isSaving,
          colorOption = colorOption,
          dragHandleModifier = dragHandleModifier,
          contentEditable = contentEditable,
          noteColorOptions = noteColorOptions,
          onCollapse = onCollapse,
          onContentChange = onContentChange,
          onBlur = onBlur,
          onToggleStatus = onToggleStatus,
          onColorChange = onColorChange,
          onAddEntity = onAddEntity,
          onEntityClick = onEntityClick,
          onDelete = onDelete,
        )
      } else {
        NoteCollapsedContent(
          note = note,
          colorOption = colorOption,
          dragHandleModifier = dragHandleModifier,
          onExpand = onExpand,
          onToggleStatus = onToggleStatus,
        )
      }
    }
  }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun NoteExpandedContent(
  note: NoteCard_note,
  content: String,
  isSaving: Boolean,
  colorOption: NoteColorOption,
  dragHandleModifier: Modifier,
  contentEditable: Boolean,
  noteColorOptions: List<NoteColorOption>,
  onCollapse: () -> Unit,
  onContentChange: (String) -> Unit,
  onBlur: () -> Unit,
  onToggleStatus: () -> Unit,
  onColorChange: (String) -> Unit,
  onAddEntity: () -> Unit,
  onEntityClick: (NoteLinkedEntity_entity) -> Unit,
  onDelete: () -> Unit,
) {
  Column(modifier = Modifier.fillMaxWidth()) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.spacedBy(2.dp),
      verticalAlignment = Alignment.Top,
    ) {
      NoteCardLeadingContent(
        resolved = note.status == NoteStatus.RESOLVED,
        colorOption = colorOption,
        dragHandleModifier = dragHandleModifier,
        onToggleStatus = onToggleStatus,
      )

      Row(
        modifier = Modifier.weight(1f).padding(end = 12.dp, top = 12.dp, bottom = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.Top,
      ) {
        Column(modifier = Modifier.weight(1f).padding(top = 2.dp)) {
          NoteContentEditor(
            content = content,
            onValueChange = onContentChange,
            onBlur = onBlur,
            readOnly = !contentEditable,
          )
        }

        NoteActionIconButton(icon = Lucide.Minimize2, onClick = onCollapse)
      }
    }

    CardDivider(inset = 12.dp)

    Column(
      modifier =
        Modifier.fillMaxWidth().padding(start = 16.dp, end = 12.dp, top = 10.dp, bottom = 10.dp),
      verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
      ) {
        NoteColorPalette(
          noteColorOptions = noteColorOptions,
          selectedColor = note.color,
          onColorChange = onColorChange,
        )

        Row(
          horizontalArrangement = Arrangement.spacedBy(8.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          if (isSaving) {
            Text("저장 중...", style = AppTheme.typography.micro, color = AppTheme.colors.textHint)
          }

          NoteCardMenuPopover(
            status = note.status,
            onAddEntity = onAddEntity,
            onToggleStatus = onToggleStatus,
            onDelete = onDelete,
          )
        }
      }

      val linkedEntities = note.entities.map { it.noteLinkedEntity_entity }
      if (linkedEntities.isNotEmpty()) {
        FlowRow(
          horizontalArrangement = Arrangement.spacedBy(8.dp),
          verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          linkedEntities.forEach { entity ->
            NoteLinkedEntityChip(linkedEntity = entity, onClick = { onEntityClick(entity) })
          }
        }
      }
    }
  }
}

@Composable
private fun NoteCollapsedContent(
  note: NoteCard_note,
  colorOption: NoteColorOption,
  dragHandleModifier: Modifier,
  onExpand: () -> Unit,
  onToggleStatus: () -> Unit,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(2.dp),
    verticalAlignment = Alignment.Top,
  ) {
    NoteCardLeadingContent(
      resolved = note.status == NoteStatus.RESOLVED,
      colorOption = colorOption,
      dragHandleModifier = dragHandleModifier,
      onToggleStatus = onToggleStatus,
    )

    InteractionScope {
      Row(
        modifier =
          Modifier.weight(1f)
            .padding(end = 12.dp, top = 12.dp, bottom = 12.dp)
            .clip(AppShapes.rounded(AppShapes.md))
            .clickable { onExpand() }
            .pressScale(0.985f),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.Top,
      ) {
        Column(
          modifier = Modifier.weight(1f).padding(top = 2.dp),
          verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          Text(
            text = note.previewText(),
            modifier = Modifier.fillMaxWidth(),
            style =
              AppTheme.typography.body.copy(
                textDecoration =
                  if (note.status == NoteStatus.RESOLVED) TextDecoration.LineThrough
                  else TextDecoration.None
              ),
            color =
              when {
                note.content.isBlank() -> AppTheme.colors.textHint
                note.status == NoteStatus.RESOLVED -> AppTheme.colors.textHint
                else -> AppTheme.colors.textDefault
              },
            maxLines = 3,
            overflow = TextOverflow.Ellipsis,
          )

          NoteCollapsedMetaRow(note = note)
        }

        Box(modifier = Modifier.size(NoteActionButtonSize), contentAlignment = Alignment.Center) {
          Icon(
            icon = Lucide.Maximize2,
            modifier = Modifier.size(15.dp),
            tint = AppTheme.colors.textMuted,
          )
        }
      }
    }
  }
}

@Composable
private fun NoteCardLeadingContent(
  resolved: Boolean,
  colorOption: NoteColorOption,
  dragHandleModifier: Modifier,
  onToggleStatus: () -> Unit,
) {
  Column(
    modifier = Modifier.padding(10.dp).width(NoteGripHitTargetSize),
    horizontalAlignment = Alignment.CenterHorizontally,
  ) {
    NoteStatusToggleButton(resolved = resolved, colorOption = colorOption, onClick = onToggleStatus)

    Box(
      modifier = dragHandleModifier.size(NoteGripHitTargetSize),
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = Lucide.GripVertical,
        modifier = Modifier.size(16.dp),
        tint = AppTheme.colors.textHint,
      )
    }
  }
}

@Composable
private fun NoteCardMenuPopover(
  status: NoteStatus,
  onAddEntity: () -> Unit,
  onToggleStatus: () -> Unit,
  onDelete: () -> Unit,
) {
  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { NoteActionIconAnchor(icon = Lucide.Ellipsis) },
    pane = {
      val row: @Composable (IconData, String, Color) -> Unit = { icon, text, color ->
        Row(
          modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
          verticalAlignment = Alignment.CenterVertically,
          horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          Icon(icon = icon, modifier = Modifier.size(18.dp), tint = color)
          Text(text = text, style = AppTheme.typography.action, color = color)
        }
      }
      val items =
        listOf(
          PopoverListItem(
            content = { row(Lucide.Link, "연결 추가", AppTheme.colors.textDefault) },
            onSelected = {
              close()
              onAddEntity()
            },
          ),
          PopoverListItem(
            content = {
              row(
                if (status == NoteStatus.RESOLVED) Lucide.Circle else Lucide.CircleCheck,
                if (status == NoteStatus.RESOLVED) "미완료로 표시" else "완료로 표시",
                AppTheme.colors.textDefault,
              )
            },
            onSelected = {
              close()
              onToggleStatus()
            },
          ),
          PopoverListItem(
            content = { row(Lucide.Trash2, "삭제", AppTheme.colors.danger) },
            onSelected = {
              close()
              onDelete()
            },
          ),
        )

      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        PopoverList(items = items)
      }
    },
  )
}

@Composable
private fun NoteStatusToggleButton(
  resolved: Boolean,
  colorOption: NoteColorOption,
  onClick: () -> Unit,
) {
  val skeleton = LocalSkeleton.current

  if (skeleton.enabled) {
    Skeleton.Bone(modifier = Modifier.size(NoteStatusHitTargetSize), shape = AppShapes.circle) {}
    return
  }

  InteractionScope {
    Box(
      modifier = Modifier.size(NoteStatusHitTargetSize).clickable { onClick() }.pressScale(0.95f),
      contentAlignment = Alignment.Center,
    ) {
      Box(
        modifier =
          Modifier.size(18.dp)
            .clip(AppShapes.circle)
            .background(if (resolved) colorOption.stroke else Color.Transparent, AppShapes.circle)
            .border(
              width = if (resolved) 0.dp else 2.dp,
              color = colorOption.stroke,
              shape = AppShapes.circle,
            ),
        contentAlignment = Alignment.Center,
      ) {
        if (resolved) {
          Icon(
            icon = Lucide.Check,
            modifier = Modifier.size(11.dp),
            tint = AppTheme.colors.surfaceDefault,
          )
        }
      }
    }
  }
}

@Composable
private fun NoteActionIconButton(
  icon: IconData,
  tint: Color = AppTheme.colors.textMuted,
  onClick: () -> Unit,
) {
  InteractionScope {
    Box(
      modifier =
        Modifier.size(NoteActionButtonSize)
          .clip(AppShapes.rounded(AppShapes.sm))
          .clickable { onClick() }
          .pressScale(0.95f),
      contentAlignment = Alignment.Center,
    ) {
      Icon(icon = icon, modifier = Modifier.size(15.dp), tint = tint)
    }
  }
}

@Composable
private fun NoteActionIconAnchor(icon: IconData, tint: Color = AppTheme.colors.textMuted) {
  Box(modifier = Modifier.size(NoteActionButtonSize), contentAlignment = Alignment.Center) {
    Icon(icon = icon, modifier = Modifier.size(15.dp), tint = tint)
  }
}

@Composable
private fun NoteColorPalette(
  noteColorOptions: List<NoteColorOption>,
  selectedColor: String,
  onColorChange: (String) -> Unit,
) {
  val density = LocalDensity.current
  val haptic = LocalHapticFeedback.current
  val selectedColorState = rememberUpdatedState(selectedColor)
  val onColorChangeState = rememberUpdatedState(onColorChange)
  val hapticState = rememberUpdatedState(haptic)
  val optionWidthPx = with(density) { NoteColorDotHitTargetWidth.toPx() }

  Row(
    modifier =
      Modifier.height(NoteColorDotHitTargetHeight).pointerInput(noteColorOptions, optionWidthPx) {
        awaitEachGesture {
          val down = awaitFirstDown(requireUnconsumed = false)
          var lastPointerX = down.position.x
          var lastSelectedColor = selectedColorState.value

          fun selectColorAt(x: Float, fromPan: Boolean) {
            val index = (x / optionWidthPx).toInt().coerceIn(0, noteColorOptions.lastIndex)
            val nextColor = noteColorOptions[index].value
            if (nextColor == lastSelectedColor) {
              return
            }

            lastSelectedColor = nextColor
            if (fromPan) {
              hapticState.value.performHapticFeedback(HapticFeedbackType.TextHandleMove)
            }
            onColorChangeState.value(nextColor)
          }

          selectColorAt(down.position.x, fromPan = false)

          while (true) {
            val event = awaitPointerEvent()
            val change = event.changes.firstOrNull { it.id == down.id } ?: break
            if (!change.pressed) {
              break
            }

            if (change.position.x != lastPointerX) {
              selectColorAt(change.position.x, fromPan = true)
              lastPointerX = change.position.x
            }
            change.consume()
          }
        }
      },
    verticalAlignment = Alignment.CenterVertically,
  ) {
    noteColorOptions.forEach { option ->
      NoteColorDot(
        option = option,
        selected = option.value == selectedColor,
        onClick = { onColorChange(option.value) },
      )
    }
  }
}

@Composable
private fun NoteCollapsedMetaRow(note: NoteCard_note) {
  val meta = buildCollapsedMeta(note.entities.map { it.noteLinkedEntity_entity })
  val density = LocalDensity.current
  val spacingPx = with(density) { 6.dp.roundToPx() }
  val mutedCaptionStyle = AppTheme.typography.caption.copy(color = AppTheme.colors.textHint)
  val showSeparator = meta.visibleEntities.isNotEmpty() || meta.overflowCount > 0

  SubcomposeLayout(modifier = Modifier.fillMaxWidth()) { constraints ->
    val looseConstraints = constraints.copy(minWidth = 0, minHeight = 0)
    val timePlaceable =
      subcompose("time") {
          Text(
            text = note.updatedAt.timeAgo(),
            style = mutedCaptionStyle,
            maxLines = 1,
            softWrap = false,
            overflow = TextOverflow.Clip,
          )
        }
        .single()
        .measure(looseConstraints)

    val overflowPlaceable =
      if (meta.overflowCount > 0) {
        subcompose("overflow") {
            Text(
              text = "+${meta.overflowCount}",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textHint,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          }
          .single()
          .measure(looseConstraints)
      } else {
        null
      }

    val separatorPlaceable =
      if (showSeparator) {
        subcompose("separator") {
            Text("·", style = AppTheme.typography.caption, color = AppTheme.colors.textHint)
          }
          .single()
          .measure(looseConstraints)
      } else {
        null
      }

    val trailingWidth =
      buildList {
        overflowPlaceable?.let { add(it.width) }
        separatorPlaceable?.let { add(it.width) }
        add(timePlaceable.width)
      }
        .sum() +
        spacingPx *
          (buildList {
            overflowPlaceable?.let { add(Unit) }
            separatorPlaceable?.let { add(Unit) }
          }
            .size)

    val chipMaxWidth =
      (constraints.maxWidth -
          trailingWidth -
          if (meta.visibleEntities.isNotEmpty()) spacingPx else 0)
        .coerceAtLeast(0)

    val chipPlaceable =
      meta.visibleEntities.firstOrNull()?.let { entity ->
        subcompose("chip") { NoteLinkedEntityChip(linkedEntity = entity) }
          .single()
          .measure(looseConstraints.copy(maxWidth = chipMaxWidth))
      }

    val placeables =
      listOfNotNull(chipPlaceable, overflowPlaceable, separatorPlaceable, timePlaceable)
    val height = maxOf(constraints.minHeight, placeables.maxOfOrNull { it.height } ?: 0)

    layout(width = constraints.maxWidth, height = height) {
      var x = 0

      fun placeWithSpacing(placeable: androidx.compose.ui.layout.Placeable?) {
        if (placeable == null) return
        placeable.placeRelative(x = x, y = (height - placeable.height) / 2)
        x += placeable.width + spacingPx
      }

      placeWithSpacing(chipPlaceable)
      placeWithSpacing(overflowPlaceable)
      placeWithSpacing(separatorPlaceable)
      timePlaceable.placeRelative(x = x, y = (height - timePlaceable.height) / 2)
    }
  }
}

@Composable
private fun NoteLinkedEntityChip(
  linkedEntity: NoteLinkedEntity_entity,
  modifier: Modifier = Modifier,
  onClick: (() -> Unit)? = null,
) {
  val entity = linkedEntity.entityRow_entity

  @Composable
  fun ChipRow(chipModifier: Modifier) {
    Row(
      modifier =
        chipModifier
          .clip(AppShapes.rounded(AppShapes.sm))
          .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.sm))
          .padding(horizontal = 6.dp, vertical = 4.dp),
      horizontalArrangement = Arrangement.spacedBy(4.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      EntityIcon(entity = linkedEntity.entityIcon_entity, modifier = Modifier.size(12.dp))

      Text(
        text = entity.displayTitle(),
        modifier = Modifier.weight(1f, fill = false),
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W500),
        color = AppTheme.colors.textMuted,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }

  if (onClick == null) {
    ChipRow(modifier)
  } else {
    InteractionScope { ChipRow(modifier.clickable { onClick() }.pressScale(0.97f)) }
  }
}

@Composable
private fun NoteColorDot(option: NoteColorOption, selected: Boolean, onClick: () -> Unit) {
  InteractionScope {
    Box(
      modifier =
        Modifier.size(width = NoteColorDotHitTargetWidth, height = NoteColorDotHitTargetHeight)
          .clickable { onClick() }
          .pressScale(0.96f),
      contentAlignment = Alignment.Center,
    ) {
      Box(
        modifier =
          Modifier.size(14.dp)
            .clip(AppShapes.circle)
            .background(if (selected) option.stroke else Color.Transparent, AppShapes.circle)
            .border(
              width = if (selected) 0.dp else 1.5.dp,
              color = option.stroke,
              shape = AppShapes.circle,
            )
      )
    }
  }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun NoteContentEditor(
  content: String,
  onValueChange: (String) -> Unit,
  onBlur: () -> Unit,
  readOnly: Boolean,
) {
  val focusManager = LocalFocusManager.current
  val textInputState =
    rememberTextInputState(
      value = content,
      onValueChange = onValueChange,
      onDismiss = { focusManager.clearFocus() },
    )

  CompositionLocalProvider(LocalBringIntoViewSpec provides NoteEditorBringIntoViewSpec) {
    BasicTextField(
      value = textInputState.value,
      onValueChange = textInputState::onValueChange,
      modifier =
        Modifier.fillMaxWidth().defaultMinSize(minHeight = 90.dp).textInputFocusable(
          textInputState
        ) { state ->
          if (!state.isFocused) {
            onBlur()
          }
        },
      readOnly = readOnly,
      textStyle = AppTheme.typography.body.copy(color = AppTheme.colors.textDefault),
      keyboardOptions =
        KeyboardOptions(
          capitalization = KeyboardCapitalization.Sentences,
          imeAction = ImeAction.Default,
        ),
      cursorBrush = SolidColor(AppTheme.colors.textDefault),
      decorationBox = { innerTextField ->
        Box(modifier = Modifier.fillMaxWidth()) {
          if (textInputState.value.text.isEmpty()) {
            Text(
              text = "내용을 입력하세요",
              style = AppTheme.typography.body,
              color = AppTheme.colors.textHint,
            )
          }

          innerTextField()
        }
      },
    )
  }
}

@Composable
internal fun rememberNoteColorOptions(): List<NoteColorOption> {
  val palette = AppTheme.colors.palette

  return remember(palette) {
    listOf(
      NoteColorOption("gray", "그레이", palette.gray),
      NoteColorOption("red", "레드", palette.red),
      NoteColorOption("orange", "오렌지", palette.orange),
      NoteColorOption("yellow", "옐로", palette.yellow),
      NoteColorOption("green", "그린", palette.green),
      NoteColorOption("blue", "블루", palette.blue),
      NoteColorOption("purple", "퍼플", palette.purple),
    )
  }
}

internal fun List<NoteColorOption>.resolve(value: String): NoteColorOption {
  return firstOrNull { it.value == value } ?: first()
}

private fun NoteCard_note.previewText(): String {
  val collapsedPreview = content.trim()
  if (collapsedPreview.isEmpty()) {
    return "(내용 없음)"
  }

  val lines = collapsedPreview.lines()
  if (lines.size <= 3) {
    return collapsedPreview
  }

  val visibleLines = lines.take(3).toMutableList()
  val lastLineIndex = visibleLines.lastIndex
  val trimmedLastLine = visibleLines[lastLineIndex].trimEnd()
  visibleLines[lastLineIndex] =
    when {
      trimmedLastLine.endsWith("…") -> trimmedLastLine
      trimmedLastLine.isEmpty() -> "…"
      else -> "$trimmedLastLine…"
    }
  return visibleLines.joinToString("\n")
}

internal data class NoteCollapsedMeta(
  val visibleEntities: List<NoteLinkedEntity_entity>,
  val overflowCount: Int,
)

internal fun buildCollapsedMeta(
  entities: List<NoteLinkedEntity_entity>,
  maxVisible: Int = 1,
): NoteCollapsedMeta {
  val safeMaxVisible = maxVisible.coerceAtLeast(0)
  return NoteCollapsedMeta(
    visibleEntities = entities.take(safeMaxVisible),
    overflowCount = (entities.size - safeMaxVisible).coerceAtLeast(0),
  )
}

internal data class NoteColorOption(val value: String, val label: String, val stroke: Color)
