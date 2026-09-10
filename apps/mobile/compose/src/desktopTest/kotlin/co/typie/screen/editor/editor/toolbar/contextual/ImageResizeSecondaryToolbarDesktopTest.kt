package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.external.EditorImageResizeDraft
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class ImageResizeSecondaryToolbarDesktopTest {
  @Test
  fun closingRestoresHeightWithoutAMountedImage() = runComposeUiTest {
    val nodeId = "image-node"
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake =
      FakeFfiEditor(
        externalElementsProvider = {
          listOf(
            ExternalElement(
              pageIdx = 0,
              node = nodeId,
              bounds = Rect(0f, 0f, 600f, 600f),
              isSelected = true,
              data = ExternalElementData.Image(id = "asset", proportion = 50, maxHeight = 800f),
            )
          )
        }
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope)
    val externalState = EditorExternalElementState()
    val toolbarVisible = mutableStateOf(true)
    try {
      fake.applySnapshot(editor)
      editor.activateVisualHost(Any())
      assertTrue(editor.acceptPublication(assertNotNull(editor.publishIfReady(emptySet()))))
      runtime.attach(editor)
      externalState.put(EditorImageAsset("asset", "", "", 600, 3000, 0.2, null))
      externalState.images.resizeDrafts[nodeId] =
        EditorImageResizeDraft(
          proportion = 75f,
          maxSize = androidx.compose.ui.geometry.Size(160f, 800f),
        )
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalHazeBlurStyle provides
            HazeBlurStyle {
              blurRadius(20.dp)
              noiseFactor(0f)
              colorEffects(emptyList())
            },
          LocalEditorRuntime provides runtime,
          LocalEditorExternalElementState provides externalState,
        ) {
          if (toolbarVisible.value) {
            ImageResizeSecondaryToolbar(
              nodeId = nodeId,
              onClose = {},
              modifier = Modifier.testTag(ToolbarTag).size(width = 320.dp, height = 48.dp),
            )
          }
        }
      }
      runOnIdle {
        assertNotNull(externalState.images.resizeDrafts[nodeId])
        toolbarVisible.value = false
      }
      waitUntil { externalState.images.resizeDrafts[nodeId] == null }
      runOnIdle {
        val heights =
          fake.enqueued.filterIsInstance<Message.System>().mapNotNull {
            (it.event as? SystemEvent.SetExternalHeight)?.height
          }
        assertEquals(listOf(400f), heights)
        assertTrue(fake.enqueued.none { it is Message.Node })
      }
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }

  @Test
  fun unavailableSurfaceClearsDraftWithoutFailingEditor() = runComposeUiTest {
    val nodeId = "image-node"
    val assetId = "image-asset"
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    var frameKey: FrameKey? = null
    val fake =
      FakeFfiEditor(
        pageSizesProvider = { listOf(Size(width = 400f, height = 700f)) },
        externalElementsProvider = {
          listOf(
            ExternalElement(
              pageIdx = 0,
              node = nodeId,
              bounds = Rect(x = 0f, y = 0f, width = 200f, height = 100f),
              isSelected = true,
              data = ExternalElementData.Image(id = assetId, proportion = 50),
            )
          )
        },
        onTick = {
          listOf(
            EditorEvent.StateChanged(listOf(StateField.ExternalElements)),
            EditorEvent.RenderInvalidated,
          )
        },
      )
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope)
    val externalState = EditorExternalElementState()

    try {
      val initialUpdate = fake.applySnapshot(editor)
      editor.activateVisualHost(Any())
      val surface =
        editor.attachSurface(
          page = 0,
          handle = 1L,
          width = 400.0,
          height = 700.0,
          scaleFactor = 1.0,
          wakeDelivery = { frameKey = it },
        )
      editor.requestSurfacePages(setOf(0))
      waitUntil { frameKey != null }
      editor.deliverFrame(
        session = surface,
        bitmap = ImageBitmap(width = 400, height = 700),
        pixelSize = IntSize(width = 400, height = 700),
        editorRevision = initialUpdate.revision,
        frameKey = assertNotNull(frameKey).value,
      )
      waitUntil { editor.publishIfReady(setOf(0)) != null }
      assertTrue(editor.acceptPublication(assertNotNull(editor.publishIfReady(setOf(0)))))
      runtime.attach(editor)
      externalState.put(
        EditorImageAsset(
          id = assetId,
          url = "https://example.com/image.png",
          originalUrl = "https://example.com/original.png",
          width = 400,
          height = 200,
          ratio = 2.0,
          placeholder = null,
        )
      )

      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalHazeBlurStyle provides
            HazeBlurStyle {
              blurRadius(20.dp)
              noiseFactor(0f)
              colorEffects(emptyList())
            },
          LocalEditorRuntime provides runtime,
          LocalEditorExternalElementState provides externalState,
        ) {
          ImageResizeSecondaryToolbar(
            nodeId = nodeId,
            onClose = {},
            modifier = Modifier.testTag(ToolbarTag).size(width = 320.dp, height = 48.dp),
          )
        }
      }
      waitForIdle()
      frameKey = null

      onNodeWithTag(ToolbarTag).performMouseInput {
        moveTo(Offset(x = 90f, y = center.y))
        press()
        moveTo(Offset(x = 180f, y = center.y))
      }
      runOnIdle {
        val draft = assertNotNull(externalState.images.resizeDrafts[nodeId])
        val height =
          fake.enqueued
            .filterIsInstance<Message.System>()
            .mapNotNull { (it.event as? SystemEvent.SetExternalHeight)?.height }
            .last()
        assertEquals(draft.maxSize.height * draft.proportion / 100f, height, 0.0001f)
        assertTrue(fake.enqueued.none { it is Message.Node })
      }
      onNodeWithTag(ToolbarTag).performMouseInput { release() }

      waitUntil { frameKey != null }
      runOnIdle { assertNotNull(externalState.images.resizeDrafts[nodeId]) }
      editor.surfaceTargetUnavailable(surface, assertNotNull(frameKey))
      waitUntil { externalState.images.resizeDrafts[nodeId] == null }

      runOnIdle { assertFalse(editor.terminal) }
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }

  private companion object {
    const val ToolbarTag = "image-resize-secondary-toolbar"
  }
}
