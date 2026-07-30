package co.typie.domain.note

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.form.FormState
import co.typie.graphql.fragment.NoteCard_note
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

private const val CONTENT_SAVE_DEBOUNCE_MILLIS = 300L
private const val COLOR_SAVE_DEBOUNCE_MILLIS = 180L
private const val SAVING_INDICATOR_DELAY_MILLIS = 500L

@Stable
internal class NoteEditState(private val scope: CoroutineScope) {
  val expandedNoteId: String?
    get() = activeForm?.noteId

  val expandedNoteSiteId: String?
    get() = activeForm?.siteId

  private val mutableSaveFailures = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
  val saveFailures = mutableSaveFailures.asSharedFlow()

  private var activeForm: ActiveNoteFormState? by mutableStateOf(null)

  fun open(note: NoteCard_note, autoFocusContent: Boolean = false) {
    val currentForm = activeForm
    if (currentForm?.noteId == note.id && currentForm.siteId == note.site.id) {
      currentForm.commitServerSnapshot(note)
      return
    }

    activeForm =
      ActiveNoteFormState(
        scope = scope,
        note = note,
        autoFocusContent = autoFocusContent,
        onSaveFailed = { mutableSaveFailures.tryEmit(Unit) },
      )
  }

  fun clearExpanded(siteId: String, noteId: String? = expandedNoteId) {
    if (noteId == null || activeFormFor(siteId = siteId, noteId = noteId) == null) {
      return
    }

    activeForm = null
  }

  fun overlay(note: NoteCard_note): NoteCard_note = activeForm?.overlay(note) ?: note

  fun commitServerSnapshot(note: NoteCard_note): NoteCard_note =
    activeForm?.commitServerSnapshot(note) ?: note

  fun isExpanded(siteId: String, noteId: String): Boolean =
    activeFormFor(siteId = siteId, noteId = noteId) != null

  fun updateContent(
    siteId: String,
    noteId: String,
    value: String,
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  ) {
    activeFormFor(siteId = siteId, noteId = noteId)?.updateContent(value = value, save = save)
  }

  fun updateColor(
    siteId: String,
    noteId: String,
    value: String,
    save: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    activeFormFor(siteId = siteId, noteId = noteId)?.updateColor(value = value, save = save)
  }

  suspend fun flush(
    siteId: String,
    noteId: String,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    val currentForm = activeFormFor(siteId = siteId, noteId = noteId) ?: return true

    return currentForm.flush(saveContent = saveContent, saveColor = saveColor)
  }

  suspend fun collapse(
    siteId: String,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    val currentForm = activeForm ?: return true
    if (currentForm.siteId != siteId) {
      return true
    }
    if (!currentForm.flush(saveContent = saveContent, saveColor = saveColor)) {
      return false
    }

    activeForm = null
    return true
  }

  suspend fun flushPendingEdits(
    savePendingContent:
      suspend (siteId: String, noteId: String, content: String) -> NoteSaveOutcome,
    savePendingColor: suspend (siteId: String, noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    val currentForm = activeForm ?: return true
    return currentForm.flush(
      saveContent = { noteId, content -> savePendingContent(currentForm.siteId, noteId, content) },
      saveColor = { noteId, color -> savePendingColor(currentForm.siteId, noteId, color) },
    )
  }

  fun isDirty(siteId: String, noteId: String): Boolean =
    activeFormFor(siteId = siteId, noteId = noteId)?.isContentDirty == true

  fun isSaving(siteId: String, noteId: String): Boolean =
    activeFormFor(siteId = siteId, noteId = noteId)?.isContentSaving == true

  fun hasPendingColor(siteId: String, noteId: String): Boolean =
    activeFormFor(siteId = siteId, noteId = noteId)?.isColorDirty == true

  fun isSavingColor(siteId: String, noteId: String): Boolean =
    activeFormFor(siteId = siteId, noteId = noteId)?.isColorSaving == true

  fun saveStatus(siteId: String, noteId: String): NoteSaveStatus =
    activeFormFor(siteId = siteId, noteId = noteId)?.saveStatus ?: NoteSaveStatus.NONE

  fun shouldAutoFocusContent(siteId: String, noteId: String): Boolean =
    activeFormFor(siteId = siteId, noteId = noteId)?.autoFocusContent == true

  fun cancelPendingSaves(siteId: String, noteId: String) {
    activeFormFor(siteId = siteId, noteId = noteId)?.cancelPendingSaves()
  }

  fun remove(siteId: String, noteId: String) {
    val currentForm = activeFormFor(siteId = siteId, noteId = noteId) ?: return
    currentForm.cancelPendingSaves()
    activeForm = null
  }

  fun dispose(
    savePendingContent:
      suspend (siteId: String, noteId: String, content: String) -> NoteSaveOutcome,
    savePendingColor: suspend (siteId: String, noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    val currentForm = activeForm ?: return
    currentForm.dispose(
      savePendingContent = savePendingContent,
      savePendingColor = savePendingColor,
    )
  }

  private fun activeFormFor(siteId: String, noteId: String): ActiveNoteFormState? =
    activeForm?.takeIf {
      it.siteId == siteId && it.noteId == noteId
    }
}

internal enum class NoteSaveStatus {
  NONE,
  SAVING,
  FAILED,
}

internal enum class NoteSaveOutcome {
  Saved,
  Failed,
  SubscriptionGated,
  Superseded,
}

@Stable
private class ActiveNoteFormState(
  private val scope: CoroutineScope,
  note: NoteCard_note,
  val autoFocusContent: Boolean,
  private val onSaveFailed: () -> Unit,
) {
  val noteId: String = note.id
  val siteId: String = note.site.id

  var serverSnapshot by mutableStateOf(note)
    private set

  var isContentSaving by mutableStateOf(false)
    private set

  var isColorSaving by mutableStateOf(false)
    private set

  private var showSaving by mutableStateOf(false)
  private var contentSaveFailed by mutableStateOf(false)
  private var colorSaveFailed by mutableStateOf(false)
  private var contentRevision = 0
  private var colorRevision = 0
  private var blockedContentRevision: Int? = null
  private var blockedColorRevision: Int? = null
  private var inFlightContentRevision: Int? = null
  private var inFlightColorRevision: Int? = null
  private var activeSaveCount = 0
  private var savingGeneration = 0L
  private var savingIntervalActive = false
  private var savingIndicatorJob: Job? = null

  val saveStatus: NoteSaveStatus
    get() =
      when {
        contentSaveFailed || colorSaveFailed -> NoteSaveStatus.FAILED
        showSaving -> NoteSaveStatus.SAVING
        else -> NoteSaveStatus.NONE
      }

  val isContentDirty: Boolean
    get() = form.content.isDirty

  val isColorDirty: Boolean
    get() = form.color.isDirty

  private val form = NoteEditorForm(scope = scope, note = note)

  private val contentSaveController =
    NotesDebouncedSaveController(scope = scope, debounceMillis = CONTENT_SAVE_DEBOUNCE_MILLIS)
  private val colorSaveController =
    NotesDebouncedSaveController(scope = scope, debounceMillis = COLOR_SAVE_DEBOUNCE_MILLIS)
  private val saveMutex = Mutex()

  fun overlay(note: NoteCard_note): NoteCard_note =
    if (note.id == noteId && note.site.id == siteId) {
      serverSnapshot.copy(content = form.content.value, color = form.color.value)
    } else {
      note
    }

  fun commitServerSnapshot(note: NoteCard_note): NoteCard_note {
    if (note.id != noteId || note.site.id != siteId) {
      return note
    }

    val desiredContent = form.content.value
    val desiredColor = form.color.value
    val preserveContent = contentSaveController.hasWork()
    val preserveColor = colorSaveController.hasWork()

    serverSnapshot = note
    form.syncFromSnapshot(note)
    if (preserveContent) form.content.setValue(desiredContent)
    if (preserveColor) form.color.setValue(desiredColor)
    if (!form.content.isDirty) {
      blockedContentRevision = null
      updateContentSaveFailed(false)
      if (contentSaveController.hasWork() && !contentSaveController.hasExecutingSave()) {
        contentSaveController.cancel()
        isContentSaving = false
        endSavingIntervalIfIdle()
      }
    }
    if (!form.color.isDirty) {
      blockedColorRevision = null
      updateColorSaveFailed(false)
      if (colorSaveController.hasWork() && !colorSaveController.hasExecutingSave()) {
        colorSaveController.cancel()
        isColorSaving = false
        endSavingIntervalIfIdle()
      }
    }
    return overlay(note)
  }

  fun updateContent(
    value: String,
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  ) {
    if (form.content.value != value) {
      contentRevision += 1
      blockedContentRevision = null
    }
    form.content.setValue(value)

    val mustCompensateForInFlightSave = inFlightContentRevision != null
    if (!form.content.isDirty && !mustCompensateForInFlightSave) {
      contentSaveController.cancel()
      isContentSaving = false
      endSavingIntervalIfIdle()
      blockedContentRevision = null
      updateContentSaveFailed(false)
      return
    }
    if (blockedContentRevision == contentRevision) return

    updateContentSaveFailed(false)
    contentSaveController.submit { generation ->
      saveContentNow(save = save, generation = generation, force = mustCompensateForInFlightSave)
    }
  }

  fun updateColor(value: String, save: suspend (noteId: String, color: String) -> NoteSaveOutcome) {
    if (form.color.value != value) {
      colorRevision += 1
      blockedColorRevision = null
    }
    form.color.setValue(value)

    val mustCompensateForInFlightSave = inFlightColorRevision != null
    if (!form.color.isDirty && !mustCompensateForInFlightSave) {
      colorSaveController.cancel()
      isColorSaving = false
      endSavingIntervalIfIdle()
      blockedColorRevision = null
      updateColorSaveFailed(false)
      return
    }
    if (blockedColorRevision == colorRevision) return

    updateColorSaveFailed(false)
    colorSaveController.submit { generation ->
      saveColorNow(save = save, generation = generation, force = mustCompensateForInFlightSave)
    }
  }

  suspend fun flush(
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    val mustCompensateContent = inFlightContentRevision != null
    val mustCompensateColor = inFlightColorRevision != null
    if (
      !colorSaveController.runNow { generation ->
        saveColorNow(save = saveColor, generation = generation, force = mustCompensateColor)
      }
    ) {
      return false
    }

    return contentSaveController.runNow { generation ->
      saveContentNow(save = saveContent, generation = generation, force = mustCompensateContent)
    }
  }

  fun cancelPendingSaves() {
    contentSaveController.cancel()
    colorSaveController.cancel()
    savingGeneration += 1
    savingIndicatorJob?.cancel()
    savingIndicatorJob = null
    activeSaveCount = 0
    savingIntervalActive = false
    isContentSaving = false
    isColorSaving = false
    showSaving = false
    updateContentSaveFailed(false)
    updateColorSaveFailed(false)
  }

  fun dispose(
    savePendingContent:
      suspend (siteId: String, noteId: String, content: String) -> NoteSaveOutcome,
    savePendingColor: suspend (siteId: String, noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    val mustCompensateContent = inFlightContentRevision != null
    val shouldFlushContent =
      blockedContentRevision != contentRevision &&
        inFlightContentRevision != contentRevision &&
        (form.content.isDirty || inFlightContentRevision != null)
    if (shouldFlushContent) {
      contentSaveController.launchNow { generation ->
        saveContentNow(
          save = { pendingNoteId, content -> savePendingContent(siteId, pendingNoteId, content) },
          generation = generation,
          force = mustCompensateContent,
        )
      }
    } else {
      contentSaveController.cancelScheduled()
    }

    val mustCompensateColor = inFlightColorRevision != null
    val shouldFlushColor =
      blockedColorRevision != colorRevision &&
        inFlightColorRevision != colorRevision &&
        (form.color.isDirty || inFlightColorRevision != null)
    if (shouldFlushColor) {
      colorSaveController.launchNow { generation ->
        saveColorNow(
          save = { pendingNoteId, color -> savePendingColor(siteId, pendingNoteId, color) },
          generation = generation,
          force = mustCompensateColor,
        )
      }
    } else {
      colorSaveController.cancelScheduled()
    }
  }

  private suspend fun saveContentNow(
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    generation: Long,
    force: Boolean,
  ): Boolean {
    if (!form.content.isDirty && !force) {
      isContentSaving = false
      endSavingIntervalIfIdle()
      return true
    }
    if (blockedContentRevision == contentRevision) {
      isContentSaving = false
      endSavingIntervalIfIdle()
      return false
    }

    val currentContent = form.content.value
    val revision = contentRevision
    updateContentSaveFailed(false)
    isContentSaving = true
    val savingGeneration = beginSaving()
    val outcome: NoteSaveOutcome?
    try {
      outcome = saveMutex.withLock {
        if (!contentSaveController.isCurrent(generation)) {
          return@withLock null
        }
        inFlightContentRevision = revision
        val result = save(noteId, currentContent)
        val isCurrent = contentSaveController.isCurrent(generation)
        if (result == NoteSaveOutcome.SubscriptionGated) {
          contentSaveController.cancel()
          colorSaveController.cancel()
          blockDirtyRevisions()
        } else if (isCurrent && result == NoteSaveOutcome.Superseded) {
          contentSaveController.cancel()
          colorSaveController.cancel()
        }
        result.takeIf { isCurrent }
      }
    } finally {
      inFlightContentRevision = null
      val hasQueuedSave = contentSaveController.hasPendingAfter(generation)
      isContentSaving = hasQueuedSave
      finishSaving(generation = savingGeneration, keepInterval = hasQueuedSave)
    }
    if (outcome == null) {
      return false
    }

    when (outcome) {
      NoteSaveOutcome.Saved -> {
        blockedContentRevision = null
        form.content.syncFromSource(currentContent)
      }
      NoteSaveOutcome.Failed -> {
        if (revision == contentRevision) {
          blockedContentRevision = revision
          updateContentSaveFailed(true)
        }
      }
      NoteSaveOutcome.SubscriptionGated,
      NoteSaveOutcome.Superseded -> Unit
    }

    return outcome == NoteSaveOutcome.Saved || outcome == NoteSaveOutcome.Superseded
  }

  private suspend fun saveColorNow(
    save: suspend (noteId: String, color: String) -> NoteSaveOutcome,
    generation: Long,
    force: Boolean,
  ): Boolean {
    if (!form.color.isDirty && !force) {
      isColorSaving = false
      endSavingIntervalIfIdle()
      return true
    }
    if (blockedColorRevision == colorRevision) {
      isColorSaving = false
      endSavingIntervalIfIdle()
      return false
    }

    val currentColor = form.color.value
    val revision = colorRevision
    updateColorSaveFailed(false)
    isColorSaving = true
    val savingGeneration = beginSaving()
    val outcome: NoteSaveOutcome?
    try {
      outcome = saveMutex.withLock {
        if (!colorSaveController.isCurrent(generation)) {
          return@withLock null
        }
        inFlightColorRevision = revision
        val result = save(noteId, currentColor)
        val isCurrent = colorSaveController.isCurrent(generation)
        if (result == NoteSaveOutcome.SubscriptionGated) {
          contentSaveController.cancel()
          colorSaveController.cancel()
          blockDirtyRevisions()
        } else if (isCurrent && result == NoteSaveOutcome.Superseded) {
          contentSaveController.cancel()
          colorSaveController.cancel()
        }
        result.takeIf { isCurrent }
      }
    } finally {
      inFlightColorRevision = null
      val hasQueuedSave = colorSaveController.hasPendingAfter(generation)
      isColorSaving = hasQueuedSave
      finishSaving(generation = savingGeneration, keepInterval = hasQueuedSave)
    }
    if (outcome == null) {
      return false
    }

    when (outcome) {
      NoteSaveOutcome.Saved -> {
        blockedColorRevision = null
        form.color.syncFromSource(currentColor)
      }
      NoteSaveOutcome.Failed -> {
        if (revision == colorRevision) {
          blockedColorRevision = revision
          updateColorSaveFailed(true)
        }
      }
      NoteSaveOutcome.SubscriptionGated,
      NoteSaveOutcome.Superseded -> Unit
    }

    return outcome == NoteSaveOutcome.Saved || outcome == NoteSaveOutcome.Superseded
  }

  private fun beginSaving(): Long {
    activeSaveCount += 1
    if (savingIntervalActive) return savingGeneration

    savingIntervalActive = true
    savingIndicatorJob?.cancel()
    savingIndicatorJob = scope.launch {
      delay(SAVING_INDICATOR_DELAY_MILLIS)
      if (activeSaveCount > 0) {
        showSaving = true
      }
    }
    return savingGeneration
  }

  private fun finishSaving(generation: Long, keepInterval: Boolean) {
    if (generation != savingGeneration) return

    activeSaveCount = (activeSaveCount - 1).coerceAtLeast(0)
    if (activeSaveCount != 0 || keepInterval) return

    endSavingIntervalIfIdle()
  }

  private fun endSavingIntervalIfIdle() {
    if (activeSaveCount != 0) return

    savingIntervalActive = false
    savingIndicatorJob?.cancel()
    savingIndicatorJob = null
    showSaving = false
  }

  private fun updateContentSaveFailed(value: Boolean) {
    val wasFailed = contentSaveFailed || colorSaveFailed
    contentSaveFailed = value
    if (!wasFailed && (contentSaveFailed || colorSaveFailed)) {
      onSaveFailed()
    }
  }

  private fun updateColorSaveFailed(value: Boolean) {
    val wasFailed = contentSaveFailed || colorSaveFailed
    colorSaveFailed = value
    if (!wasFailed && (contentSaveFailed || colorSaveFailed)) {
      onSaveFailed()
    }
  }

  private fun blockDirtyRevisions() {
    if (form.content.isDirty) blockedContentRevision = contentRevision
    if (form.color.isDirty) blockedColorRevision = colorRevision
  }
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
  private var debounceJob: Job? = null
  private val saveMutex = Mutex()
  private val executingGenerations = mutableSetOf<Long>()
  private var generation = 0L

  fun submit(action: suspend (generation: Long) -> Unit) {
    debounceJob?.cancel()
    val generation = advanceGeneration()

    var nextDebounceJob: Job? = null
    nextDebounceJob = scope.launch {
      delay(debounceMillis)
      if (debounceJob === nextDebounceJob) {
        debounceJob = null
      }
      runNow(generation) {
        action(generation)
        true
      }
    }

    debounceJob = nextDebounceJob
  }

  suspend fun runNow(action: suspend (generation: Long) -> Boolean): Boolean {
    val generation = advanceGeneration()
    val currentJob = debounceJob
    debounceJob = null
    if (currentJob != coroutineContext[Job]) {
      currentJob?.cancel()
    }

    return runNow(generation) { action(generation) }
  }

  private suspend fun runNow(generation: Long, action: suspend () -> Boolean): Boolean {
    executingGenerations.add(generation)
    return try {
      saveMutex.withLock {
        if (this.generation != generation) {
          false
        } else {
          action()
        }
      }
    } finally {
      executingGenerations.remove(generation)
    }
  }

  fun hasPendingAfter(generation: Long): Boolean =
    debounceJob != null || executingGenerations.any { it != generation && it == this.generation }

  fun hasWork(): Boolean = debounceJob != null || executingGenerations.isNotEmpty()

  fun hasExecutingSave(): Boolean = executingGenerations.isNotEmpty()

  fun isCurrent(generation: Long): Boolean = this.generation == generation

  fun cancel() {
    advanceGeneration()
    debounceJob?.cancel()
    debounceJob = null
  }

  fun cancelScheduled() {
    debounceJob?.cancel()
    debounceJob = null
  }

  fun launchNow(action: suspend (generation: Long) -> Boolean) {
    val generation = advanceGeneration()
    cancelScheduled()
    scope.launch(start = CoroutineStart.UNDISPATCHED) { runNow(generation) { action(generation) } }
  }

  private fun advanceGeneration(): Long = ++generation
}
