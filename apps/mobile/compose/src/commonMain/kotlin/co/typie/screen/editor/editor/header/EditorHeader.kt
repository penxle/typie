package co.typie.screen.editor.editor.header

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
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
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Text
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.SkeletonTextBone
import co.typie.ui.theme.AppTheme

private val TitleHorizontalPadding = 20.dp
private val TitleTopPadding = 12.dp
private val TitleBlockSpacing = 40.dp
private val TitleBetweenSpacing = 8.dp
private val SubtitleDividerWidth = 120.dp

@Composable
internal fun EditorHeader(
  title: String,
  subtitle: String,
  layoutSpec: EditorDocumentLayoutSpec,
  trackWidth: Float,
  loading: Boolean,
  topInset: Dp,
  modifier: Modifier = Modifier,
  onTitleChange: (String) -> Unit,
  onSubtitleChange: (String) -> Unit,
  onHeightChanged: (Float) -> Unit,
  onEnterDocument: () -> Unit,
) {
  val density = LocalDensity.current
  val showSkeleton = LocalSkeleton.current.enabled || loading
  val titleInputState =
    rememberTextInputState(
      value = title,
      onValueChange = onTitleChange,
      onDismiss = onEnterDocument,
    )
  val subtitleInputState =
    rememberTextInputState(
      value = subtitle,
      onValueChange = onSubtitleChange,
      onDismiss = onEnterDocument,
    )
  val resolveHeight: (Int) -> Float = remember(density) { { height -> height / density.density } }

  Box(
    modifier =
      modifier.fillMaxWidth().onSizeChanged { size -> onHeightChanged(resolveHeight(size.height)) },
    contentAlignment = Alignment.TopCenter,
  ) {
    val contentModifier = Modifier.run {
      when {
        layoutSpec is EditorDocumentLayoutSpec.Paginated && trackWidth > 0f -> width(trackWidth.dp)
        else -> widthIn(max = ResponsiveContainerDefaults.MaxWidth).fillMaxWidth()
      }
    }

    Column(
      modifier =
        contentModifier.padding(
          top = topInset + TitleTopPadding,
          start = TitleHorizontalPadding,
          end = TitleHorizontalPadding,
        )
    ) {
      Spacer(Modifier.height(TitleBlockSpacing))

      EditorHeaderField(
        text = titleInputState.value,
        onValueChange = { titleInputState.onValueChange(sanitizeTitleFieldValue(it)) },
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
        imeAction = ImeAction.Next,
        onFocusNext = { subtitleInputState.requestFocus() },
        onEnterDocument = { subtitleInputState.requestFocus() },
        onFocusPrevious = {},
        onBackspaceAtStart = {},
        modifier = Modifier.fillMaxWidth(),
        textInputState = titleInputState,
      )

      Spacer(Modifier.height(TitleBetweenSpacing))

      EditorHeaderField(
        text = subtitleInputState.value,
        onValueChange = { subtitleInputState.onValueChange(sanitizeSubtitleFieldValue(it)) },
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
        imeAction = ImeAction.Done,
        onFocusNext = onEnterDocument,
        onEnterDocument = onEnterDocument,
        onFocusPrevious = { titleInputState.requestFocus() },
        onBackspaceAtStart = {
          if (subtitleInputState.value.text.isEmpty()) {
            titleInputState.requestFocus()
          }
        },
        modifier = Modifier.fillMaxWidth(),
        textInputState = subtitleInputState,
      )

      Spacer(Modifier.height(TitleBlockSpacing))

      if (layoutSpec !is EditorDocumentLayoutSpec.Paginated) {
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
  imeAction: ImeAction,
  onFocusNext: () -> Unit,
  onEnterDocument: () -> Unit,
  onFocusPrevious: () -> Unit,
  onBackspaceAtStart: () -> Unit,
  modifier: Modifier = Modifier,
  textInputState: TextInputState,
) {
  BasicTextField(
    value = text,
    onValueChange = onValueChange,
    modifier =
      modifier.textInputFocusable(textInputState).onPreviewKeyEvent {
        if (it.type != KeyEventType.KeyDown) {
          return@onPreviewKeyEvent false
        }

        when (it.key) {
          Key.DirectionDown -> {
            onFocusNext()
            true
          }
          Key.DirectionUp -> {
            onFocusPrevious()
            true
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
              onFocusPrevious()
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
    cursorBrush = SolidColor(AppTheme.colors.textDefault),
    keyboardOptions = KeyboardOptions(imeAction = imeAction),
    keyboardActions = KeyboardActions(onNext = { onFocusNext() }, onDone = { onEnterDocument() }),
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
