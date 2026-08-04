package co.typie.domain.note

import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
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
    val state = createNoteEditState()
    val note = notesNote(id = "existing")

    state.open(note = note)

    assertEquals("existing", state.expandedNoteId)
  }

  @Test
  fun `opening an existing note does not request content autofocus`() = runTest {
    val state = createNoteEditState()
    val note = notesNote(id = "existing")

    state.open(note)

    assertFalse(state.shouldAutoFocusContent(siteId = note.site.id, noteId = note.id))
  }

  @Test
  fun `opening a newly created note requests content autofocus`() = runTest {
    val state = createNoteEditState()
    val note = notesNote(id = "new")

    state.openNew(note)

    assertTrue(state.shouldAutoFocusContent(siteId = note.site.id, noteId = note.id))
  }

  @Test
  fun `debounced content save runs after delay`() = runTest {
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
  fun `route removal flushes pending drafts with their site identity`() = runTest {
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
  fun `superseded older save does not cancel a newer field debounce`() = runTest {
    val state = createNoteEditState()
    val contentSaveStarted = CompletableDeferred<Unit>()
    val finishContentSave = CompletableDeferred<Unit>()
    val savedColors = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      contentSaveStarted.complete(Unit)
      finishContentSave.await()
      NoteSaveOutcome.Superseded
    }
    advanceTimeBy(300)
    runCurrent()
    contentSaveStarted.await()

    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, color ->
      savedColors += color
      NoteSaveOutcome.Saved
    }
    finishContentSave.complete(Unit)
    runCurrent()
    advanceTimeBy(180)
    runCurrent()

    assertEquals(listOf("red"), savedColors)
    assertFalse(state.hasPendingColor(siteId = "site", noteId = "note"))
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `same field saves are serialized so the latest draft is written last`() = runTest {
    val state = createNoteEditState()
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
      val state = createNoteEditState()
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
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
      val state = createNoteEditState()
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
  fun `authoritative content matching desired does not clear the local save obligation`() =
    runTest {
      val state = createNoteEditState()
      val firstSaveStarted = CompletableDeferred<Unit>()
      val finishFirstSave = CompletableDeferred<Unit>()
      val saved = mutableListOf<String>()
      state.open(note = notesNote(id = "note", content = "server"))

      state.updateContent(siteId = "site", noteId = "note", value = "draft A") { _, content ->
        saved += content
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
        saved += "draft B"
        NoteSaveOutcome.Saved
      }
      finishFirstSave.complete(Unit)
      runCurrent()
      assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

      state.commitServerSnapshot(notesNote(id = "note", content = "draft B"))
      runCurrent()

      assertTrue(state.isDirty(siteId = "site", noteId = "note"))

      advanceTimeBy(300)
      runCurrent()

      assertEquals(listOf("draft A", "draft B"), saved)
      assertFalse(state.isDirty(siteId = "site", noteId = "note"))
      assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
    }

  @Test
  fun `reverting content while a save is in flight persists the reverted value`() = runTest {
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
  fun `flush saves a reverted color after the older color save fails`() = runTest {
    val state = createNoteEditState()
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", color = "gray"))

    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, color ->
      saved += color
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(180)
    runCurrent()
    firstSaveStarted.await()

    state.updateColor(siteId = "site", noteId = "note", value = "gray") { _, _ ->
      error("flush must own the reverted save")
    }
    finishFirstSave.complete(Unit)
    runCurrent()

    val flushed =
      state.flush(
        siteId = "site",
        noteId = "note",
        saveContent = { _, _ -> NoteSaveOutcome.Saved },
        saveColor = { _, color ->
          saved += color
          NoteSaveOutcome.Saved
        },
      )

    assertTrue(flushed)
    assertEquals(listOf("red", "gray"), saved)
    assertFalse(state.hasPendingColor(siteId = "site", noteId = "note"))
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `failed save keeps the draft and the next edit saves the latest value`() = runTest {
    val state = createNoteEditState()
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
  fun `reopening the same live session keeps a failed desired value`() = runTest {
    val state = createNoteEditState()
    val serverNote = notesNote(id = "note", content = "server")
    state.open(note = serverNote)
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()

    state.open(note = serverNote)

    assertEquals("draft", state.overlay(serverNote).content)
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))
    assertEquals(NoteSaveStatus.FAILED, state.saveStatus(siteId = "site", noteId = "note"))
  }

  @Test
  fun `content and color failures emit once per aggregate failure episode`() = runTest {
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
    val state = createNoteEditState()
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
  fun `a later flush retries an unchanged failed or gated revision`() = runTest {
    listOf(NoteSaveOutcome.Failed, NoteSaveOutcome.SubscriptionGated).forEach { outcome ->
      val state = createNoteEditState()
      var saveCount = 0
      val saveContent: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
        saveCount += 1
        if (saveCount == 1) outcome else NoteSaveOutcome.Saved
      }
      state.open(note = notesNote(id = "note-$outcome", content = "server"))

      state.updateContent(
        siteId = "site",
        noteId = "note-$outcome",
        value = "draft",
        save = saveContent,
      )
      advanceTimeBy(300)
      runCurrent()
      assertEquals(1, saveCount)
      assertTrue(state.isDirty(siteId = "site", noteId = "note-$outcome"))

      advanceTimeBy(1_000)
      runCurrent()
      assertEquals(1, saveCount)

      val flushed =
        state.flush(
          siteId = "site",
          noteId = "note-$outcome",
          saveContent = saveContent,
          saveColor = { _, _ -> NoteSaveOutcome.Saved },
        )

      assertTrue(flushed)
      assertEquals(2, saveCount)
      assertFalse(state.isDirty(siteId = "site", noteId = "note-$outcome"))
    }
  }

  @Test
  fun `a later focus loss retries desired fields after a subscription gate`() = runTest {
    val state = createNoteEditState()
    var contentSaveCount = 0
    var colorSaveCount = 0
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      contentSaveCount += 1
      NoteSaveOutcome.Saved
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      colorSaveCount += 1
      NoteSaveOutcome.SubscriptionGated
    }
    advanceTimeBy(180)
    runCurrent()

    state.flushOnFocusLoss(
      siteId = "site",
      noteId = "note",
      saveContent = { _, _ ->
        contentSaveCount += 1
        NoteSaveOutcome.Saved
      },
      saveColor = { _, _ ->
        colorSaveCount += 1
        NoteSaveOutcome.Saved
      },
    )

    assertEquals(1, contentSaveCount)
    assertEquals(2, colorSaveCount)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
    assertFalse(state.hasPendingColor(siteId = "site", noteId = "note"))
  }

  @Test
  fun `focus loss does not duplicate a matching in flight save`() = runTest {
    val state = createNoteEditState()
    val finishSave = CompletableDeferred<Unit>()
    var saveCount = 0
    val saveContent: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
      saveCount += 1
      finishSave.await()
      NoteSaveOutcome.Saved
    }
    state.open(note = notesNote(id = "note", content = "server"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft", save = saveContent)
    advanceTimeBy(300)
    runCurrent()

    backgroundScope.launch {
      state.flushOnFocusLoss(
        siteId = "site",
        noteId = "note",
        saveContent = saveContent,
        saveColor = { _, _ -> NoteSaveOutcome.Saved },
      )
    }
    runCurrent()
    assertEquals(1, saveCount)

    finishSave.complete(Unit)
    runCurrent()

    assertEquals(1, saveCount)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `focus loss does not retry content that fails while waiting for color`() = runTest {
    val state = createNoteEditState()
    val contentSaveStarted = CompletableDeferred<Unit>()
    val finishContentSave = CompletableDeferred<Unit>()
    var contentSaveCount = 0
    var colorSaveCount = 0
    val saveColor: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
      colorSaveCount += 1
      NoteSaveOutcome.Saved
    }
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      contentSaveCount += 1
      contentSaveStarted.complete(Unit)
      finishContentSave.await()
      NoteSaveOutcome.Failed
    }
    advanceTimeBy(300)
    runCurrent()
    contentSaveStarted.await()

    state.updateColor(siteId = "site", noteId = "note", value = "red", save = saveColor)
    val firstBlur = backgroundScope.launch {
      state.flushOnFocusLoss(
        siteId = "site",
        noteId = "note",
        saveContent = { _, _ ->
          contentSaveCount += 1
          NoteSaveOutcome.Saved
        },
        saveColor = saveColor,
      )
    }
    runCurrent()

    finishContentSave.complete(Unit)
    runCurrent()

    assertTrue(firstBlur.isCompleted)
    assertEquals(1, contentSaveCount)
    assertEquals(1, colorSaveCount)
    assertTrue(state.isDirty(siteId = "site", noteId = "note"))

    state.flushOnFocusLoss(
      siteId = "site",
      noteId = "note",
      saveContent = { _, _ ->
        contentSaveCount += 1
        NoteSaveOutcome.Saved
      },
      saveColor = { _, _ -> NoteSaveOutcome.Saved },
    )

    assertEquals(2, contentSaveCount)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `focus loss cancels pending content debounce before waiting for color`() = runTest {
    val state = createNoteEditState()
    val colorSaveStarted = CompletableDeferred<Unit>()
    val finishColorSave = CompletableDeferred<Unit>()
    var automaticContentSaveCount = 0
    var focusLossContentSaveCount = 0
    var colorSaveCount = 0
    val saveColor: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
      colorSaveCount += 1
      colorSaveStarted.complete(Unit)
      finishColorSave.await()
      NoteSaveOutcome.Saved
    }
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateColor(siteId = "site", noteId = "note", value = "red", save = saveColor)
    advanceTimeBy(180)
    runCurrent()
    colorSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      automaticContentSaveCount += 1
      NoteSaveOutcome.Superseded
    }
    val blurJob = backgroundScope.launch {
      state.flushOnFocusLoss(
        siteId = "site",
        noteId = "note",
        saveContent = { _, _ ->
          focusLossContentSaveCount += 1
          NoteSaveOutcome.Saved
        },
        saveColor = saveColor,
      )
    }
    runCurrent()

    advanceTimeBy(300)
    runCurrent()
    finishColorSave.complete(Unit)
    runCurrent()

    assertTrue(blurJob.isCompleted)
    assertEquals(0, automaticContentSaveCount)
    assertEquals(1, focusLossContentSaveCount)
    assertEquals(1, colorSaveCount)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `subscription gated save does not automatically run a queued newer draft`() = runTest {
    val state = createNoteEditState()
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
  fun `concurrent triggers share content queued behind an active color save`() = runTest {
    val state = createNoteEditState()
    val colorSaveStarted = CompletableDeferred<Unit>()
    val finishColorSave = CompletableDeferred<Unit>()
    val contentSaveStarted = CompletableDeferred<Unit>()
    val finishContentSave = CompletableDeferred<Unit>()
    var contentSaveCount = 0
    var colorSaveCount = 0
    val saveColor: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
      colorSaveCount += 1
      colorSaveStarted.complete(Unit)
      finishColorSave.await()
      NoteSaveOutcome.Saved
    }
    val saveContent: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
      contentSaveCount += 1
      contentSaveStarted.complete(Unit)
      finishContentSave.await()
      NoteSaveOutcome.Saved
    }
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateColor(siteId = "site", noteId = "note", value = "red", save = saveColor)
    advanceTimeBy(180)
    runCurrent()
    colorSaveStarted.await()

    state.updateContent(siteId = "site", noteId = "note", value = "draft", save = saveContent)
    advanceTimeBy(300)
    runCurrent()
    assertEquals(0, contentSaveCount)

    val blurJob = backgroundScope.launch {
      state.flushOnFocusLoss(
        siteId = "site",
        noteId = "note",
        saveContent = saveContent,
        saveColor = saveColor,
      )
    }
    var flushResult: Boolean? = null
    val flushJob = backgroundScope.launch {
      flushResult =
        state.flush(
          siteId = "site",
          noteId = "note",
          saveContent = saveContent,
          saveColor = saveColor,
        )
    }
    runCurrent()

    finishColorSave.complete(Unit)
    runCurrent()
    contentSaveStarted.await()
    assertEquals(1, contentSaveCount)
    assertFalse(flushJob.isCompleted)

    finishContentSave.complete(Unit)
    runCurrent()

    assertTrue(blurJob.isCompleted)
    assertTrue(flushJob.isCompleted)
    assertEquals(true, flushResult)
    assertEquals(1, contentSaveCount)
    assertEquals(1, colorSaveCount)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `caller cancellation does not strand an owner scoped field attempt`() = runTest {
    val state = createNoteEditState()
    val saveStarted = CompletableDeferred<Unit>()
    val finishSave = CompletableDeferred<Unit>()
    var saveCount = 0
    val saveContent: suspend (String, String) -> NoteSaveOutcome = { _, _ ->
      saveCount += 1
      saveStarted.complete(Unit)
      finishSave.await()
      NoteSaveOutcome.Saved
    }
    state.open(note = notesNote(id = "note", content = "server"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft", save = saveContent)

    val firstFlush = backgroundScope.launch {
      state.flush(
        siteId = "site",
        noteId = "note",
        saveContent = saveContent,
        saveColor = { _, _ -> NoteSaveOutcome.Saved },
      )
    }
    runCurrent()
    saveStarted.await()

    firstFlush.cancel()
    runCurrent()
    assertTrue(firstFlush.isCancelled)

    var secondResult: Boolean? = null
    val secondFlush = backgroundScope.launch {
      secondResult =
        state.flush(
          siteId = "site",
          noteId = "note",
          saveContent = saveContent,
          saveColor = { _, _ -> NoteSaveOutcome.Saved },
        )
    }
    runCurrent()
    assertFalse(secondFlush.isCompleted)
    assertEquals(1, saveCount)

    finishSave.complete(Unit)
    runCurrent()

    assertTrue(secondFlush.isCompleted)
    assertEquals(true, secondResult)
    assertEquals(1, saveCount)
    assertFalse(state.isDirty(siteId = "site", noteId = "note"))
  }

  @Test
  fun `owner cancellation completes a waiting field attempt`() = runTest {
    val ownerJob = Job()
    val ownerScope = CoroutineScope(coroutineContext + ownerJob)
    val state = NoteEditState(scope = ownerScope)
    val saveStarted = CompletableDeferred<Unit>()
    val finishSave = CompletableDeferred<Unit>()
    state.open(note = notesNote(id = "note", content = "server"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      error("flush must replace the pending debounce callback")
    }

    val flushJob = backgroundScope.launch {
      state.flush(
        siteId = "site",
        noteId = "note",
        saveContent = { _, _ ->
          saveStarted.complete(Unit)
          finishSave.await()
          NoteSaveOutcome.Saved
        },
        saveColor = { _, _ -> NoteSaveOutcome.Saved },
      )
    }
    runCurrent()
    saveStarted.await()

    ownerJob.cancel()
    runCurrent()

    assertTrue(flushJob.isCompleted)
    assertTrue(flushJob.isCancelled)
  }

  @Test
  fun `flush rechecks color after content wait creates a newer color generation`() = runTest {
    val state = createNoteEditState()
    val contentStarted = CompletableDeferred<Unit>()
    val finishContent = CompletableDeferred<Unit>()
    val secondColorStarted = CompletableDeferred<Unit>()
    val finishSecondColor = CompletableDeferred<Unit>()
    val colorSaves = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(siteId = "site", noteId = "note", value = "draft") { _, _ ->
      error("flush must own the content save")
    }
    state.updateColor(siteId = "site", noteId = "note", value = "red") { _, _ ->
      error("flush must own the color save")
    }

    var flushResult: Boolean? = null
    val flushJob = backgroundScope.launch {
      flushResult =
        state.flush(
          siteId = "site",
          noteId = "note",
          saveContent = { _, _ ->
            contentStarted.complete(Unit)
            finishContent.await()
            NoteSaveOutcome.Saved
          },
          saveColor = { _, color ->
            colorSaves += color
            if (colorSaves.size == 2) {
              secondColorStarted.complete(Unit)
              finishSecondColor.await()
            }
            NoteSaveOutcome.Saved
          },
        )
    }
    runCurrent()
    contentStarted.await()
    assertEquals(listOf("red"), colorSaves)

    state.updateColor(siteId = "site", noteId = "note", value = "blue") { _, _ ->
      error("the same flush must own the newer color")
    }
    finishContent.complete(Unit)
    runCurrent()
    secondColorStarted.await()

    assertFalse(flushJob.isCompleted)
    assertEquals(listOf("red", "blue"), colorSaves)

    finishSecondColor.complete(Unit)
    runCurrent()

    assertTrue(flushJob.isCompleted)
    assertEquals(true, flushResult)
    assertFalse(state.hasPendingColor(siteId = "site", noteId = "note"))
  }

  @Test
  fun `cancelled active save ignores its late failure`() = runTest {
    val state = createNoteEditState()
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
  fun `cancelling an active save lets a newer save own its saving interval`() = runTest {
    val state = createNoteEditState()
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
    advanceTimeBy(300)
    runCurrent()

    assertTrue(colorSaveStarted.isCompleted)
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))

    advanceTimeBy(500)
    runCurrent()
    assertEquals(NoteSaveStatus.SAVING, state.saveStatus(siteId = "site", noteId = "note"))

    finishColorSave.complete(Unit)
    runCurrent()
    assertEquals(NoteSaveStatus.NONE, state.saveStatus(siteId = "site", noteId = "note"))
  }
}
