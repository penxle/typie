package co.typie.editor.input

import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.time.TimeSource
import kotlinx.collections.immutable.PersistentList
import kotlinx.collections.immutable.persistentListOf

@OptIn(ExperimentalAtomicApi::class)
internal class EditorInputRecorder {
  private val origin = TimeSource.Monotonic.markNow()
  private val seqCounter = AtomicLong(0L)
  private val entries = AtomicReference<PersistentList<RecordedInputEntry>>(persistentListOf())

  fun record(build: (seq: Long, t: Long) -> RecordedInputEntry) {
    val entry = build(seqCounter.addAndFetch(1L), origin.elapsedNow().inWholeMilliseconds)
    while (true) {
      val current = entries.load()
      val appended = current.add(entry)
      val next = if (appended.size > Capacity) appended.removeAt(0) else appended
      if (entries.compareAndSet(current, next)) return
    }
  }

  fun snapshot(): List<RecordedInputEntry> = entries.load()

  companion object {
    const val Capacity = 500
  }
}
