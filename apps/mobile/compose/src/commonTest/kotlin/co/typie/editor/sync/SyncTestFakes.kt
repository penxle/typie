package co.typie.editor.sync

import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

internal fun enc(vararg ids: Int): ByteArray = ByteArray(ids.size) { ids[it].toByte() }

internal fun dec(payload: ByteArray): List<Int> = payload.map { it.toInt() and 0xFF }.sorted()

internal fun createTestDocumentEditingSession(
  editor: Editor,
  scope: CoroutineScope,
  documentId: String = "doc",
): DocumentEditingSession {
  val syncEditor = FakeSyncEditor()
  val engine =
    SyncEngine(
      editor = syncEditor,
      documentId = documentId,
      initialServerHeads = enc(),
      initialDurableHeads = enc(),
      store = FakeDeltaStore(),
      pushFn = TestSyncTransport::push,
      scope = scope,
      now = { 0L },
    )
  val pipeline =
    RemoteChangesetPipeline(
      editor = syncEditor,
      headsSink = engine,
      transport = TestSyncTransport,
      initialSeq = "",
      scope = scope,
      onNeedsReload = {},
    )
  return DocumentEditingSession(
    documentId = documentId,
    editor = editor,
    engine = engine,
    pipeline = pipeline,
    scope = scope,
  )
}

private object TestSyncTransport : SyncTransport {
  override suspend fun push(changesets: ByteArray): PushResult =
    PushResult(heads = enc(), durableHeads = enc())

  override suspend fun pull(sinceSeq: String?): PullResult =
    PullResult(
      changesets = emptyList(),
      seq = "",
      heads = enc(),
      durableHeads = enc(),
      needsReload = false,
    )

  override fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent> = emptyFlow()
}

internal class FakeSyncEditor(initial: List<Int> = emptyList()) : SyncEditor {
  val known = initial.toMutableSet()
  var withheld = 0
  val missingCalls = mutableListOf<List<Int>>()

  override suspend fun currentHeads(): ByteArray = if (known.isEmpty()) enc() else enc(known.max())

  override suspend fun changesetIds(): List<String> = known.sorted().map { it.toString() }

  override suspend fun missingChangesetsFor(confirmedHeads: ByteArray): MissingBytes {
    missingCalls.add(dec(confirmedHeads))
    val effective = dec(confirmedHeads).filter { it in known }.maxOrNull() ?: 0
    val missing = known.filter { it > effective }.sorted()
    val emitted = missing.take(maxOf(0, missing.size - withheld))
    return MissingBytes(bytes = enc(*emitted.toIntArray()), withheld = withheld)
  }

  override suspend fun partitionRemoteChangesets(payload: ByteArray): PartitionedBytes {
    val ready = dec(payload).filter { it !in known }
    return PartitionedBytes(ready = enc(*ready.toIntArray()), blocked = enc())
  }

  override suspend fun splitChangesets(payload: ByteArray): List<SplitChangeset> =
    dec(payload).map { SplitChangeset(id = it.toString(), bytes = enc(it)) }

  override suspend fun receiveRemoteChangeset(payload: ByteArray) {
    dec(payload).forEach { known.add(it) }
  }
}

internal class FakeDeltaStore : DeltaStore {
  val records = mutableListOf<DeltaRecord>()
  var onLoad: (suspend (String) -> List<DeltaRecord>)? = null
  var onPut: (suspend (DeltaRecord) -> Unit)? = null
  var onDeleteMany: (suspend (String, List<String>) -> Unit)? = null
  private val insertionOrder = mutableMapOf<String, Long>()
  private var insertionCounter = 0L

  private fun orderKey(record: DeltaRecord) = "${record.documentId}/${record.id}"

  override suspend fun load(documentId: String): List<DeltaRecord> =
    onLoad?.invoke(documentId)
      ?: records
        .filter { it.documentId == documentId }
        .sortedWith(compareBy({ it.createdAt }, { insertionOrder[orderKey(it)] ?: 0L }))

  override suspend fun put(record: DeltaRecord) {
    onPut?.let {
      it(record)
      return
    }
    defaultPut(record)
  }

  suspend fun defaultPut(record: DeltaRecord) {
    records.removeAll { it.documentId == record.documentId && it.id == record.id }
    insertionOrder.getOrPut(orderKey(record)) { insertionCounter++ }
    records.add(record)
  }

  override suspend fun deleteMany(documentId: String, ids: List<String>) {
    onDeleteMany?.invoke(documentId, ids)
    records.removeAll { it.documentId == documentId && it.id in ids }
  }

  override suspend fun listDocumentIds(): List<String> = records.map { it.documentId }.distinct()

  override suspend fun wipeAll() {
    records.clear()
  }
}
