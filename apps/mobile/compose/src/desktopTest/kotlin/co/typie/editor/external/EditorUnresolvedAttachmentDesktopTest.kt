package co.typie.editor.external

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.DocumentEditingSession
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.PlainNode
import co.typie.platform.IncomingContentItem
import co.typie.platform.PickedFile
import co.typie.screen.editor.editor.attachment.EditorAttachmentDestination
import co.typie.screen.editor.editor.attachment.EditorAttachmentImporter
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentImporter
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.contextual.editorFileToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorImageToolbarPage
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob

// 스펙: ID가 붙어 있어도 서버가 missing으로 확정한 노드는 빈 placeholder와 **같은 모습·어포던스**여야 한다
// (누구나 자기 기기에서 이어받을 수 있어야 하고, 노드에 ID가 있는지는 사용자에게 드러나지 않는다).
@OptIn(ExperimentalTestApi::class)
class EditorUnresolvedAttachmentDesktopTest {
  @Test
  fun unresolvedImageLooksExactlyLikeAnEmptyPlaceholder() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["IMG1"] = EditorAssetResolution.Missing

    setContent { ImageElementHost(state = state, assetId = "IMG1") }
    waitForIdle()

    onNodeWithText("이미지").assertExists()
    onNodeWithText("이미지를 불러올 수 없어요").assertDoesNotExist()
  }

  @Test
  fun pendingImageStillShowsThePeerUploadCard() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["IMG1"] =
      EditorAssetResolution.Pending(
        EditorAssetPendingMeta(kind = "image", name = "peer.png", size = 10)
      )

    setContent { ImageElementHost(state = state, assetId = "IMG1") }
    waitForIdle()

    onNodeWithText("peer.png").assertExists()
  }

  @Test
  fun unresolvedFileLooksExactlyLikeAnEmptyPlaceholder() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["FILE1"] = EditorAssetResolution.Missing

    setContent { FileElementHost(state = state, assetId = "FILE1") }
    waitForIdle()

    onNodeWithText("파일").assertExists()
    onNodeWithText("파일을 불러올 수 없어요").assertDoesNotExist()
  }

  @Test
  fun imageToolbarOpensThePickerForAnUnresolvedNodeWithItsExistingId() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["IMG1"] = EditorAssetResolution.Missing
    val picks = mutableListOf<String>()

    setContent {
      ToolbarHost(state) {
        editorImageToolbarPage(
          image = PlainNode.Image(id = "IMG1"),
          nodeId = "node-a",
          pickImage = { nodeId, reclaimAssetId -> picks += "$nodeId|$reclaimAssetId" },
        )
      }
    }
    waitForIdle()
    onNodeWithContentDescription("이미지 선택").performClick()
    waitForIdle()

    // 새 ID를 발급하지 않고 문서에 적힌 ID를 그대로 재예약해야 한다.
    assertEquals(listOf("node-a|IMG1"), picks)
  }

  @Test
  fun imageToolbarHidesThePickerWhileAPeerUploadIsPending() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["IMG1"] =
      EditorAssetResolution.Pending(
        EditorAssetPendingMeta(kind = "image", name = "peer.png", size = 10)
      )

    setContent {
      ToolbarHost(state) {
        editorImageToolbarPage(
          image = PlainNode.Image(id = "IMG1"),
          nodeId = "node-a",
          pickImage = { _, _ -> },
        )
      }
    }
    waitForIdle()

    onNodeWithContentDescription("이미지 선택").assertDoesNotExist()
    onNodeWithContentDescription("이미지 삭제").assertExists()
  }

  @Test
  fun fileToolbarOpensThePickerForAnUnresolvedNodeWithItsExistingId() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["FILE1"] = EditorAssetResolution.Missing
    val picks = mutableListOf<String>()

    setContent {
      ToolbarHost(state) {
        editorFileToolbarPage(
          file = PlainNode.File(id = "FILE1"),
          nodeId = "node-a",
          pickFile = { nodeId, reclaimAssetId -> picks += "$nodeId|$reclaimAssetId" },
        )
      }
    }
    waitForIdle()
    onNodeWithContentDescription("파일 첨부").performClick()
    waitForIdle()

    assertEquals(listOf("node-a|FILE1"), picks)
  }

  @Test
  fun fileToolbarHidesThePickerWhileAPeerUploadIsPending() = runComposeUiTest {
    val state = EditorExternalElementState()
    state.resolutions["FILE1"] =
      EditorAssetResolution.Pending(
        EditorAssetPendingMeta(kind = "file", name = "peer.pdf", size = 10)
      )

    setContent {
      ToolbarHost(state) {
        editorFileToolbarPage(
          file = PlainNode.File(id = "FILE1"),
          nodeId = "node-a",
          pickFile = { _, _ -> },
        )
      }
    }
    waitForIdle()

    onNodeWithContentDescription("파일 첨부").assertDoesNotExist()
  }

  @Composable
  private fun ImageElementHost(state: EditorExternalElementState, assetId: String) {
    ElementHost(state) {
      EditorImageExternalElement(
        data = ExternalElementData.Image(id = assetId, proportion = 100),
        nodeId = "node-a",
        boundsWidth = 400f,
      )
    }
  }

  @Composable
  private fun FileElementHost(state: EditorExternalElementState, assetId: String) {
    ElementHost(state) {
      EditorFileExternalElement(
        data = ExternalElementData.File(id = assetId),
        nodeId = "node-a",
      )
    }
  }

  @Composable
  private fun ElementHost(
    state: EditorExternalElementState,
    content: @Composable context(EditorExternalElementRenderScope) () -> Unit,
  ) {
    val renderScope = remember {
      EditorExternalElementRenderScope(zoom = 1f, shape = AppShapes.rounded(4.dp))
    }

    Theme {
      CompositionLocalProvider(LocalEditorExternalElementState provides state) {
        Box(modifier = Modifier.size(width = 400.dp, height = 200.dp)) {
          context(renderScope) { content() }
        }
      }
    }
  }

  @Composable
  private fun ToolbarHost(state: EditorExternalElementState, page: () -> EditorToolbarPage) {
    val toolbarPage = remember(page) { page() }
    val scope = rememberToolbarPageScope(toolbarPage)

    Theme {
      CompositionLocalProvider(
        LocalEditorExternalElementState provides state,
        LocalEditorAttachmentImporter provides NoopImporter,
      ) {
        Box(modifier = Modifier.size(width = 400.dp, height = 56.dp)) { toolbarPage.content(scope) }
      }
    }
  }

  @Composable
  private fun rememberToolbarPageScope(page: EditorToolbarPage): EditorToolbarPageScope =
    remember(page) {
      EditorToolbarPageScope(
        toolbarScope = page.toolbarScope,
        activeBottomPanel = null,
        activeSecondaryToolbar = null,
        commandScope = CoroutineScope(SupervisorJob()),
        hasNextPage = false,
        navigateToPage = {},
        onSecondaryToolbarToggle = { _, _ -> },
        clearSecondaryToolbar = {},
        onBottomPanelToggle = { _, _ -> },
        sendMessage = {},
        performToolAction = {},
      )
    }

  @Composable
  private fun Theme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      content = content,
    )
  }

  private object NoopImporter : EditorAttachmentImporter {
    override suspend fun import(
      session: DocumentEditingSession,
      items: List<IncomingContentItem>,
      destination: EditorAttachmentDestination,
      onCompleted: (importedCount: Int) -> Unit,
    ): Boolean = false

    override fun retry(assetId: String): Boolean = false

    override fun canRetry(assetId: String): Boolean = false

    override fun reselect(
      session: DocumentEditingSession,
      assetId: String,
      nodeId: String,
      kind: IncomingContentItem.Kind,
      file: PickedFile,
      onFailure: () -> Unit,
    ): Boolean = false

    override fun retireCompleted(assetIds: Collection<String>) = Unit

    override fun cancelNode(nodeId: String) = Unit

    override fun setForeground(foreground: Boolean) = Unit

    override fun dispose() = Unit
  }
}
