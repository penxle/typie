package co.typie.domain.note

import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class NoteEditStateTest {
  @Test
  fun `open tracks expanded note`() = runTest {
    val state = NoteEditState(scope = this)
    val note = notesNote(id = "existing")

    state.open(note = note)

    assertEquals("existing", state.expandedNoteId)
  }

  @Test
  fun `debounced content save runs after delay`() = runTest {
    val state = NoteEditState(scope = this)
    val saved = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note"))

    state.updateContent(siteId = "site", noteId = "note", value = "updated") { noteId, content ->
      delay(1)
      saved += noteId to content
      NoteSaveOutcome.Saved
    }

    advanceTimeBy(299)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertEquals(listOf("note" to "updated"), saved)
  }

  @Test
  fun `debounced color save runs after delay`() = runTest {
    val state = NoteEditState(scope = this)
    val saved = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note", color = "gray"))

    state.updateColor(siteId = "site", noteId = "note", value = "red") { noteId, color ->
      delay(1)
      saved += noteId to color
      NoteSaveOutcome.Saved
    }

    advanceTimeBy(179)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertEquals(listOf("note" to "red"), saved)
  }

  @Test
  fun `overlay preserves latest server snapshot fields while applying local drafts`() = runTest {
    val state = NoteEditState(scope = this)
    val linkedEntity = notesDocumentEntity(id = "entity-1")
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft content") { _, _ ->
      NoteSaveOutcome.Saved
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      NoteSaveOutcome.Saved
    }

    val overlaid =
      state.commitServerSnapshot(
        notesNote(
          id = "note",
          content = "server snapshot",
          color = "gray",
          status = NoteStatus.RESOLVED,
          updatedAt = Instant.parse("2024-01-02T00:00:00Z"),
          entities = listOf(linkedEntity),
        )
      )

    assertEquals("draft content", overlaid.content)
    assertEquals("red", overlaid.color)
    assertEquals(NoteStatus.RESOLVED, overlaid.status)
    assertEquals(Instant.parse("2024-01-02T00:00:00Z"), overlaid.updatedAt)
    assertEquals(listOf(linkedEntity.id), overlaid.entities.map { it.noteLinkedEntity_entity.id })
    assertEquals(
      listOf(linkedEntity.id),
      state.overlay(notesNote(id = "note", content = "server", color = "gray")).entities.map {
        it.noteLinkedEntity_entity.id
      },
    )
  }

  @Test
  fun `unrelated note snapshot does not clear active draft`() = runTest {
    val state = NoteEditState(scope = this)
    state.open(note = notesNote(id = "open", color = "gray"))
    state.updateColor(siteId = "site", noteId = "open", value = "red") { _, _ ->
      NoteSaveOutcome.Saved
    }

    state.commitServerSnapshot(
      notesNote(id = "other", color = "blue", status = NoteStatus.RESOLVED)
    )

    assertEquals("red", state.overlay(notesNote(id = "open", color = "gray")).color)
    assertTrue(state.hasPendingColor(siteId = "site", noteId = "open"))
  }

  @Test
  fun `the same note id in another site cannot reuse or overwrite the active draft`() = runTest {
    val state = NoteEditState(scope = this)
    state.open(notesNote(id = "shared", siteId = "site-a", content = "site a"))
    state.updateContent(siteId = "site-a", noteId = "shared", value = "site a draft") { _, _ ->
      NoteSaveOutcome.Saved
    }

    val siteBSnapshot = notesNote(id = "shared", siteId = "site-b", content = "site b")
    assertEquals("site b", state.commitServerSnapshot(siteBSnapshot).content)
    assertEquals("site b", state.overlay(siteBSnapshot).content)

    state.open(siteBSnapshot)
    state.updateContent(siteId = "site-a", noteId = "shared", value = "late site a draft") { _, _ ->
      NoteSaveOutcome.Saved
    }

    assertEquals("site-b", state.expandedNoteSiteId)
    assertEquals("site b", state.overlay(siteBSnapshot).content)
    assertFalse(state.isDirty(siteId = "site-b", noteId = "shared"))
  }

  @Test
  fun `flush persists both content and color before collapse`() = runTest {
    val state = NoteEditState(scope = this)
    val contentSaves = mutableListOf<Pair<String, String>>()
    val colorSaves = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(siteId = "site", noteId = "note", value = "local content") { noteId, content
      ->
      contentSaves += noteId to content
      NoteSaveOutcome.Saved
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { noteId, color ->
      colorSaves += noteId to color
      NoteSaveOutcome.Saved
    }

    val collapsed =
      state.collapse(
        siteId = "site",
        saveColor = { noteId, color ->
          colorSaves += noteId to color
          NoteSaveOutcome.Saved
        },
        saveContent = { noteId, content ->
          contentSaves += noteId to content
          NoteSaveOutcome.Saved
        },
      )

    assertTrue(collapsed)
    assertEquals(listOf("note" to "local content"), contentSaves)
    assertEquals(listOf("note" to "red"), colorSaves)
    assertEquals(null, state.expandedNoteId)
  }

  @Test
  fun `dispose saves pending drafts and keeps expanded note`() = runTest {
    val state = NoteEditState(scope = this)
    val contentSaves = mutableListOf<Pair<String, String>>()
    val colorSaves = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(siteId = "site", noteId = "note", value = "local content") { noteId, content
      ->
      contentSaves += noteId to content
      NoteSaveOutcome.Saved
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { noteId, color ->
      colorSaves += noteId to color
      NoteSaveOutcome.Saved
    }

    state.dispose(
      savePendingContent = { _, noteId, content ->
        contentSaves += noteId to content
        NoteSaveOutcome.Saved
      },
      savePendingColor = { _, noteId, color ->
        colorSaves += noteId to color
        NoteSaveOutcome.Saved
      },
    )
    runCurrent()

    assertEquals(listOf("note" to "local content"), contentSaves)
    assertEquals(listOf("note" to "red"), colorSaves)
    assertEquals("note", state.expandedNoteId)
    assertEquals(
      notesNote(id = "note", content = "local content", color = "red"),
      state.overlay(notesNote(id = "note", content = "server", color = "gray")),
    )
  }

  @Test
  fun `route removal flushes pending drafts with their site identity`() = runTest {
    val state = NoteEditState(scope = this)
    val saved = mutableListOf<Triple<String, String, String>>()
    state.open(note = notesNote(id = "note", siteId = "site", content = "server", color = "gray"))
    state.updateContent(siteId = "site", noteId = "note", value = "local content") { _, _ ->
      error("the debounced surface callback must not run")
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      error("the debounced surface callback must not run")
    }

    val flushed =
      state.flushPendingEdits(
        savePendingContent = { siteId, noteId, content ->
          saved += Triple(siteId, noteId, content)
          NoteSaveOutcome.Saved
        },
        savePendingColor = { siteId, noteId, color ->
          saved += Triple(siteId, noteId, color)
          NoteSaveOutcome.Saved
        },
      )

    assertTrue(flushed)
    assertEquals(
      listOf(Triple("site", "note", "red"), Triple("site", "note", "local content")),
      saved,
    )
  }

  @Test
  fun `older content save completion does not clear newer draft before snapshot`() = runTest {
    val state = NoteEditState(scope = this)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      saved += content
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      NoteSaveOutcome.Saved
    }

    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, content ->
      saved += content
      NoteSaveOutcome.Saved
    }
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))

    finishFirstSave.complete(Unit)
    runCurrent()

    assertEquals(listOf("draft A"), saved)
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))

    val overlaid = state.commitServerSnapshot(notesNote(id = "note", content = "draft A"))
    assertEquals("draft B", overlaid.content)
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `same field saves are serialized so the latest draft is written last`() = runTest {
    val state = NoteEditState(scope = this)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      saved += content
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, content ->
      saved += content
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()
    assertEquals(listOf("draft A"), saved)

    finishFirstSave.complete(Unit)
    runCurrent()

    assertEquals(listOf("draft A", "draft B"), saved)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
  }

  @Test
  fun `content and color saves are serialized so full snapshots cannot arrive out of order`() =
    runTest {
      val state = NoteEditState(scope = this)
      val contentStarted = CompletableDeferred<Unit>()
      val finishContent = CompletableDeferred<Unit>()
      val colorStarted = CompletableDeferred<Unit>()
      val finishColor = CompletableDeferred<Unit>()
      val saves = mutableListOf<String>()
      state.open(note = notesNote(id = "note", content = "server", color = "gray"))

      state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
        saves += "content"
        contentStarted.complete(Unit)
        finishContent.await()
        NoteSaveOutcome.Saved
      }
      advanceTimeBy(300)
      runCurrent()
      contentStarted.await()

      state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
        saves += "color"
        colorStarted.complete(Unit)
        finishColor.await()
        NoteSaveOutcome.Saved
      }
      advanceTimeBy(180)
      runCurrent()

      assertFalse(colorStarted.isCompleted)
      assertEquals(listOf("content"), saves)

      finishContent.complete(Unit)
      runCurrent()
      colorStarted.await()

      assertEquals(listOf("content", "color"), saves)
      finishColor.complete(Unit)
      runCurrent()
    }

  @Test
  fun `saving status stays hidden until the request has lasted 500ms`() = runTest {
    val state = NoteEditState(scope = this)
    val saveStarted = CompletableDeferred<Unit>()
    val finishSave = CompletableDeferred<Unit>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      saveStarted.complete(Unit)
      finishSave.await()
      NoteSaveOutcome.Saved
    }

    advanceTimeBy(300)
    runCurrent()
    saveStarted.await()
    assertTrue(state.isSaving(siteId = "site", noteId = "note"))
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))

    advanceTimeBy(499)
    runCurrent()
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))

    advanceTimeBy(1)
    runCurrent()
    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

    finishSave.complete(Unit)
    runCurrent()
    assertFalse(state.isSaving(siteId = "site", noteId = "note"))
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `saving status uses one interval across serialized content and color requests`() = runTest {
    val state = NoteEditState(scope = this)
    val contentStarted = CompletableDeferred<Unit>()
    val finishContent = CompletableDeferred<Unit>()
    val colorStarted = CompletableDeferred<Unit>()
    val finishColor = CompletableDeferred<Unit>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      contentStarted.complete(Unit)
      finishContent.await()
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()
    contentStarted.await()

    advanceTimeBy(400)
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      colorStarted.complete(Unit)
      finishColor.await()
      NoteSaveOutcome.Saved
    }
    runCurrent()
    assertFalse(colorStarted.isCompleted)

    advanceTimeBy(180)
    runCurrent()
    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

    finishContent.complete(Unit)
    runCurrent()
    colorStarted.await()
    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

    finishColor.complete(Unit)
    runCurrent()
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `saving status stays visible while a newer content save waits for the active request`() =
    runTest {
      val state = NoteEditState(scope = this)
      val firstSaveStarted = CompletableDeferred<Unit>()
      val finishFirstSave = CompletableDeferred<Unit>()
      val secondSaveStarted = CompletableDeferred<Unit>()
      val finishSecondSave = CompletableDeferred<Unit>()
      state.open(note = notesNote(id = "note", content = "server"))

      state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, _ ->
        firstSaveStarted.complete(Unit)
        finishFirstSave.await()
        NoteSaveOutcome.Saved
      }
      advanceTimeBy(300)
      runCurrent()
      firstSaveStarted.await()
      advanceTimeBy(500)
      runCurrent()
      assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

      state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, _ ->
        secondSaveStarted.complete(Unit)
        finishSecondSave.await()
        NoteSaveOutcome.Saved
      }
      runCurrent()

      finishFirstSave.complete(Unit)
      runCurrent()
      assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

      advanceTimeBy(300)
      runCurrent()
      secondSaveStarted.await()
      finishSecondSave.complete(Unit)
      runCurrent()
      assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
    }

  @Test
  fun `authoritative content matching a queued save ends the retained saving interval`() = runTest {
    val state = NoteEditState(scope = this)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      state.commitServerSnapshot(notesNote(id = "note", content = content))
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()
    advanceTimeBy(500)
    runCurrent()

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, _ ->
      NoteSaveOutcome.Saved
    }
    finishFirstSave.complete(Unit)
    runCurrent()
    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

    state.commitServerSnapshot(notesNote(id = "note", content = "draft B"))
    runCurrent()

    assertFalse(state.isSaving(siteId = "site", noteId = "note"))
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `reverting content while a save is in flight persists the reverted value`() = runTest {
    val state = NoteEditState(scope = this)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    val save: suspend (String, String) -> NoteSaveOutcome = { _, content ->
      saved += content
      if (saved.size == 1) {
        firstSaveStarted.complete(Unit)
        finishFirstSave.await()
      }
      state.commitServerSnapshot(notesNote(id = "note", content = content))
      NoteSaveOutcome.Saved
    }

    state.updateContent(siteId = "site", noteId = "note", value = "draft", save = save)
    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "server", save = save)
    finishFirstSave.complete(Unit)
    runCurrent()
    advanceTimeBy(300)
    runCurrent()

    assertEquals(listOf("draft", "server"), saved)
    assertEquals("server", state.overlay(notesNote(id = "note", content = "draft")).content)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `reverting content compensates for an ambiguous in flight failure`() = runTest {
    val state = NoteEditState(scope = this)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    val save: suspend (String, String) -> NoteSaveOutcome = { _, content ->
      saved += content
      if (saved.size == 1) {
        firstSaveStarted.complete(Unit)
        finishFirstSave.await()
        NoteSaveOutcome.Failed
      } else {
        NoteSaveOutcome.Saved
      }
    }

    state.updateContent(siteId = "site", noteId = "note", value = "draft", save = save)
    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "server", save = save)
    finishFirstSave.complete(Unit)
    runCurrent()
    advanceTimeBy(300)
    runCurrent()

    assertEquals(listOf("draft", "server"), saved)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `failed save keeps the draft and the next edit saves the latest value`() = runTest {
    val state = NoteEditState(scope = this)
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      saved += content
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()

    assertEquals(NoteSaveStatus.FAILED, state.saveStatus(siteId = "site", noteId = "note"))
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals("draft A", state.overlay(notesNote(id = "note", content = "server")).content)

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, content ->
      saved += content
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()

    assertEquals(listOf("draft A", "draft B"), saved)
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
  }

  @Test
  fun `content and color failures emit once per aggregate failure episode`() = runTest {
    val state = NoteEditState(scope = this)
    val failures = mutableListOf<Unit>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.saveFailures.collect { failures += it }
    }
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, _ ->
      NoteSaveOutcome.Failed
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()

    assertEquals(listOf(Unit), failures)
    assertEquals(NoteSaveStatus.FAILED, state.saveStatus(siteId = "site", noteId = "note"))

    val contentStarted = CompletableDeferred<Unit>()
    val finishContent = CompletableDeferred<Unit>()
    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, _ ->
      contentStarted.complete(Unit)
      finishContent.await()
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()
    contentStarted.await()

    assertEquals(NoteSaveStatus.FAILED, state.saveStatus(siteId = "site", noteId = "note"))

    val colorStarted = CompletableDeferred<Unit>()
    val finishColor = CompletableDeferred<Unit>()
    state.updateColor(siteId = "site", noteId = "note", value = "blue") { _, _ ->
      colorStarted.complete(Unit)
      finishColor.await()
      NoteSaveOutcome.Failed
    }
    runCurrent()
    assertFalse(colorStarted.isCompleted)

    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))

    finishContent.complete(Unit)
    runCurrent()
    advanceTimeBy(180)
    runCurrent()
    colorStarted.await()
    finishColor.complete(Unit)
    runCurrent()

    assertEquals(listOf(Unit, Unit), failures)
    assertEquals(NoteSaveStatus.FAILED, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `older failed save does not report failure for a newer queued draft`() = runTest {
    val state = NoteEditState(scope = this)
    val failures = mutableListOf<Unit>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.saveFailures.collect { failures += it }
    }
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      saved += content
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, content ->
      saved += content
      NoteSaveOutcome.Saved
    }
    runCurrent()

    assertEquals(listOf("draft A"), saved)
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)

    finishFirstSave.complete(Unit)
    runCurrent()
    advanceTimeBy(300)
    runCurrent()

    assertTrue(failures.isEmpty())
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
    assertEquals(listOf("draft A", "draft B"), saved)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
  }

  @Test
  fun `subscription gated save keeps the draft without reporting a failure`() = runTest {
    val state = NoteEditState(scope = this)
    val failures = mutableListOf<Unit>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.saveFailures.collect { failures += it }
    }
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      NoteSaveOutcome.SubscriptionGated
    }
    advanceTimeBy(300)
    runCurrent()

    assertTrue(failures.isEmpty())
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals("draft", state.overlay(notesNote(id = "note", content = "server")).content)
  }

  @Test
  fun `focus loss does not retry an unchanged failed or gated revision`() = runTest {
    listOf(NoteSaveOutcome.Failed, NoteSaveOutcome.SubscriptionGated).forEach { outcome ->
      val state = NoteEditState(scope = this)
      var saveCount = 0
      val saveContent: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
        saveCount += 1
        outcome
      }
      state.open(note = notesNote(id = "note-$outcome", content = "server"))

      state.updateContent(
        siteId = "site",
        noteId = "note-$outcome",
        value = "draft",
        save = saveContent,
      )
      runCurrent()

      val flushed =
        state.flush(
          siteId = "site",
          noteId = "note-$outcome",
          saveContent = saveContent,
          saveColor = { _, _ -> NoteSaveOutcome.Saved },
        )

      assertFalse(flushed)
      assertEquals(1, saveCount)
      assertTrue(state.isDirty(siteId = "site", noteId = "note-$outcome"))
    }
  }

  @Test
  fun `subscription gated save does not automatically run a queued newer draft`() = runTest {
    val state = NoteEditState(scope = this)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      saved += content
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      NoteSaveOutcome.SubscriptionGated
    }
    advanceTimeBy(300)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, content ->
      saved += content
      NoteSaveOutcome.Saved
    }
    runCurrent()

    finishFirstSave.complete(Unit)
    runCurrent()

    assertEquals(listOf("draft A"), saved)
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
  }

  @Test
  fun `subscription gated save discards a queued save for the other field`() = runTest {
    val state = NoteEditState(scope = this)
    val contentStarted = CompletableDeferred<Unit>()
    val finishContent = CompletableDeferred<Unit>()
    var colorSaveCount = 0
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      contentStarted.complete(Unit)
      finishContent.await()
      NoteSaveOutcome.SubscriptionGated
    }
    advanceTimeBy(300)
    runCurrent()
    contentStarted.await()

    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      colorSaveCount += 1
      NoteSaveOutcome.Saved
    }
    runCurrent()

    finishContent.complete(Unit)
    runCurrent()

    assertEquals(0, colorSaveCount)
    assertTrue(state.hasPendingColor(siteId = "site", noteId = "note"))
    assertEquals("red", state.overlay(notesNote(id = "note", color = "gray")).color)
  }

  @Test
  fun `cancelled active save ignores its late failure`() = runTest {
    val state = NoteEditState(scope = this)
    val saveStarted = CompletableDeferred<Unit>()
    val finishSave = CompletableDeferred<Unit>()
    val failures = mutableListOf<Unit>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.saveFailures.collect { failures += it }
    }
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      saveStarted.complete(Unit)
      finishSave.await()
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()
    saveStarted.await()

    state.cancelPendingSaves(siteId = "site", noteId = "note")
    finishSave.complete(Unit)
    runCurrent()

    assertTrue(failures.isEmpty())
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `cancelled save completion cannot end a newer saving interval`() = runTest {
    val state = NoteEditState(scope = this)
    val contentSaveStarted = CompletableDeferred<Unit>()
    val finishContentSave = CompletableDeferred<Unit>()
    val colorSaveStarted = CompletableDeferred<Unit>()
    val finishColorSave = CompletableDeferred<Unit>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      contentSaveStarted.complete(Unit)
      finishContentSave.await()
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()
    contentSaveStarted.await()

    state.cancelPendingSaves(siteId = "site", noteId = "note")
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      colorSaveStarted.complete(Unit)
      finishColorSave.await()
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(680)
    runCurrent()

    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))
    assertFalse(colorSaveStarted.isCompleted)

    finishContentSave.complete(Unit)
    runCurrent()
    colorSaveStarted.await()

    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

    finishColorSave.complete(Unit)
    runCurrent()
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `dispose does not retry an unchanged failed draft`() = runTest {
    val state = NoteEditState(scope = this)
    var disposeSaveCount = 0
    state.open(note = notesNote(id = "note", content = "server"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()

    repeat(2) {
      state.dispose(
        savePendingContent = { _, _, _ ->
          disposeSaveCount += 1
          NoteSaveOutcome.Saved
        },
        savePendingColor = { _, _, _ -> NoteSaveOutcome.Saved },
      )
      runCurrent()
    }

    assertEquals(0, disposeSaveCount)
    assertEquals(NoteSaveStatus.FAILED, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `dispose does not duplicate an active save`() = runTest {
    val state = NoteEditState(scope = this)
    val saveStarted = CompletableDeferred<Unit>()
    val finishSave = CompletableDeferred<Unit>()
    var disposeSaveCount = 0
    state.open(note = notesNote(id = "note", content = "server"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      saveStarted.complete(Unit)
      finishSave.await()
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()
    saveStarted.await()

    state.dispose(
      savePendingContent = { _, _, _ ->
        disposeSaveCount += 1
        NoteSaveOutcome.Saved
      },
      savePendingColor = { _, _, _ -> NoteSaveOutcome.Saved },
    )
    finishSave.complete(Unit)
    runCurrent()

    assertEquals(0, disposeSaveCount)
  }

  @Test
  fun `dispose persists a newer draft behind a superseded active save`() = runTest {
    val state = NoteEditState(scope = this)
    val activeSaveStarted = CompletableDeferred<Unit>()
    val finishActiveSave = CompletableDeferred<Unit>()
    val activeSaves = mutableListOf<String>()
    val pendingSaves = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
      activeSaves += content
      activeSaveStarted.complete(Unit)
      finishActiveSave.await()
      NoteSaveOutcome.Superseded
    }
    advanceTimeBy(300)
    runCurrent()
    activeSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft B") { _, _ ->
      error("the disposed surface callback must not save the newer draft")
    }
    state.dispose(
      savePendingContent = { _, _, content ->
        pendingSaves += content
        NoteSaveOutcome.Saved
      },
      savePendingColor = { _, _, _ -> NoteSaveOutcome.Saved },
    )
    finishActiveSave.complete(Unit)
    runCurrent()

    assertEquals(listOf("draft A"), activeSaves)
    assertEquals(listOf("draft B"), pendingSaves)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `dispose replaces a content save that has not reached the network`() = runTest {
    val state = NoteEditState(scope = this)
    val colorSaveStarted = CompletableDeferred<Unit>()
    val finishColorSave = CompletableDeferred<Unit>()
    var disposedSurfaceSaveCount = 0
    val pendingSaves = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      colorSaveStarted.complete(Unit)
      finishColorSave.await()
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(180)
    runCurrent()
    colorSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      disposedSurfaceSaveCount += 1
      NoteSaveOutcome.Saved
    }
    advanceTimeBy(300)
    runCurrent()

    state.dispose(
      savePendingContent = { _, _, content ->
        pendingSaves += content
        NoteSaveOutcome.Saved
      },
      savePendingColor = { _, _, _ -> NoteSaveOutcome.Saved },
    )
    finishColorSave.complete(Unit)
    runCurrent()

    assertEquals(0, disposedSurfaceSaveCount)
    assertEquals(listOf("draft"), pendingSaves)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }
}
