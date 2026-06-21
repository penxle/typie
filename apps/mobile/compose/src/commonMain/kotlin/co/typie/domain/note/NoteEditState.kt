package co.typie.domain.note

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.form.FormState
import co.typie.graphql.fragment.NoteCard_note
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

@Stable
internal class NoteEditState(
  private val scope: CoroutineScope,
  private val contentDebounceMillis: Long = 300L,
  private val colorDebounceMillis: Long = 180L,
) {
  val expandedNoteId: String?
    get() = activeForm?.noteId

  private var activeForm: ActiveNoteFormState? by mutableStateOf(null)

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
    val currentForm = activeForm ?: return
    currentForm.dispose(
      savePendingContent = savePendingContent,
      savePendingColor = savePendingColor,
    )
  }
}

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
