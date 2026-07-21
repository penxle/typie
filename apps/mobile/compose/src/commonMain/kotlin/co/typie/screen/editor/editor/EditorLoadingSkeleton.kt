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
import co.typie.editor.computeInitialPaginatedZoom
import co.typie.editor.ffi.Size
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.screen.editor.editor.header.resolveEditorHeaderTrackWidth
import co.typie.ui.component.Text
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.floatOrNull

private const val ContinuousPageHorizontalPadding = 20f

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
      val bodyTrackWidth =
        when (layoutSpec) {
          is EditorDocumentLayoutSpec.Continuous ->
            minOf(availableWidth, layoutSpec.maxWidth + ContinuousPageHorizontalPadding * 2f)
          is EditorDocumentLayoutSpec.Paginated ->
            layoutSpec.pageWidth *
              computeInitialPaginatedZoom(
                pageWidth = layoutSpec.pageWidth,
                viewportWidth = availableWidth,
              )
        }
      val headerTrackWidth =
        resolveEditorHeaderTrackWidth(
          layoutSpec = layoutSpec,
          resolvedPageWidth = bodyTrackWidth,
          visibleBodyWidth = availableWidth,
          bodyTrackWidth = bodyTrackWidth,
        )
      Column {
        EditorHeader(
          title = "",
          subtitle = "",
          layoutSpec = layoutSpec,
          trackWidth = headerTrackWidth,
          pageTrackWidth = bodyTrackWidth,
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
