package co.typie.screen.editor.editor

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.absolutePadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.computeInitialDocumentZoom
import co.typie.editor.ffi.Size
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.screen.editor.editor.header.EditorHeaderFrame
import co.typie.screen.editor.editor.header.resolveEditorHeaderGeometry
import co.typie.ui.component.Text
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

private const val ContinuousPageHorizontalPadding = 20f

internal fun hasValidEditorGeometry(
  editorAttached: Boolean,
  pageSizes: List<Size>,
  trackWidth: Float,
): Boolean =
  editorAttached &&
    trackWidth.isPositiveFinite() &&
    pageSizes.isNotEmpty() &&
    pageSizes.all { it.width.isPositiveFinite() && it.height.isPositiveFinite() }

internal fun hasInvalidPublishedEditorGeometry(
  publishedRevision: Long?,
  geometryValid: Boolean,
): Boolean = publishedRevision != null && !geometryValid

internal fun canHideEditorLoadingSkeleton(
  loading: Boolean,
  geometryValid: Boolean,
  sessionAttached: Boolean,
  hasInitialFrame: Boolean,
): Boolean = !loading && geometryValid && sessionAttached && hasInitialFrame

private fun Float.isPositiveFinite(): Boolean = isFinite() && this > 0f

@Composable
internal fun EditorLoadingSkeleton(
  layoutSpec: EditorDocumentLayoutSpec,
  topInset: Dp,
  background: Color,
  modifier: Modifier = Modifier,
) {
  Skeleton(enabled = true, modifier = modifier.fillMaxSize().background(background)) {
    BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
      val availableWidth = maxWidth.value
      val displayZoom =
        computeInitialDocumentZoom(layoutSpec = layoutSpec, viewportWidth = availableWidth)
      val bodyTrackWidth =
        when (layoutSpec) {
          is EditorDocumentLayoutSpec.Continuous ->
            minOf(availableWidth, layoutSpec.maxWidth + ContinuousPageHorizontalPadding * 2f)
          is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageWidth * displayZoom
        }
      val headerGeometry =
        resolveEditorHeaderGeometry(
          layoutSpec = layoutSpec,
          viewportWidth = availableWidth,
          bodyTrackWidth = bodyTrackWidth,
          displayZoom = displayZoom,
        )
      Column {
        EditorHeaderFrame(geometry = headerGeometry) {
          EditorHeader(
            title = "",
            subtitle = "",
            loading = true,
            enabled = false,
            showBottomDivider = layoutSpec is EditorDocumentLayoutSpec.Continuous,
            topInset = topInset,
            onTitleChange = {},
            onSubtitleChange = {},
            onTitleFocused = {},
            onSubtitleFocused = {},
            onHeightChanged = {},
            onEnterDocument = {},
          )
        }
        EditorBodyLoadingSkeleton(
          layoutSpec = layoutSpec,
          trackWidth = bodyTrackWidth,
          modifier = Modifier.fillMaxSize(),
        )
      }
    }
  }
}

@Composable
private fun EditorBodyLoadingSkeleton(
  layoutSpec: EditorDocumentLayoutSpec,
  trackWidth: Float,
  modifier: Modifier = Modifier,
) {
  val trackModifier =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous ->
        Modifier.width(trackWidth.dp).padding(horizontal = ContinuousPageHorizontalPadding.dp)
      is EditorDocumentLayoutSpec.Paginated -> {
        val displayScale =
          layoutSpec.pageWidth.takeIf { it.isFinite() && it > 0f }?.let { trackWidth / it } ?: 0f
        fun scaledMargin(margin: Float): Float =
          margin.takeIf { it.isFinite() && it >= 0f }?.times(displayScale) ?: 0f

        Modifier.width(trackWidth.dp)
          .absolutePadding(
            left = scaledMargin(layoutSpec.pageMarginLeft).dp,
            right = scaledMargin(layoutSpec.pageMarginRight).dp,
          )
      }
    }
  Box(modifier = modifier, contentAlignment = Alignment.TopCenter) {
    Column(
      modifier = trackModifier.padding(vertical = 32.dp),
      verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
      Skeleton.list(6) { text(16..34) }
        .forEach { line -> Text(text = line, style = AppTheme.typography.body) }
    }
  }
}
