package co.typie.editor.external

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import coil3.ColorImage
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.annotation.DelicateCoilApi
import coil3.intercept.Interceptor
import coil3.request.SuccessResult
import coil3.size.Size
import java.util.concurrent.CopyOnWriteArrayList
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class, DelicateCoilApi::class)
class EditorExternalElementOverlayDesktopTest {
  @Test
  fun zoomKeepsLogicalHeightStableWhileContentChangesStillReportHeight() {
    val assets =
      listOf(
        ExternalElementData.File(id = "asset") to EditorFileAsset("asset", "report.txt", "", 100L),
        ExternalElementData.Embed(id = "asset") to
          EditorEmbedAsset(
            id = "asset",
            url = "https://example.com",
            title = "report.txt",
            description = "An embed description that wraps onto a second line inside the card.",
            thumbnailUrl = null,
            html = null,
          ),
        ExternalElementData.Embed(id = "asset") to
          EditorEmbedAsset(
            id = "asset",
            url = "https://example.com",
            title = "report.txt",
            description = "An embed description that wraps onto a second line inside the card.",
            thumbnailUrl = "https://example.com/thumbnail.png",
            html = null,
          ),
      )
    for ((data, asset) in assets) {
      runComposeUiTest {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val fake = FakeFfiEditor()
        val editor = Editor(fake, scope, Dispatchers.Unconfined)
        val runtime = EditorRuntime(scope)
        val externalState = EditorExternalElementState()
        externalState.put(asset)
        val zoom = mutableFloatStateOf(2f)
        val hasThumbnail = (asset as? EditorEmbedAsset)?.thumbnailUrl != null
        val thumbnailSizes = CopyOnWriteArrayList<Size>()
        val previousImageLoader = SingletonImageLoader.get(PlatformContext.INSTANCE)
        val imageLoader =
          ImageLoader.Builder(PlatformContext.INSTANCE)
            .components {
              add(
                Interceptor { chain ->
                  thumbnailSizes.add(chain.size)
                  SuccessResult(image = ColorImage(), request = chain.request)
                }
              )
            }
            .build()
        val element =
          ExternalElement(
            pageIdx = 0,
            node = "external",
            bounds = Rect(0f, 0f, 200f, 64f),
            isSelected = false,
            data = data,
          )
        fun reportedHeights() =
          fake.enqueued.filterIsInstance<Message.System>().mapNotNull {
            (it.event as? SystemEvent.SetExternalHeight)?.height
          }

        try {
          SingletonImageLoader.setUnsafe(imageLoader)
          runtime.attach(editor)
          setContent {
            CompositionLocalProvider(
              LocalDensity provides Density(3f),
              LocalThemeMode provides ResolvedThemeMode.Light,
              LocalEditorRuntime provides runtime,
              LocalEditorUiState provides EditorUiState(),
              LocalEditorExternalElementState provides externalState,
            ) {
              Box(Modifier.requiredSize(300.dp, 220.dp)) {
                EditorExternalElementOverlay(listOf(element), displayZoom = zoom.floatValue)
              }
            }
          }
          waitUntil { reportedHeights().isNotEmpty() }
          val originalHeight = runOnIdle { reportedHeights().single() }
          if (hasThumbnail) {
            waitUntil { thumbnailSizes.isNotEmpty() }
            assertEquals(Size(708, 708), thumbnailSizes.single())
            assertEquals(118f, originalHeight)
          }
          val originalTextHeight =
            onNodeWithText("report.txt").fetchSemanticsNode().boundsInRoot.height
          assertTrue(originalTextHeight > 0f)
          for ((scale, thumbnailPixels) in
            listOf(0.13f to 46, 0.128f to 45, 0.127f to 45, 2.1f to 743)) {
            runOnIdle { zoom.floatValue = scale }
            waitForIdle()
            runOnIdle { assertEquals(listOf(originalHeight), reportedHeights()) }
            assertEquals(
              originalTextHeight * scale / 2f,
              onNodeWithText("report.txt").fetchSemanticsNode().boundsInRoot.height,
              1f,
            )
            if (hasThumbnail) {
              waitUntil { thumbnailSizes.lastOrNull() == Size(thumbnailPixels, thumbnailPixels) }
            }
          }

          runOnIdle {
            externalState.clear()
            externalState.resolutions["asset"] = EditorAssetResolution.Unavailable
          }
          waitUntil { reportedHeights().lastOrNull() == 48f }
          runOnIdle { assertEquals(listOf(originalHeight, 48f), reportedHeights()) }
        } finally {
          SingletonImageLoader.setUnsafe(previousImageLoader)
          imageLoader.shutdown()
          runtime.clear()
          scope.cancel()
        }
      }
    }
  }
}
