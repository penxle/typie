package co.typie.screen.space.notes

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import co.typie.form.FormState
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.type.NoteStatus
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

@Composable
internal fun rememberNotesScreenState(
  contentDebounceMillis: Long = 300L,
  colorDebounceMillis: Long = 180L,
): NotesScreenState {
  val scope = rememberCoroutineScope()
  return remember(scope, contentDebounceMillis, colorDebounceMillis) {
    NotesScreenState(
      scope = scope,
      contentDebounceMillis = contentDebounceMillis,
      colorDebounceMillis = colorDebounceMillis,
    )
  }
}

@Stable
internal class NotesScreenState(
  private val scope: CoroutineScope,
  private val contentDebounceMillis: Long = 300L,
  private val colorDebounceMillis: Long = 180L,
) {
  var filterStatus by mutableStateOf(NoteStatus.OPEN)
    private set

  val expandedNoteId: String?
    get() = activeForm?.noteId

  private var activeForm: ActiveNoteFormState? by mutableStateOf(null)
  private val openSceneState = NotesSceneState(NoteStatus.OPEN)
  private val resolvedSceneState = NotesSceneState(NoteStatus.RESOLVED)

  fun updateFilterStatus(status: NoteStatus) {
    if (status == NoteStatus.UNKNOWN__ || filterStatus == status) {
      return
    }

    filterStatus = status
  }

  fun sceneState(status: NoteStatus): NotesSceneState =
    when (status) {
      NoteStatus.RESOLVED -> resolvedSceneState
      else -> openSceneState
    }

  fun syncScene(status: NoteStatus, notes: List<NoteCard_note>) {
    sceneState(status).sync(notes)

    val activeNoteId = activeForm?.noteId ?: return
    notes.firstOrNull { it.id == activeNoteId }?.let(::commitServerSnapshot)
  }

  fun open(note: NoteCard_note) {
    val currentForm = activeForm
    if (currentForm?.noteId == note.id) {
      currentForm.commitServerSnapshot(note)
      return
    }

    activeForm =
      ActiveNoteFormState(
        scope = scope,
        note = note,
        contentDebounceMillis = contentDebounceMillis,
        colorDebounceMillis = colorDebounceMillis,
      )
  }

  fun clearExpanded(noteId: String? = expandedNoteId) {
    if (noteId == null || activeForm?.noteId != noteId) {
      return
    }

    activeForm = null
  }

  fun overlay(note: NoteCard_note): NoteCard_note = activeForm?.overlay(note) ?: note

  fun commitServerSnapshot(note: NoteCard_note): NoteCard_note =
    activeForm?.commitServerSnapshot(note) ?: note

  fun updateContent(
    noteId: String,
    value: String,
    save: suspend (noteId: String, content: String) -> Boolean,
  ) {
    activeForm?.takeIf { it.noteId == noteId }?.updateContent(value = value, save = save)
  }

  fun updateColor(
    noteId: String,
    value: String,
    save: suspend (noteId: String, color: String) -> Boolean,
  ) {
    activeForm?.takeIf { it.noteId == noteId }?.updateColor(value = value, save = save)
  }

  suspend fun flush(
    noteId: String,
    saveContent: suspend (noteId: String, content: String) -> Boolean,
    saveColor: suspend (noteId: String, color: String) -> Boolean,
  ): Boolean {
    val currentForm = activeForm ?: return true
    if (currentForm.noteId != noteId) {
      return true
    }

    return currentForm.flush(saveContent = saveContent, saveColor = saveColor)
  }

  suspend fun collapse(
    saveContent: suspend (noteId: String, content: String) -> Boolean,
    saveColor: suspend (noteId: String, color: String) -> Boolean,
  ): Boolean {
    val currentForm = activeForm ?: return true
    if (!currentForm.flush(saveContent = saveContent, saveColor = saveColor)) {
      return false
    }

    activeForm = null
    return true
  }

  fun isDirty(noteId: String): Boolean =
    activeForm?.takeIf { it.noteId == noteId }?.isContentDirty == true

  fun isSaving(noteId: String): Boolean =
    activeForm?.takeIf { it.noteId == noteId }?.isContentSaving == true

  fun hasPendingColor(noteId: String): Boolean =
    activeForm?.takeIf { it.noteId == noteId }?.isColorDirty == true

  fun isSavingColor(noteId: String): Boolean =
    activeForm?.takeIf { it.noteId == noteId }?.isColorSaving == true

  fun cancelPendingSaves(noteId: String) {
    activeForm?.takeIf { it.noteId == noteId }?.cancelPendingSaves()
  }

  fun remove(noteId: String) {
    val currentForm = activeForm ?: return
    if (currentForm.noteId != noteId) {
      return
    }

    currentForm.cancelPendingSaves()
    activeForm = null
  }

  fun dispose(
    savePendingContent: (noteId: String, content: String) -> Unit,
    savePendingColor: (noteId: String, color: String) -> Unit,
  ) {
    activeForm?.dispose(
      savePendingContent = savePendingContent,
      savePendingColor = savePendingColor,
    )
  }
}

@Stable
internal class NotesSceneState(private val status: NoteStatus) {
  var hasSettled by mutableStateOf(false)
    private set

  private val enteringNotesById = mutableStateMapOf<String, NoteCard_note>()
  private val enteringAnimationIds = mutableStateMapOf<String, Boolean>()
  private val expectedEntryNotesById = mutableStateMapOf<String, NoteCard_note>()
  private val exitingNotesById = mutableStateMapOf<String, ExitingNoteSnapshot>()

  fun sync(serverNotes: List<NoteCard_note>) {
    hasSettled = true
    val serverIds = serverNotes.mapTo(mutableSetOf()) { it.id }

    enteringNotesById.keys
      .filter { it in serverIds }
      .forEach { noteId -> enteringNotesById.remove(noteId) }

    expectedEntryNotesById.keys.toList().forEach { noteId ->
      if (noteId in serverIds) {
        enteringAnimationIds[noteId] = true
        expectedEntryNotesById.remove(noteId)
      }
    }

    exitingNotesById.keys.toList().forEach { noteId ->
      val snapshot = exitingNotesById[noteId] ?: return@forEach
      if (!snapshot.isVisible && noteId !in serverIds) {
        exitingNotesById.remove(noteId)
      }
    }
  }

  fun merge(serverNotes: List<NoteCard_note>): List<NoteCard_note> {
    val mergedNotesById = linkedMapOf<String, NoteCard_note>()
    val exitingNoteIds = exitingNotesById.keys
    val serverIds = serverNotes.mapTo(mutableSetOf()) { it.id }

    enteringNotesById.values
      .asSequence()
      .filter { it.status == status && it.id !in serverIds && it.id !in exitingNoteIds }
      .forEach { mergedNotesById[it.id] = it }

    serverNotes
      .asSequence()
      .filter { it.id !in exitingNoteIds }
      .forEach { mergedNotesById[it.id] = it }

    exitingNotesById.values
      .asSequence()
      .filter { it.isVisible }
      .forEach { mergedNotesById[it.note.id] = it.note }

    return mergedNotesById.values.sortedBy { it.order }
  }

  fun markEntering(note: NoteCard_note) {
    expectedEntryNotesById.remove(note.id)
    exitingNotesById.remove(note.id)
    enteringNotesById[note.id] = note
    enteringAnimationIds[note.id] = true
  }

  fun expectEntry(note: NoteCard_note) {
    if (note.status != status) {
      return
    }

    exitingNotesById.remove(note.id)
    expectedEntryNotesById[note.id] = note
  }

  fun finishEntering(noteId: String) {
    enteringAnimationIds.remove(noteId)
  }

  fun markExiting(note: NoteCard_note) {
    expectedEntryNotesById.remove(note.id)
    enteringNotesById.remove(note.id)
    enteringAnimationIds.remove(note.id)
    exitingNotesById[note.id] = ExitingNoteSnapshot(note = note, isVisible = true)
  }

  fun finishExiting(noteId: String) {
    val snapshot = exitingNotesById[noteId] ?: return
    exitingNotesById[noteId] = snapshot.copy(isVisible = false)
  }

  fun remove(noteId: String) {
    expectedEntryNotesById.remove(noteId)
    enteringNotesById.remove(noteId)
    enteringAnimationIds.remove(noteId)
    exitingNotesById.remove(noteId)
  }

  fun isEntering(noteId: String): Boolean = noteId in enteringAnimationIds

  fun isExiting(noteId: String): Boolean = noteId in exitingNotesById

  fun isExitVisible(noteId: String): Boolean = exitingNotesById[noteId]?.isVisible == true
}

private data class ExitingNoteSnapshot(val note: NoteCard_note, val isVisible: Boolean)

@Stable
private class ActiveNoteFormState(
  scope: CoroutineScope,
  note: NoteCard_note,
  contentDebounceMillis: Long,
  colorDebounceMillis: Long,
) {
  val noteId: String = note.id

  var serverSnapshot by mutableStateOf(note)
    private set

  var isContentSaving by mutableStateOf(false)
    private set

  var isColorSaving by mutableStateOf(false)
    private set

  val isContentDirty: Boolean
    get() = form.content.isDirty

  val isColorDirty: Boolean
    get() = form.color.isDirty

  private val form = NoteEditorForm(scope = scope, note = note)

  private val contentSaveController =
    NotesDebouncedSaveController(scope = scope, debounceMillis = contentDebounceMillis)
  private val colorSaveController =
    NotesDebouncedSaveController(scope = scope, debounceMillis = colorDebounceMillis)

  fun overlay(note: NoteCard_note): NoteCard_note =
    if (note.id == noteId) {
      serverSnapshot.copy(content = form.content.value, color = form.color.value)
    } else {
      note
    }

  fun commitServerSnapshot(note: NoteCard_note): NoteCard_note {
    if (note.id != noteId) {
      return note
    }

    serverSnapshot = note
    form.syncFromSnapshot(note)
    return overlay(note)
  }

  fun updateContent(value: String, save: suspend (noteId: String, content: String) -> Boolean) {
    form.content.setValue(value)

    if (!form.content.isDirty) {
      contentSaveController.cancel(contentSaveKey())
      return
    }

    contentSaveController.submit(contentSaveKey()) { saveContentNow(save) }
  }

  fun updateColor(value: String, save: suspend (noteId: String, color: String) -> Boolean) {
    form.color.setValue(value)

    if (!form.color.isDirty) {
      colorSaveController.cancel(colorSaveKey())
      return
    }

    colorSaveController.submit(colorSaveKey()) { saveColorNow(save) }
  }

  suspend fun flush(
    saveContent: suspend (noteId: String, content: String) -> Boolean,
    saveColor: suspend (noteId: String, color: String) -> Boolean,
  ): Boolean {
    if (!colorSaveController.runNow(colorSaveKey()) { saveColorNow(saveColor) }) {
      return false
    }

    return contentSaveController.runNow(contentSaveKey()) { saveContentNow(saveContent) }
  }

  fun cancelPendingSaves() {
    contentSaveController.cancel(contentSaveKey())
    colorSaveController.cancel(colorSaveKey())
    isContentSaving = false
    isColorSaving = false
  }

  fun dispose(
    savePendingContent: (noteId: String, content: String) -> Unit,
    savePendingColor: (noteId: String, color: String) -> Unit,
  ) {
    contentSaveController.cancelAll()
    colorSaveController.cancelAll()

    if (form.content.isDirty) {
      savePendingContent(noteId, form.content.value)
    }

    if (form.color.isDirty) {
      savePendingColor(noteId, form.color.value)
    }
  }

  private suspend fun saveContentNow(
    save: suspend (noteId: String, content: String) -> Boolean
  ): Boolean {
    if (!form.content.isDirty) {
      return true
    }

    val currentContent = form.content.value
    isContentSaving = true
    val didSave =
      try {
        save(noteId, currentContent)
      } finally {
        isContentSaving = false
      }

    if (didSave) {
      form.content.syncFromSource(currentContent)
    }

    return didSave
  }

  private suspend fun saveColorNow(
    save: suspend (noteId: String, color: String) -> Boolean
  ): Boolean {
    if (!form.color.isDirty) {
      return true
    }

    val currentColor = form.color.value
    isColorSaving = true
    val didSave =
      try {
        save(noteId, currentColor)
      } finally {
        isColorSaving = false
      }

    if (didSave) {
      form.color.syncFromSource(currentColor)
    }

    return didSave
  }

  private fun contentSaveKey(): String = "content:$noteId"

  private fun colorSaveKey(): String = "color:$noteId"
}

private class NoteEditorForm(scope: CoroutineScope, note: NoteCard_note) : FormState(scope) {
  val content = field(note.content)
  val color = field(note.color) { focusable = false }

  fun syncFromSnapshot(note: NoteCard_note) {
    content.syncFromSource(note.content)
    color.syncFromSource(note.color)
  }
}

private class NotesDebouncedSaveController(
  private val scope: CoroutineScope,
  private val debounceMillis: Long,
) {
  private val debounceJobs = mutableMapOf<String, Job>()
  private val saveMutexes = mutableMapOf<String, Mutex>()

  fun submit(saveKey: String, action: suspend () -> Unit) {
    debounceJobs.remove(saveKey)?.cancel()

    var debounceJob: Job? = null
    debounceJob = scope.launch {
      delay(debounceMillis)
      runNow(saveKey) {
        action()
        true
      }
      if (debounceJobs[saveKey] === debounceJob) {
        debounceJobs.remove(saveKey)
      }
    }

    debounceJobs[saveKey] = debounceJob
  }

  suspend fun runNow(saveKey: String, action: suspend () -> Boolean): Boolean {
    val currentJob = debounceJobs.remove(saveKey)
    if (currentJob != coroutineContext[Job]) {
      currentJob?.cancel()
    }

    val mutex = saveMutexes.getOrPut(saveKey) { Mutex() }
    return mutex.withLock { action() }
  }

  fun cancel(saveKey: String) {
    debounceJobs.remove(saveKey)?.cancel()
  }

  fun cancelAll() {
    debounceJobs.values.forEach { it.cancel() }
    debounceJobs.clear()
    saveMutexes.clear()
  }
}
