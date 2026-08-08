package co.typie.domain.note

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.form.FieldState
import co.typie.form.FormState
import co.typie.graphql.fragment.NoteCard_note
import kotlinx.coroutines.CompletableDeferred
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

  fun open(note: NoteCard_note) {
    open(note = note, autoFocusContent = false)
  }

  fun openNew(note: NoteCard_note) {
    open(note = note, autoFocusContent = true)
  }

  private fun open(note: NoteCard_note, autoFocusContent: Boolean) {
    val currentForm = activeForm
    if (currentForm?.noteId == note.id && currentForm.siteId == note.site.id) {
      currentForm.commitServerSnapshot(note)
      return
    }

    currentForm?.cancelPendingSaves()
    activeForm =
      ActiveNoteFormState(
        scope = scope,
        note = note,
        autoFocusContent = autoFocusContent,
        onSaveFailed = { mutableSaveFailures.tryEmit(Unit) },
      )
  }

  fun clearExpanded(siteId: String, noteId: String? = expandedNoteId) {
    val currentForm = noteId?.let { activeFormFor(siteId = siteId, noteId = it) } ?: return
    currentForm.cancelPendingSaves()
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

  suspend fun flushOnFocusLoss(
    siteId: String,
    noteId: String,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    activeFormFor(siteId = siteId, noteId = noteId)
      ?.flushOnFocusLoss(saveContent = saveContent, saveColor = saveColor)
  }

  suspend fun collapse(
    siteId: String,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    val currentForm = activeForm ?: return true
    if (currentForm.siteId != siteId) return true
    if (!currentForm.flush(saveContent = saveContent, saveColor = saveColor)) return false

    currentForm.cancelPendingSaves()
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

  fun consumeAutoFocusContent(siteId: String, noteId: String) {
    activeFormFor(siteId = siteId, noteId = noteId)?.consumeAutoFocusContent()
  }

  fun cancelPendingSaves(siteId: String, noteId: String) {
    activeFormFor(siteId = siteId, noteId = noteId)?.cancelPendingSaves()
  }

  fun remove(siteId: String, noteId: String) {
    val currentForm = activeFormFor(siteId = siteId, noteId = noteId) ?: return
    currentForm.cancelPendingSaves()
    activeForm = null
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
  autoFocusContent: Boolean,
  private val onSaveFailed: () -> Unit,
) {
  val noteId: String = note.id
  val siteId: String = note.site.id

  var autoFocusContent by mutableStateOf(autoFocusContent)
    private set

  var serverSnapshot by mutableStateOf(note)
    private set

  var isContentSaving by mutableStateOf(false)
    private set

  var isColorSaving by mutableStateOf(false)
    private set

  private val form = NoteEditorForm(scope = scope, note = note)
  private val contentEdit = FieldEditState()
  private val colorEdit = FieldEditState()
  private val contentSaveController =
    NotesDebouncedSaveController(scope = scope, debounceMillis = CONTENT_SAVE_DEBOUNCE_MILLIS)
  private val colorSaveController =
    NotesDebouncedSaveController(scope = scope, debounceMillis = COLOR_SAVE_DEBOUNCE_MILLIS)
  private val attemptAdmissionMutex = Mutex()
  private val saveMutex = Mutex()

  private var showSaving by mutableStateOf(false)
  private var activeSaveCount = 0
  private var savingGeneration = 0L
  private var savingIntervalActive = false
  private var savingIndicatorJob: Job? = null

  val saveStatus: NoteSaveStatus
    get() =
      when {
        hasSaveFailure -> NoteSaveStatus.FAILED
        showSaving -> NoteSaveStatus.SAVING
        else -> NoteSaveStatus.NONE
      }

  fun consumeAutoFocusContent() {
    autoFocusContent = false
  }

  val isContentDirty: Boolean
    get() = contentEdit.desired != null

  val isColorDirty: Boolean
    get() = colorEdit.desired != null

  private val hasSaveFailure: Boolean
    get() = contentEdit.hasCurrentFailure || colorEdit.hasCurrentFailure

  fun overlay(note: NoteCard_note): NoteCard_note =
    if (note.id == noteId && note.site.id == siteId) {
      serverSnapshot.copy(content = form.content.value, color = form.color.value)
    } else {
      note
    }

  fun commitServerSnapshot(note: NoteCard_note): NoteCard_note {
    if (note.id != noteId || note.site.id != siteId) return note

    serverSnapshot = note
    syncFieldFromSource(field = form.content, edit = contentEdit, sourceValue = note.content)
    syncFieldFromSource(field = form.color, edit = colorEdit, sourceValue = note.color)
    return overlay(note)
  }

  fun updateContent(
    value: String,
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  ) {
    if (form.content.value == value) return

    val desired = contentEdit.recordEdit(value)
    form.content.setValue(value)
    contentSaveController.submit { attemptContent(desired = desired, save = save) }
  }

  fun updateColor(value: String, save: suspend (noteId: String, color: String) -> NoteSaveOutcome) {
    if (form.color.value == value) return

    val desired = colorEdit.recordEdit(value)
    form.color.setValue(value)
    colorSaveController.submit { attemptColor(desired = desired, save = save) }
  }

  suspend fun flush(
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    contentSaveController.cancel()
    colorSaveController.cancel()
    val attemptedContent = mutableSetOf<Long>()
    val attemptedColor = mutableSetOf<Long>()

    while (true) {
      if (!flushColorGenerations(attempted = attemptedColor, save = saveColor)) return false
      if (!flushContentGenerations(attempted = attemptedContent, save = saveContent)) return false
      if (contentEdit.desired == null && colorEdit.desired == null) return true
    }
  }

  suspend fun flushOnFocusLoss(
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    val contentTarget = contentEdit.desired
    val colorTarget = colorEdit.desired
    val contentAttemptAtEntry =
      contentEdit.attempt?.takeIf { it.generation == contentTarget?.generation }
    val colorAttemptAtEntry = colorEdit.attempt?.takeIf { it.generation == colorTarget?.generation }

    colorSaveController.cancel()
    contentSaveController.cancel()
    if (colorTarget != null) {
      if (colorAttemptAtEntry != null) {
        colorAttemptAtEntry.result.await()
      } else {
        attemptColor(desired = colorTarget, save = saveColor)
      }
    }

    if (contentTarget != null) {
      if (contentAttemptAtEntry != null) {
        contentAttemptAtEntry.result.await()
      } else {
        attemptContent(desired = contentTarget, save = saveContent)
      }
    }
  }

  fun cancelPendingSaves() {
    contentSaveController.cancel()
    colorSaveController.cancel()

    val contentAttempt = contentEdit.attempt
    val colorAttempt = colorEdit.attempt
    contentEdit.attempt = null
    colorEdit.attempt = null
    contentAttempt?.job?.cancel()
    colorAttempt?.job?.cancel()

    contentEdit.failedGeneration = null
    colorEdit.failedGeneration = null

    savingGeneration += 1
    savingIndicatorJob?.cancel()
    savingIndicatorJob = null
    activeSaveCount = 0
    savingIntervalActive = false
    isContentSaving = false
    isColorSaving = false
    showSaving = false
  }

  private suspend fun flushContentGenerations(
    attempted: MutableSet<Long>,
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  ): Boolean {
    while (true) {
      val desired = contentEdit.desired ?: return true
      if (!attempted.add(desired.generation)) return false
      attemptContent(desired = desired, save = save)
      val remaining = contentEdit.desired ?: return true
      if (remaining.generation == desired.generation) return false
    }
  }

  private suspend fun flushColorGenerations(
    attempted: MutableSet<Long>,
    save: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    while (true) {
      val desired = colorEdit.desired ?: return true
      if (!attempted.add(desired.generation)) return false
      attemptColor(desired = desired, save = save)
      val remaining = colorEdit.desired ?: return true
      if (remaining.generation == desired.generation) return false
    }
  }

  private suspend fun attemptContent(
    desired: DesiredValue,
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  ) {
    attemptField(
      edit = contentEdit,
      controller = contentSaveController,
      desired = desired,
      setSaving = { isContentSaving = it },
      save = { value -> save(noteId, value) },
      applyOutcome = { outcome -> applyContentOutcome(outcome = outcome, saved = desired) },
    )
  }

  private suspend fun attemptColor(
    desired: DesiredValue,
    save: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    attemptField(
      edit = colorEdit,
      controller = colorSaveController,
      desired = desired,
      setSaving = { isColorSaving = it },
      save = { value -> save(noteId, value) },
      applyOutcome = { outcome -> applyColorOutcome(outcome = outcome, saved = desired) },
    )
  }

  private suspend fun attemptField(
    edit: FieldEditState,
    controller: NotesDebouncedSaveController,
    desired: DesiredValue,
    setSaving: (Boolean) -> Unit,
    save: suspend (value: String) -> NoteSaveOutcome,
    applyOutcome: (NoteSaveOutcome) -> Unit,
  ) {
    while (true) {
      var created = false
      val attempt =
        attemptAdmissionMutex.withLock {
          if (edit.desired?.generation != desired.generation) return@withLock null

          edit.attempt
            ?: FieldSaveAttempt(generation = desired.generation, value = desired.value).also {
              edit.attempt = it
              created = true
              setSaving(true)
            }
        } ?: return

      if (created) {
        startFieldAttempt(
          edit = edit,
          controller = controller,
          attempt = attempt,
          setSaving = setSaving,
          save = save,
          applyOutcome = applyOutcome,
        )
      }

      val outcome = attempt.result.await()
      if (
        attempt.generation == desired.generation || outcome == NoteSaveOutcome.SubscriptionGated
      ) {
        return
      }
    }
  }

  private fun startFieldAttempt(
    edit: FieldEditState,
    controller: NotesDebouncedSaveController,
    attempt: FieldSaveAttempt,
    setSaving: (Boolean) -> Unit,
    save: suspend (value: String) -> NoteSaveOutcome,
    applyOutcome: (NoteSaveOutcome) -> Unit,
  ) {
    val job =
      scope.launch(start = CoroutineStart.LAZY) {
        try {
          val outcome = saveMutex.withLock {
            if (edit.desired?.generation != attempt.generation) return@withLock null

            clearFailureForAttempt(edit = edit, generation = attempt.generation)
            val intervalGeneration = beginSaving()
            try {
              save(attempt.value)
            } finally {
              finishSaving(
                generation = intervalGeneration,
                keepInterval = hasSaveWork(excluding = attempt),
              )
            }
          }

          if (outcome != null) applyOutcome(outcome)
          attempt.result.complete(outcome)
        } catch (error: Throwable) {
          if (!attempt.result.isCompleted) attempt.result.completeExceptionally(error)
          throw error
        } finally {
          finishFieldAttempt(
            edit = edit,
            controller = controller,
            attempt = attempt,
            setSaving = setSaving,
          )
        }
      }
    attempt.job = job
    job.invokeOnCompletion { error ->
      if (error != null && !attempt.result.isCompleted) {
        attempt.result.completeExceptionally(error)
      }
      finishFieldAttempt(
        edit = edit,
        controller = controller,
        attempt = attempt,
        setSaving = setSaving,
      )
    }
    job.start()
  }

  private fun finishFieldAttempt(
    edit: FieldEditState,
    controller: NotesDebouncedSaveController,
    attempt: FieldSaveAttempt,
    setSaving: (Boolean) -> Unit,
  ) {
    if (edit.attempt === attempt) edit.attempt = null
    setSaving(edit.attempt != null || controller.hasWork())
    settleSavingInterval()
  }

  private fun applyContentOutcome(outcome: NoteSaveOutcome, saved: DesiredValue) {
    when (outcome) {
      NoteSaveOutcome.Saved -> applySaved(field = form.content, edit = contentEdit, saved = saved)
      NoteSaveOutcome.Failed -> applyFailed(edit = contentEdit, generation = saved.generation)
      NoteSaveOutcome.SubscriptionGated -> cancelDebouncedSaves()
      NoteSaveOutcome.Superseded -> Unit
    }
  }

  private fun applyColorOutcome(outcome: NoteSaveOutcome, saved: DesiredValue) {
    when (outcome) {
      NoteSaveOutcome.Saved -> applySaved(field = form.color, edit = colorEdit, saved = saved)
      NoteSaveOutcome.Failed -> applyFailed(edit = colorEdit, generation = saved.generation)
      NoteSaveOutcome.SubscriptionGated -> cancelDebouncedSaves()
      NoteSaveOutcome.Superseded -> Unit
    }
  }

  private fun applySaved(field: FieldState<String>, edit: FieldEditState, saved: DesiredValue) {
    if (edit.desired?.generation == saved.generation) {
      edit.desired = null
      edit.failedGeneration = null
      field.syncFromSource(saved.value, preserveDirty = false)
      return
    }

    field.syncFromSource(saved.value, preserveDirty = false)
    edit.desired?.let { field.setValue(it.value) }
  }

  private fun applyFailed(edit: FieldEditState, generation: Long) {
    if (edit.desired?.generation != generation) return

    val wasFailed = hasSaveFailure
    edit.failedGeneration = generation
    if (!wasFailed && hasSaveFailure) onSaveFailed()
  }

  private fun clearFailureForAttempt(edit: FieldEditState, generation: Long) {
    if (edit.failedGeneration == generation) edit.failedGeneration = null
  }

  private fun cancelDebouncedSaves() {
    contentSaveController.cancel()
    colorSaveController.cancel()
    isContentSaving = contentEdit.attempt != null
    isColorSaving = colorEdit.attempt != null
    settleSavingInterval()
  }

  private fun syncFieldFromSource(
    field: FieldState<String>,
    edit: FieldEditState,
    sourceValue: String,
  ) {
    field.syncFromSource(sourceValue, preserveDirty = false)
    edit.desired?.let { field.setValue(it.value) }
  }

  private fun beginSaving(): Long {
    activeSaveCount += 1
    if (savingIntervalActive) return savingGeneration

    savingIntervalActive = true
    savingIndicatorJob?.cancel()
    savingIndicatorJob = scope.launch {
      delay(SAVING_INDICATOR_DELAY_MILLIS)
      if (activeSaveCount > 0 || hasSaveWork()) showSaving = true
    }
    return savingGeneration
  }

  private fun finishSaving(generation: Long, keepInterval: Boolean) {
    if (generation != savingGeneration) return

    activeSaveCount = (activeSaveCount - 1).coerceAtLeast(0)
    if (activeSaveCount != 0 || keepInterval) return
    endSavingInterval()
  }

  private fun settleSavingInterval() {
    if (activeSaveCount == 0 && !hasSaveWork()) endSavingInterval()
  }

  private fun endSavingInterval() {
    savingIntervalActive = false
    savingIndicatorJob?.cancel()
    savingIndicatorJob = null
    showSaving = false
  }

  private fun hasSaveWork(excluding: FieldSaveAttempt? = null): Boolean =
    contentSaveController.hasWork() ||
      colorSaveController.hasWork() ||
      (contentEdit.attempt != null && contentEdit.attempt !== excluding) ||
      (colorEdit.attempt != null && colorEdit.attempt !== excluding)
}

private data class DesiredValue(val generation: Long, val value: String)

private class FieldEditState {
  var desired: DesiredValue? by mutableStateOf(null)
  var failedGeneration: Long? by mutableStateOf(null)
  var attempt: FieldSaveAttempt? = null
  private var nextGeneration = 0L

  val hasCurrentFailure: Boolean
    get() = failedGeneration != null && failedGeneration == desired?.generation

  fun recordEdit(value: String): DesiredValue {
    val next = DesiredValue(generation = ++nextGeneration, value = value)
    desired = next
    failedGeneration = null
    return next
  }
}

private class FieldSaveAttempt(val generation: Long, val value: String) {
  val result = CompletableDeferred<NoteSaveOutcome?>()
  var job: Job? = null
}

private class NoteEditorForm(scope: CoroutineScope, note: NoteCard_note) : FormState(scope) {
  val content = field(note.content)
  val color = field(note.color) { focusable = false }
}

private class NotesDebouncedSaveController(
  private val scope: CoroutineScope,
  private val debounceMillis: Long,
) {
  private var job: Job? = null

  fun submit(action: suspend () -> Unit) {
    job?.cancel()
    var nextJob: Job? = null
    nextJob = scope.launch {
      delay(debounceMillis)
      if (job === nextJob) job = null
      action()
    }
    job = nextJob
  }

  fun cancel() {
    job?.cancel()
    job = null
  }

  fun hasWork(): Boolean = job != null
}
