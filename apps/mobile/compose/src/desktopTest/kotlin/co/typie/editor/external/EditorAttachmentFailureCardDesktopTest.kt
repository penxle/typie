package co.typie.editor.external

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.icons.Lucide
import co.typie.platform.IncomingContentItem
import co.typie.platform.PickedFile
import co.typie.screen.editor.editor.attachment.EditorAttachmentDestination
import co.typie.screen.editor.editor.attachment.EditorAttachmentImporter
import co.typie.screen.editor.editor.attachment.EditorAttachmentPicker
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentImporter
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentPicker
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorAttachmentFailureCardDesktopTest {
  @Test
  fun rendersTheRetryReselectAndDeleteAffordances() = runComposeUiTest {
    val importer = RecordingImporter(canRetry = true)

    setContent { FailureCardHost(importer = importer) }
    waitForIdle()

    onNodeWithText("재시도").assertExists()
    onNodeWithText("다시 선택").assertExists()
    onNodeWithContentDescription("이미지 삭제").assertExists()
  }

  @Test
  fun hidesRetryWhenTheRetainedSourceIsGone() = runComposeUiTest {
    val importer = RecordingImporter(canRetry = false)

    setContent { FailureCardHost(importer = importer) }
    waitForIdle()

    onNodeWithText("재시도").assertDoesNotExist()
    onNodeWithText("다시 선택").assertExists()
  }

  @Test
  fun retryTapsReachTheImporter() = runComposeUiTest {
    val importer = RecordingImporter(canRetry = true)

    setContent { FailureCardHost(importer = importer) }
    waitForIdle()
    onNodeWithText("재시도").performClick()
    waitForIdle()

    assertEquals(listOf("IMG1"), importer.retried)
  }

  @Test
  fun reselectTapsReachThePickerWithTheNodeAndAssetId() = runComposeUiTest {
    val importer = RecordingImporter(canRetry = true)
    val picks = mutableListOf<String>()

    setContent {
      FailureCardHost(
        importer = importer,
        picker = { kind, nodeId, assetId -> picks += "$kind|$nodeId|$assetId" },
      )
    }
    waitForIdle()
    onNodeWithText("다시 선택").performClick()
    waitForIdle()

    assertEquals(listOf("Image|node-a|IMG1"), picks)
  }

  @Test
  fun deleteRetiresTheRunAndStillEnqueuesTheDelete() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val editorScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val runtimeScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, editorScope)
    val runtime = EditorRuntime(runtimeScope)
    val session = createTestDocumentEditingSession(editor, editorScope)
    runtime.attach(session)
    val importer = RecordingImporter(canRetry = true)

    try {
      setContent { FailureCardHost(importer = importer, runtime = runtime) }
      waitForIdle()
      onNodeWithContentDescription("이미지 삭제").performClick()
      waitForIdle()
      waitUntil { fake.enqueued.any { it is Message.Node } }

      assertEquals(listOf("node-a"), importer.cancelledNodes)
      assertEquals(
        listOf(NodeOp.Delete(id = "node-a")),
        fake.enqueued.filterIsInstance<Message.Node>().map(Message.Node::op),
      )
    } finally {
      runtime.clear(session)
      editorScope.cancel()
      runtimeScope.cancel()
    }
  }

  @Test
  fun deleteWithoutAnAttachedSessionStillRetiresTheRunAndTouchesNoDocument() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val runtimeScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val runtime = EditorRuntime(runtimeScope)
    val importer = RecordingImporter(canRetry = true)

    try {
      setContent { FailureCardHost(importer = importer, runtime = runtime) }
      waitForIdle()
      onNodeWithContentDescription("이미지 삭제").performClick()
      waitForIdle()

      assertEquals(listOf("node-a"), importer.cancelledNodes)
      assertTrue(fake.enqueued.isEmpty())
    } finally {
      runtimeScope.cancel()
    }
  }

  @Test
  fun tappingTheCardDoesNotPullFocusOffTheEditor() = runComposeUiTest {
    val importer = RecordingImporter(canRetry = true)

    setContent {
      val focusRequester = remember { FocusRequester() }

      Column {
        Box(Modifier.testTag(EditorFocusTag).size(48.dp).focusRequester(focusRequester).focusable())
        FailureCardHost(importer = importer)
      }

      LaunchedEffect(Unit) { focusRequester.requestFocus() }
    }
    waitForIdle()

    onNodeWithText("재시도").performClick()
    waitForIdle()

    assertEquals(listOf("IMG1"), importer.retried)
    onNodeWithTag(EditorFocusTag).assertIsFocused()
  }

  @androidx.compose.runtime.Composable
  private fun FailureCardHost(
    importer: EditorAttachmentImporter,
    runtime: EditorRuntime = remember { EditorRuntime(CoroutineScope(SupervisorJob())) },
    picker: (IncomingContentItem.Kind, String, String?) -> Unit = { _, _, _ -> },
  ) {
    val renderScope = remember {
      EditorExternalElementRenderScope(zoom = 1f, shape = AppShapes.rounded(4.dp))
    }

    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalEditorRuntime provides runtime,
      LocalEditorAttachmentImporter provides importer,
      LocalEditorAttachmentPicker provides
        EditorAttachmentPicker { kind, nodeId, assetId -> picker(kind, nodeId, assetId) },
    ) {
      context(renderScope) {
        EditorAttachmentFailureCard(
          icon = Lucide.Image,
          text = "이미지를 업로드하지 못했어요",
          deleteContentDescription = "이미지 삭제",
          kind = IncomingContentItem.Kind.Image,
          assetId = "IMG1",
          nodeId = "node-a",
        )
      }
    }
  }

  private class RecordingImporter(private val canRetry: Boolean) : EditorAttachmentImporter {
    val retried = mutableListOf<String>()
    val cancelledNodes = mutableListOf<String>()

    override suspend fun import(
      session: DocumentEditingSession,
      items: List<IncomingContentItem>,
      destination: EditorAttachmentDestination,
      onCompleted: (importedCount: Int) -> Unit,
    ): Boolean = false

    override fun retry(assetId: String): Boolean {
      retried += assetId
      return true
    }

    override fun canRetry(assetId: String): Boolean = canRetry

    override fun reselect(
      session: DocumentEditingSession,
      assetId: String,
      nodeId: String,
      kind: IncomingContentItem.Kind,
      file: PickedFile,
      onFailure: () -> Unit,
    ): Boolean = false

    override fun retireCompleted(assetIds: Collection<String>) = Unit

    override fun cancelNode(nodeId: String) {
      cancelledNodes += nodeId
    }

    override fun setForeground(foreground: Boolean) = Unit

    override fun dispose() = Unit
  }

  private companion object {
    const val EditorFocusTag = "editor-focus-target"
  }
}
