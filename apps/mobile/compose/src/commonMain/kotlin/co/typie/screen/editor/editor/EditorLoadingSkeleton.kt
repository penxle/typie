package co.typie.screen.editor.editor

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.absolutePadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.computeInitialPaginatedZoom
import co.typie.editor.ffi.Size
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.ui.component.Text
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.floatOrNull

private const val ContinuousPageHorizontalPadding = 20f

internal data class EditorLoadingBodyGeometry(
  val trackWidth: Float,
  val leftPadding: Float,
  val rightPadding: Float,
)

internal fun resolveEditorLoadingLayoutSpec(
  encodedLayoutMode: JsonElement?
): EditorDocumentLayoutSpec? {
  val value = encodedLayoutMode as? JsonObject ?: return null
  return when ((value["type"] as? JsonPrimitive)?.contentOrNull) {
    "continuous" -> value.positiveFloat("maxWidth")?.let(EditorDocumentLayoutSpec::Continuous)
    "paginated" ->
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = value.positiveFloat("pageWidth") ?: return null,
        pageHeight = value.positiveFloat("pageHeight") ?: return null,
        pageMarginTop = value.nonNegativeFloat("pageMarginTop") ?: return null,
        pageMarginBottom = value.nonNegativeFloat("pageMarginBottom") ?: return null,
        pageMarginLeft = value.nonNegativeFloat("pageMarginLeft") ?: return null,
        pageMarginRight = value.nonNegativeFloat("pageMarginRight") ?: return null,
      )
    else -> null
  }
}

private fun JsonObject.positiveFloat(key: String): Float? =
  (get(key) as? JsonPrimitive)?.floatOrNull?.takeIf { it.isFinite() && it > 0f }

private fun JsonObject.nonNegativeFloat(key: String): Float? =
  (get(key) as? JsonPrimitive)?.floatOrNull?.takeIf { it.isFinite() && it >= 0f }

internal fun resolveEditorLoadingBodyTrackWidth(
  layoutSpec: EditorDocumentLayoutSpec,
  availableWidth: Float,
): Float {
  val available = availableWidth.takeIf { it.isFinite() && it > 0f } ?: return 0f
  return when (layoutSpec) {
    is EditorDocumentLayoutSpec.Continuous -> {
      val contentCap = layoutSpec.maxWidth.takeIf { it.isFinite() && it > 0f }
      minOf(available, contentCap?.plus(ContinuousPageHorizontalPadding * 2f) ?: available)
    }
    is EditorDocumentLayoutSpec.Paginated -> {
      val pageWidth = layoutSpec.pageWidth.takeIf { it.isFinite() && it > 0f } ?: return available
      pageWidth * computeInitialPaginatedZoom(pageWidth = pageWidth, viewportWidth = available)
    }
  }
}

internal fun resolveEditorLoadingBodyGeometry(
  layoutSpec: EditorDocumentLayoutSpec,
  availableWidth: Float,
): EditorLoadingBodyGeometry {
  val trackWidth =
    resolveEditorLoadingBodyTrackWidth(layoutSpec = layoutSpec, availableWidth = availableWidth)
  return when (layoutSpec) {
    is EditorDocumentLayoutSpec.Continuous ->
      EditorLoadingBodyGeometry(
        trackWidth = trackWidth,
        leftPadding = ContinuousPageHorizontalPadding,
        rightPadding = ContinuousPageHorizontalPadding,
      )
    is EditorDocumentLayoutSpec.Paginated -> {
      val pageWidth = layoutSpec.pageWidth.takeIf { it.isFinite() && it > 0f }
      val displayScale = pageWidth?.let { trackWidth / it } ?: 0f
      EditorLoadingBodyGeometry(
        trackWidth = trackWidth,
        leftPadding =
          layoutSpec.pageMarginLeft.takeIf { it.isFinite() && it >= 0f }?.times(displayScale) ?: 0f,
        rightPadding =
          layoutSpec.pageMarginRight.takeIf { it.isFinite() && it >= 0f }?.times(displayScale) ?: 0f,
      )
    }
  }
}

internal fun hasValidEditorGeometry(
  editorAttached: Boolean,
  pageSizes: List<Size>,
  trackWidth: Float,
): Boolean =
  editorAttached &&
    trackWidth.isPositiveFinite() &&
    pageSizes.isNotEmpty() &&
    pageSizes.all { it.width.isPositiveFinite() && it.height.isPositiveFinite() }

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
      val bodyGeometry =
        resolveEditorLoadingBodyGeometry(layoutSpec = layoutSpec, availableWidth = availableWidth)
      val headerTrackWidth =
        when (layoutSpec) {
          is EditorDocumentLayoutSpec.Continuous -> bodyGeometry.trackWidth
          is EditorDocumentLayoutSpec.Paginated -> availableWidth
        }
      Column {
        EditorHeader(
          title = "",
          subtitle = "",
          layoutSpec = layoutSpec,
          trackWidth = headerTrackWidth,
          loading = true,
          enabled = false,
          topInset = topInset,
          onTitleChange = {},
          onSubtitleChange = {},
          onTitleFocused = {},
          onSubtitleFocused = {},
          onHeightChanged = {},
          onEnterDocument = {},
        )
        EditorBodyLoadingSkeleton(geometry = bodyGeometry, modifier = Modifier.fillMaxSize())
      }
    }
  }
}

@Composable
private fun EditorBodyLoadingSkeleton(
  geometry: EditorLoadingBodyGeometry,
  modifier: Modifier = Modifier,
) {
  Box(modifier = modifier, contentAlignment = Alignment.TopCenter) {
    Column(
      modifier =
        Modifier.width(geometry.trackWidth.dp)
          .absolutePadding(
            left = geometry.leftPadding.dp,
            top = 32.dp,
            right = geometry.rightPadding.dp,
            bottom = 32.dp,
          ),
      verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
      Skeleton.list(6) { text(16..34) }
        .forEach { line -> Text(text = line, style = AppTheme.typography.body) }
    }
  }
}
