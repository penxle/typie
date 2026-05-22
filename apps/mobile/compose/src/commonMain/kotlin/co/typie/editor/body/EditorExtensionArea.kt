package co.typie.editor.body

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.interaction.EditorPointerCoordinateResolver
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState

@Composable
internal fun EditorExtensionArea(
  layoutSpec: EditorDocumentLayoutSpec,
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val editor = runtime.editor
  val uiState = LocalEditorUiState.current
  val interactionScope = LocalEditorInteractionScope.current
  val editorBounds = uiState.editorBoundsInContainer
  val densityValue = density.density
  val extensionAreaModifier =
    if (editor != null) {
      Modifier.editorInteractions(
        density = densityValue,
        interactionController = interactionScope.controller,
        coordinateResolver =
          EditorExtensionAreaPointerCoordinateResolver(
            layoutSpec = layoutSpec,
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
  private val layoutSpec: EditorDocumentLayoutSpec,
  private val bounds: EditorBoundsInContainer,
  private val density: Float,
) : EditorPointerCoordinateResolver {
  override fun positionForPointerStart(position: Offset): Offset? {
    if (!isOutsideEditorBounds(position)) {
      return null
    }
    return positionForActivePointer(position)
  }

  override fun positionForTapStart(position: Offset): Offset? {
    if (layoutSpec !is EditorDocumentLayoutSpec.Continuous || !isOutsideEditorBounds(position)) {
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

  private fun isOutsideEditorBounds(position: Offset): Boolean {
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
