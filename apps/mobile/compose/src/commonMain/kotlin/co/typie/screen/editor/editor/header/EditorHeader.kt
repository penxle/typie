package co.typie.screen.editor.editor.header

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.ext.TextInputState
import co.typie.ext.rememberTextInputState
import co.typie.ext.textInputFocusable
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.SkeletonTextBone
import co.typie.ui.theme.AppTheme

private val TitleTopPadding = 12.dp
private val TitleBlockSpacing = 40.dp
private val TitleBetweenSpacing = 8.dp
private val SubtitleDividerWidth = 120.dp
private const val EditorHeaderContentInset = 20f
private const val EditorHeaderMinContentWidth = 320f
private const val EditorHeaderMinScale = 0.75f
private const val EditorHeaderViewportGap = 20f

internal data class EditorHeaderGeometry(
  val contentLeft: Float,
  val contentRight: Float,
  val fieldWidth: Float,
  val maxScrollOffset: Float,
) {
  fun resolveFieldScreenLeft(scrollOffsetX: Float): Float {
    val resolvedScroll =
      (scrollOffsetX.takeIf { it.isFinite() } ?: 0f).coerceIn(0f, maxScrollOffset)
    val fieldLeft =
      (resolvedScroll + EditorHeaderViewportGap).coerceIn(
        contentLeft,
        maxOf(contentLeft, contentRight - fieldWidth),
      )
    return fieldLeft - resolvedScroll
  }
}

internal fun resolveEditorHeaderGeometry(
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
  bodyTrackWidth: Float,
  displayZoom: Float,
): EditorHeaderGeometry? {
  if (
    !viewportWidth.isFinite() ||
      viewportWidth <= 0f ||
      !bodyTrackWidth.isFinite() ||
      bodyTrackWidth <= 0f ||
      !displayZoom.isFinite() ||
      displayZoom <= 0f
  ) {
    return null
  }

  val (contentInsetLeft, contentInsetRight) =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> EditorHeaderContentInset to EditorHeaderContentInset
      is EditorDocumentLayoutSpec.Paginated ->
        layoutSpec.pageMarginLeft to layoutSpec.pageMarginRight
    }

  fun scaledInset(value: Float): Float =
    (value * displayZoom).takeIf { it.isFinite() }?.coerceAtLeast(0f) ?: 0f

  val leftInset = scaledInset(contentInsetLeft)
  val rightInset = scaledInset(contentInsetRight)
  val readableMin =
    maxOf(
      EditorHeaderMinContentWidth * displayZoom,
      EditorHeaderMinContentWidth * EditorHeaderMinScale,
    )
  val visibleCapacity = (viewportWidth - leftInset - rightInset).coerceAtLeast(0f)
  val trackWidth =
    maxOf(bodyTrackWidth, leftInset + rightInset + minOf(readableMin, visibleCapacity))
  val contentWidth = maxOf(viewportWidth, trackWidth)
  val trackLeft = (contentWidth - trackWidth) / 2f
  val contentLeft = trackLeft + leftInset
  val contentRight = maxOf(contentLeft, trackLeft + trackWidth - rightInset)
  val fieldWidth =
    minOf(
      contentRight - contentLeft,
      (viewportWidth - EditorHeaderViewportGap * 2f).coerceAtLeast(0f),
    )

  return EditorHeaderGeometry(
    contentLeft = contentLeft,
    contentRight = contentRight,
    fieldWidth = fieldWidth,
    maxScrollOffset = (contentWidth - viewportWidth).coerceAtLeast(0f),
  )
}

@Composable
internal fun EditorHeaderFrame(
  geometry: EditorHeaderGeometry?,
  horizontalScrollOffset: () -> Float = { 0f },
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  val density = LocalDensity.current.density

  Box(modifier = modifier.fillMaxWidth(), contentAlignment = Alignment.TopStart) {
    Box(
      modifier =
        if (geometry == null) {
          Modifier.fillMaxWidth().padding(horizontal = EditorHeaderContentInset.dp)
        } else {
          Modifier.width(geometry.fieldWidth.dp).graphicsLayer {
            translationX = geometry.resolveFieldScreenLeft(horizontalScrollOffset()) * density
          }
        }
    ) {
      content()
    }
  }
}

@Composable
internal fun EditorHeader(
  title: String,
  subtitle: String,
  loading: Boolean,
  enabled: Boolean = true,
  editing: Boolean = true,
  doubleTapToEditEnabled: Boolean = true,
  editingActivationEnabled: Boolean = true,
  readingTapIdentity: Any? = Unit,
  readingModeCleanupRequest: Int = 0,
  showBottomDivider: Boolean = true,
  topInset: Dp,
  subtitleFocusRequestVersion: Int = 0,
  modifier: Modifier = Modifier,
  onTitleChange: (String) -> Unit,
  onSubtitleChange: (String) -> Unit,
  onTitleFocused: () -> Unit,
  onSubtitleFocused: () -> Unit,
  onHeightChanged: (Float) -> Unit,
  onEnterDocument: () -> Unit,
  onRequestEditing: () -> Boolean = { false },
  onReadingTapHint: () -> Unit = {},
) {
  val density = LocalDensity.current
  val showSkeleton = LocalSkeleton.current.enabled || loading
  val inputEnabled = enabled && editing
  val titleInputState =
    rememberTextInputState(
      value = title,
      onValueChange = onTitleChange,
      enabled = inputEnabled,
      onDismiss = onEnterDocument,
    )
  val subtitleInputState =
    rememberTextInputState(
      value = subtitle,
      onValueChange = onSubtitleChange,
      enabled = inputEnabled,
      onDismiss = onEnterDocument,
    )
  LaunchedEffect(subtitleFocusRequestVersion, inputEnabled) {
    if (inputEnabled && subtitleFocusRequestVersion > 0) {
      subtitleInputState.requestFocus()
    }
  }
  LaunchedEffect(editing, readingModeCleanupRequest) {
    if (!editing) {
      titleInputState.finishCompositionAndCollapseSelection()
      subtitleInputState.finishCompositionAndCollapseSelection()
    }
  }
  val resolveHeight: (Int) -> Float = remember(density) { { height -> height / density.density } }

  Box(
    modifier =
      modifier.fillMaxWidth().onSizeChanged { size -> onHeightChanged(resolveHeight(size.height)) },
    contentAlignment = Alignment.TopStart,
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(top = topInset + TitleTopPadding)) {
      Spacer(Modifier.height(TitleBlockSpacing))

      EditorHeaderField(
        text = titleInputState.value,
        onValueChange = {
          if (editing) {
            titleInputState.onValueChange(sanitizeTitleFieldValue(it))
          } else {
            titleInputState.updateSelection(it.selection)
          }
        },
        placeholder = "제목",
        style =
          AppTheme.typography.title.copy(
            fontSize = 20.sp,
            lineHeight = 28.sp,
            fontWeight = FontWeight.Bold,
          ),
        placeholderStyle =
          AppTheme.typography.title.copy(
            fontSize = 20.sp,
            lineHeight = 28.sp,
            fontWeight = FontWeight.Bold,
          ),
        showSkeleton = showSkeleton,
        enabled = enabled,
        editing = editing,
        doubleTapToEditEnabled = doubleTapToEditEnabled,
        editingActivationEnabled = editingActivationEnabled,
        readingTapIdentity = readingTapIdentity,
        readingModeCleanupRequest = readingModeCleanupRequest,
        imeAction = ImeAction.Next,
        onFocusNext = { subtitleInputState.requestFocus() },
        onEnterDocument = { subtitleInputState.requestFocus() },
        onBackspaceAtStart = {},
        onFocused = onTitleFocused,
        modifier = Modifier.fillMaxWidth(),
        textInputState = titleInputState,
        onRequestEditing = onRequestEditing,
        onReadingTapHint = onReadingTapHint,
      )

      Spacer(Modifier.height(TitleBetweenSpacing))

      EditorHeaderField(
        text = subtitleInputState.value,
        onValueChange = {
          if (editing) {
            subtitleInputState.onValueChange(sanitizeSubtitleFieldValue(it))
          } else {
            subtitleInputState.updateSelection(it.selection)
          }
        },
        placeholder = "부제목",
        style =
          AppTheme.typography.body.copy(
            fontSize = 16.sp,
            lineHeight = 24.sp,
            fontWeight = FontWeight.Medium,
          ),
        placeholderStyle =
          AppTheme.typography.body.copy(
            fontSize = 16.sp,
            lineHeight = 24.sp,
            fontWeight = FontWeight.Medium,
          ),
        showSkeleton = showSkeleton,
        enabled = enabled,
        editing = editing,
        doubleTapToEditEnabled = doubleTapToEditEnabled,
        editingActivationEnabled = editingActivationEnabled,
        readingTapIdentity = readingTapIdentity,
        readingModeCleanupRequest = readingModeCleanupRequest,
        imeAction = ImeAction.Done,
        onFocusNext = onEnterDocument,
        onEnterDocument = onEnterDocument,
        onFocusPrevious = { titleInputState.requestFocus() },
        onBackspaceAtStart = {
          if (subtitleInputState.value.text.isEmpty()) {
            titleInputState.requestFocus()
          }
        },
        onFocused = onSubtitleFocused,
        modifier = Modifier.fillMaxWidth(),
        textInputState = subtitleInputState,
        onRequestEditing = onRequestEditing,
        onReadingTapHint = onReadingTapHint,
      )

      Spacer(Modifier.height(TitleBlockSpacing))

      if (showBottomDivider) {
        Box(modifier = Modifier.width(SubtitleDividerWidth)) {
          Divider(color = AppTheme.colors.borderDefault)
        }
      }
    }
  }
}

@Composable
private fun EditorHeaderField(
  text: TextFieldValue,
  onValueChange: (TextFieldValue) -> Unit,
  placeholder: String,
  style: TextStyle,
  placeholderStyle: TextStyle,
  showSkeleton: Boolean,
  enabled: Boolean,
  editing: Boolean,
  doubleTapToEditEnabled: Boolean,
  editingActivationEnabled: Boolean,
  readingTapIdentity: Any?,
  readingModeCleanupRequest: Int,
  imeAction: ImeAction,
  onFocusNext: () -> Unit,
  onEnterDocument: () -> Unit,
  onFocusPrevious: (() -> Unit)? = null,
  onBackspaceAtStart: () -> Unit,
  onFocused: () -> Unit,
  modifier: Modifier = Modifier,
  textInputState: TextInputState,
  onRequestEditing: () -> Boolean,
  onReadingTapHint: () -> Unit,
) {
  val verticalExitScope = rememberCoroutineScope()
  val keyboardController = LocalSoftwareKeyboardController.current
  val density = LocalDensity.current
  val viewConfiguration = LocalViewConfiguration.current
  val inputEnabled = enabled && editing
  val currentInputEnabled by rememberUpdatedState(inputEnabled)
  val currentReadingTapEnabled by rememberUpdatedState(enabled && !editing)
  val currentOnRequestEditing by rememberUpdatedState(onRequestEditing)
  val currentOnReadingTapHint by rememberUpdatedState(onReadingTapHint)
  var latestTextLayout by remember { mutableStateOf<TextLayoutResult?>(null) }
  var pendingActivationOffset by remember { mutableStateOf<Int?>(null) }
  val readingTapTracker =
    remember(
      viewConfiguration.touchSlop,
      density.density,
      doubleTapToEditEnabled,
      editingActivationEnabled,
      readingTapIdentity,
    ) {
      EditorHeaderReadingTapTracker(
        touchSlopPx = viewConfiguration.touchSlop,
        maxTapDistancePx = with(density) { 20.dp.toPx() },
        doubleTapToEditEnabled = doubleTapToEditEnabled,
        editingActivationEnabled = editingActivationEnabled,
      )
    }
  val verticalNavigation =
    remember(verticalExitScope, textInputState) {
      HeaderVerticalNavigationState(
        scope = verticalExitScope,
        currentValue = { textInputState.value },
        currentEnabled = { currentInputEnabled },
      )
    }
  LaunchedEffect(inputEnabled, pendingActivationOffset) {
    val offset = pendingActivationOffset ?: return@LaunchedEffect
    if (!inputEnabled) return@LaunchedEffect
    withFrameNanos {}
    textInputState.updateSelection(TextRange(offset.coerceIn(0, textInputState.value.text.length)))
    textInputState.requestFocus()
    keyboardController?.show()
    pendingActivationOffset = null
  }
  LaunchedEffect(readingTapIdentity) {
    pendingActivationOffset = null
    readingTapTracker.onModeChanged()
  }
  LaunchedEffect(editing, readingModeCleanupRequest) {
    if (!editing) {
      pendingActivationOffset = null
      readingTapTracker.onModeChanged()
    }
  }

  BasicTextField(
    value = text,
    onValueChange = onValueChange,
    enabled = enabled,
    readOnly = !editing,
    modifier =
      modifier
        .textInputFocusable(textInputState, enabled = inputEnabled) {
          verticalNavigation.onFocusChanged(it.isFocused)
          if (it.isFocused) {
            onFocused()
          }
        }
        .invalidateHeaderVerticalNavigationOnPointerDown(verticalNavigation)
        .then(
          if (enabled && !editing && editingActivationEnabled) {
            Modifier.semantics {
              customActions =
                listOf(
                  CustomAccessibilityAction(label = "편집") {
                    val offset = textInputState.value.selection.start
                    if (currentOnRequestEditing()) {
                      pendingActivationOffset = offset
                      true
                    } else {
                      false
                    }
                  }
                )
            }
          } else {
            Modifier
          }
        )
        .observeEditorHeaderReadingTaps(
          enabled = { currentReadingTapEnabled },
          tracker = readingTapTracker,
          currentSelectionIsExpanded = { !textInputState.value.selection.collapsed },
          offsetForPosition = { position ->
            (latestTextLayout?.getOffsetForPosition(position)
                ?: textInputState.value.selection.start)
              .coerceIn(0, textInputState.value.text.length)
          },
          onActivate = { offset ->
            if (currentOnRequestEditing()) {
              pendingActivationOffset = offset
            }
          },
          onShowHint = { currentOnReadingTapHint() },
        )
        .onPreviewKeyEvent {
          if (!currentInputEnabled) {
            return@onPreviewKeyEvent false
          }
          if (it.type != KeyEventType.KeyDown) {
            return@onPreviewKeyEvent false
          }

          verticalNavigation.invalidate()

          when (it.key) {
            Key.DirectionDown,
            Key.DirectionUp -> {
              verticalNavigation.handleVerticalKeyDown(
                event = it,
                onExitUp = onFocusPrevious,
                onExitDown = onFocusNext,
              )
              false
            }
            Key.Enter -> {
              if (it.isShiftPressed) {
                false
              } else {
                onEnterDocument()
                true
              }
            }
            Key.Tab -> {
              if (it.isShiftPressed) {
                onFocusPrevious?.invoke()
              } else {
                onFocusNext()
              }
              true
            }
            Key.Backspace -> {
              onBackspaceAtStart()
              false
            }
            else -> false
          }
        },
    textStyle = style.copy(color = AppTheme.colors.textDefault),
    cursorBrush =
      if (editing) {
        SolidColor(AppTheme.colors.textDefault)
      } else {
        SolidColor(Color.Transparent)
      },
    keyboardOptions = KeyboardOptions(imeAction = imeAction),
    keyboardActions = KeyboardActions(onNext = { onFocusNext() }, onDone = { onEnterDocument() }),
    onTextLayout = {
      latestTextLayout = it
      verticalNavigation.onTextLayout(it)
    },
    minLines = 1,
    maxLines = Int.MAX_VALUE,
    decorationBox = { innerTextField ->
      Box(modifier = Modifier.fillMaxWidth()) {
        when {
          showSkeleton && text.text.isEmpty() -> {
            SkeletonTextBone(
              text = placeholder,
              style = style,
              modifier = Modifier.fillMaxWidth(),
              maxLines = 2,
            )
          }
          text.text.isEmpty() -> {
            Text(text = placeholder, style = placeholderStyle, color = AppTheme.colors.textHint)
          }
        }

        innerTextField()
      }
    },
  )
}
