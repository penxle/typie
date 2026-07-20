package co.typie.screen.document.bodysettings

import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import co.typie.editor.Editor
import co.typie.editor.EditorLocalChangesetBus
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.sync.PullResult
import co.typie.editor.sync.PushResult
import co.typie.editor.sync.RemoteChangesetEvent
import co.typie.editor.sync.RemoteChangesetPipeline
import co.typie.editor.sync.SyncHeadsSink
import co.typie.editor.sync.SyncTransport
import co.typie.editor.sync.asSyncEditor
import co.typie.editor.sync.ws.AttachEvent
import co.typie.editor.sync.ws.DocumentSyncBaseline
import co.typie.result.Result
import co.typie.ui.component.editorsettings.EditorStyleSettings
import co.typie.ui.component.editorsettings.toEditorStyleSettings
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

class DocumentBodySettingsSyncTest {
  @Test
  fun bodySettingsEditPublishesLocalChangesetsForAnotherOpenEditor() = runTest {
    val entityId = "body-settings-local-relay"
    val changesets = byteArrayOf(7, 8)
    var heads = byteArrayOf(1)
    val editor =
      Editor(
        FakeFfiEditor(
          onTick = {
            heads = byteArrayOf(2)
            emptyList()
          },
          currentHeadsProvider = { heads },
          localChangesetsSinceProvider = { baseHeads ->
            assertContentEquals(byteArrayOf(1), baseHeads)
            changesets
          },
        ),
        backgroundScope,
        StandardTestDispatcher(testScheduler),
      )
    val store = ViewModelStore()
    val provider =
      ViewModelProvider.create(
        store = store,
        factory = viewModelFactory { initializer { DocumentBodySettingsViewModel(entityId) } },
      )
    val model = provider[DocumentBodySettingsViewModel::class]

    EditorLocalChangesetBus.consume(entityId)
    try {
      val result =
        model.applyAndRelayBodySettings(editor) {
          enqueue(Message.System(SystemEvent.SetFocused(false)))
        }

      assertIs<Result.Ok<Unit>>(result)
      assertContentEquals(changesets, EditorLocalChangesetBus.consume(entityId).single())
    } finally {
      store.clear()
      editor.dispose()
      EditorLocalChangesetBus.consume(entityId)
    }
  }

  @Test
  fun bootstrapAndInstalledLiveSinkApplyEachBundleOnceAndProjectRootState() = runTest {
    val load =
      DocumentBodySettingsLoad(
        graph = byteArrayOf(0),
        baseline =
          DocumentSyncBaseline(seq = "1-0", heads = byteArrayOf(1), durableHeads = byteArrayOf(1)),
      )
    val bootstrap = changesetsEvent(seq = "2-0", value = 2)
    val queuedDuringDrain = changesetsEvent(seq = "3-0", value = 3)
    val received = mutableListOf<Int>()
    var currentValue = 0
    val ffi =
      FakeFfiEditor(
        receiveRemoteChangesetProvider = { payload ->
          currentValue = payload.single().toInt()
          received += currentValue
        },
        onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Doc))) },
        rootAttrsProvider = {
          PlainRootNode(layoutMode = LayoutMode.Continuous(maxWidth = 600 + currentValue))
        },
        rootModifiersProvider = { listOf(EditorModifier.FontSize(1200 + currentValue)) },
      )
    val editor = Editor(ffi, backgroundScope, StandardTestDispatcher(testScheduler))
    val transport = RecordingTransport()
    val sink = RecordingHeadsSink()
    var pipeline: RemoteChangesetPipeline? = null
    var liveBaseline: DocumentSyncBaseline? = null

    assertTrue(load.queue(bootstrap))
    load.activate(
      apply = { event ->
        event.bundles.forEach { editor.receiveRemoteChangeset(it) }
        if (event.seq == "2-0") assertTrue(load.queue(queuedDuringDrain))
      },
      startLive = { baseline ->
        liveBaseline = baseline
        pipeline =
          RemoteChangesetPipeline(
              editor = editor.asSyncEditor(),
              headsSink = sink,
              transport = transport,
              initialSeq = baseline.seq,
              scope = backgroundScope,
              onNeedsReload = {},
            )
            .also { it.start() }
      },
    )

    runCurrent()
    transport.subscriptionEvents.send(remoteEvent(seq = "4-0", value = 4))
    runCurrent()

    assertEquals(listOf(2, 3, 4), received)
    assertEquals("3-0", liveBaseline?.seq)
    assertContentEquals(byteArrayOf(3), liveBaseline?.heads)
    assertEquals(listOf<String?>("3-0"), transport.subscribeCalls)
    assertEquals(LayoutMode.Continuous(maxWidth = 604), editor.rootAttrs?.layoutMode)
    assertEquals(EditorStyleSettings(fontSize = 1204), editor.rootModifiers.toEditorStyleSettings())

    pipeline?.stop()
    editor.dispose()
  }

  @Test
  fun rootModifiersProjectAllSupportedBodySettings() {
    val modifiers =
      listOf(
        EditorModifier.FontFamily("RIDIBatang"),
        EditorModifier.FontSize(1500),
        EditorModifier.FontWeight(700),
        EditorModifier.LetterSpacing(10),
        EditorModifier.LineHeight(180),
        EditorModifier.ParagraphIndent(120),
        EditorModifier.BlockGap(140),
      )

    assertEquals(
      EditorStyleSettings(
        fontFamily = "RIDIBatang",
        fontSize = 1500,
        fontWeight = 700,
        letterSpacing = 10,
        lineHeight = 180,
        paragraphIndent = 120,
        blockGap = 140,
      ),
      modifiers.toEditorStyleSettings(),
    )
  }

  private fun changesetsEvent(seq: String, value: Int) =
    AttachEvent.ChangesetsEvent(
      seq = seq,
      bundles = listOf(byteArrayOf(value.toByte())),
      heads = byteArrayOf(value.toByte()),
      durableHeads = byteArrayOf(value.toByte()),
    )

  private fun remoteEvent(seq: String, value: Int) =
    RemoteChangesetEvent(
      changesets = listOf(byteArrayOf(value.toByte())),
      seq = seq,
      heads = byteArrayOf(value.toByte()),
      durableHeads = byteArrayOf(value.toByte()),
    )
}

private class RecordingTransport : SyncTransport {
  val subscriptionEvents = Channel<RemoteChangesetEvent>(Channel.UNLIMITED)
  val subscribeCalls = mutableListOf<String?>()

  override suspend fun push(changesets: ByteArray): PushResult =
    PushResult(heads = ByteArray(0), durableHeads = ByteArray(0))

  override suspend fun pull(sinceSeq: String?): PullResult =
    PullResult(
      changesets = emptyList(),
      seq = sinceSeq.orEmpty(),
      heads = ByteArray(0),
      durableHeads = ByteArray(0),
      needsReload = false,
    )

  override fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent> {
    subscribeCalls += sinceSeq
    return flow { for (event in subscriptionEvents) emit(event) }
  }
}

private class RecordingHeadsSink : SyncHeadsSink {
  override fun setConfirmedHeads(heads: ByteArray) = Unit

  override fun setDurableHeads(heads: ByteArray) = Unit
}
