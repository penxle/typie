package co.typie.editor.body

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.interaction.EditorPointerCoordinateResolver
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests

@Composable
internal fun EditorExtensionArea(
  forwardingEnabled: Boolean,
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val editor = runtime.editor
  val uiState = LocalEditorUiState.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val editorBounds = uiState.editorBoundsInContainer
  val densityValue = density.density
  val extensionAreaModifier =
    if (forwardingEnabled && editor != null) {
      Modifier.editorInteractions(
        editor = editor,
        bringIntoViewRequests = bringIntoViewRequests,
        uiState = uiState,
        density = densityValue,
        coordinateResolver =
          EditorExtensionAreaPointerCoordinateResolver(
            bounds = editorBounds,
            density = densityValue,
          ),
      )
    } else {
      Modifier
    }

  Box(modifier = modifier.fillMaxWidth().then(extensionAreaModifier), content = content)
}

internal data class EditorExtensionAreaPointerCoordinateResolver(
  private val bounds: EditorBoundsInContainer,
  private val density: Float,
) : EditorPointerCoordinateResolver {
  override fun positionForStart(position: Offset): Offset? {
    if (!canForwardStart(position)) {
      return null
    }
    return positionForActivePointer(position)
  }

  override fun positionForActivePointer(position: Offset): Offset? {
    if (!bounds.isValid || density <= 0f) {
      return null
    }

    val x = (position.x / density).coerceIn(bounds.x, bounds.x + bounds.width)
    val y = (position.y / density).coerceIn(bounds.y, bounds.y + bounds.height)
    return Offset(x = (x - bounds.x) * density, y = (y - bounds.y) * density)
  }

  private fun canForwardStart(position: Offset): Boolean {
    if (!bounds.isValid || density <= 0f) {
      return false
    }

    val x = position.x / density
    val y = position.y / density
    return !bounds.contains(x = x, y = y)
  }
}

private fun EditorBoundsInContainer.contains(x: Float, y: Float): Boolean =
  x >= this.x && x <= this.x + width && y >= this.y && y <= this.y + height
