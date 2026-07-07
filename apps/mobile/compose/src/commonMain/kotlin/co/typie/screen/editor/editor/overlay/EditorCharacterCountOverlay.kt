package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.ffi.CharacterCounts
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState
import co.typie.icons.Lucide
import co.typie.storage.Preference
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.drop

private const val CharacterCountDebounceMs = 150L

// Legacy fade values (native_editor_floating_fade.dart): typing/scroll fades the widget out,
// and it fades back in after 1.5s of inactivity.
private const val WidgetFadeIdleMs = 1500L
private const val WidgetFadeAnimMs = 200
private const val WidgetFadedAlpha = 0.05f
private val CharacterCountShape = AppShapes.rounded(16.dp)

/**
 * Floating widget that shows the document character count over the editor. Visibility is controlled
 * by [Preference.characterCountFloatingEnabled]; the drag position is persisted as a relative
 * fraction. Composed from [CharacterCountFloatingState] (position/expand), [collapsedLabel]/
 * [expandedRows] (display), and [collectDebouncedCharacterCounts] (refresh policy).
 *
 * [visibleArea] geometry is in dp; the floating state works in px so it matches the units of
 * [onSizeChanged], drag deltas, and the applied [offset].
 *
 * Auto-fade mirrors the legacy widget: typing (cursor/selection movement) or scrolling fades the
 * widget out so it does not cover the text being edited, and it fades back in once idle. Tapping a
 * faded widget only wakes it; tapping a visible one toggles the expanded rows.
 *
 * Must be hosted inside a Box-like scope whose size matches the editor viewport.
 */
@Composable
internal fun EditorCharacterCountOverlay(
  editor: Editor?,
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  modifier: Modifier = Modifier,
) {
  if (!Preference.characterCountFloatingEnabled || editor == null) {
    return
  }

  val density = LocalDensity.current

  val state = remember {
    CharacterCountFloatingState(
      relativeX = Preference.characterCountFloatingPositionX.toFloat(),
      relativeY = Preference.characterCountFloatingPositionY.toFloat(),
      persist = { x, y ->
        Preference.characterCountFloatingPositionX = x.toDouble()
        Preference.characterCountFloatingPositionY = y.toDouble()
      },
    )
  }

  var counts by remember { mutableStateOf<CharacterCounts?>(null) }
  var widgetSize by remember { mutableStateOf(0 to 0) }

  // Recompute character counts only when the document content changes (documentRevision), debounced
  // to avoid per-keystroke work — mirroring the legacy doc-dirty + debounce policy. Cursor- or
  // selection-only ticks bump [EditorState.version] but not documentRevision, so they never
  // refetch.
  LaunchedEffect(editor) {
    collectDebouncedCharacterCounts(
      versions = snapshotFlow { editor.state.documentRevision },
      debounceMillis = CharacterCountDebounceMs,
      fetch = { editor.characterCounts()?.let { counts = it } },
    )
  }

  // Map the relative position onto the current viewport whenever any input geometry changes,
  // converting the dp-based visible area into px for the floating state.
  LaunchedEffect(visibleArea, widgetSize, density) {
    with(density) {
      state.onViewportMeasured(
        width = visibleArea.viewport.width.dp.toPx(),
        height = visibleArea.viewport.height.dp.toPx(),
        widgetWidth = widgetSize.first.toFloat(),
        widgetHeight = widgetSize.second.toFloat(),
        topOcclusion = visibleArea.topOcclusion.dp.toPx(),
        bottomOcclusion = visibleArea.bottomOcclusion.dp.toPx(),
      )
    }
  }

  val autoFade = Preference.widgetAutoFadeEnabled
  var faded by remember { mutableStateOf(false) }
  var fadeOutRequest by remember { mutableIntStateOf(0) }
  var wakeRequest by remember { mutableIntStateOf(0) }

  // Typing: the cursor or selection moving means the user is editing, so get out of the way.
  LaunchedEffect(editor, autoFade) {
    if (!autoFade) return@LaunchedEffect
    snapshotFlow { editor.state.cursor?.caret to editor.state.selection }
      .drop(1)
      .collect { fadeOutRequest += 1 }
  }

  // Scrolling fades the widget out the same way.
  LaunchedEffect(viewportState, autoFade) {
    if (!autoFade) return@LaunchedEffect
    snapshotFlow { viewportState.scrollOffset }.drop(1).collect { fadeOutRequest += 1 }
  }

  LaunchedEffect(fadeOutRequest, autoFade) {
    if (!autoFade) {
      faded = false
      return@LaunchedEffect
    }
    if (fadeOutRequest == 0) return@LaunchedEffect
    faded = true
    delay(WidgetFadeIdleMs)
    faded = false
  }
  LaunchedEffect(wakeRequest) {
    if (wakeRequest > 0) faded = false
  }

  val alpha by
    animateFloatAsState(
      targetValue = if (faded) WidgetFadedAlpha else 1f,
      animationSpec = tween(WidgetFadeAnimMs),
      label = "character-count-overlay-alpha",
    )

  val chevronRotation by
    animateFloatAsState(
      targetValue = if (state.expanded) 90f else 0f,
      animationSpec = tween(WidgetFadeAnimMs),
      label = "character-count-overlay-chevron",
    )

  val current = counts ?: return

  Box(
    modifier =
      modifier
        .offset { IntOffset(state.offsetX.toInt(), state.offsetY.toInt()) }
        .onSizeChanged { widgetSize = it.width to it.height }
        .graphicsLayer { this.alpha = alpha }
        .pointerInput(Unit) {
          detectDragGestures(
            onDragStart = {
              wakeRequest += 1
              state.onDragStart()
            },
            onDragEnd = { state.onDragEnd() },
            onDragCancel = { state.onDragEnd() },
          ) { change, dragAmount ->
            change.consume()
            state.onDrag(dragAmount.x, dragAmount.y)
          }
        }
        .pointerInput(Unit) {
          detectTapGestures {
            // Legacy: tapping a faded widget only wakes it, without toggling the expanded rows.
            val wasFaded = faded
            wakeRequest += 1
            if (!wasFaded) {
              state.toggleExpanded()
            }
          }
        }
        .clip(CharacterCountShape)
        .border(1.dp, AppTheme.colors.borderDefault, CharacterCountShape)
        .background(AppTheme.colors.surfaceDefault.copy(alpha = 0.95f), CharacterCountShape)
        .padding(horizontal = 14.dp, vertical = 8.dp)
  ) {
    // The with-whitespace count stays visible in the header and expanding only appends the detail
    // rows below it. The chevron hints that the header is expandable, rotating to point down when
    // the detail rows are shown.
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
      Row(
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Text(
          text = current.collapsedLabel(),
          style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W500),
          color = AppTheme.colors.textDefault,
        )
        Icon(
          icon = Lucide.ChevronRight,
          modifier = Modifier.size(14.dp).rotate(chevronRotation),
          tint = AppTheme.colors.textHint,
        )
      }

      if (state.expanded) {
        current.expandedRows().forEach { (label, value) ->
          Row(
            modifier = Modifier.width(160.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
          ) {
            Text(
              text = label,
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
            )
            Text(
              text = value,
              style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W500),
              color = AppTheme.colors.textDefault,
            )
          }
        }
      }
    }
  }
}
