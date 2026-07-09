package co.typie.editor.sync

data class MissingBytes(val bytes: ByteArray, val withheld: Int)

data class PartitionedBytes(val ready: ByteArray, val blocked: ByteArray)

data class SplitChangeset(val id: String, val bytes: ByteArray)

interface SyncEditor {
  suspend fun currentHeads(): ByteArray

  suspend fun changesetIds(): List<String>

  suspend fun missingChangesetsFor(confirmedHeads: ByteArray): MissingBytes

  suspend fun partitionRemoteChangesets(payload: ByteArray): PartitionedBytes

  suspend fun splitChangesets(payload: ByteArray): List<SplitChangeset>

  suspend fun receiveRemoteChangeset(payload: ByteArray)
}

internal fun List<Int>.toChangesetBytes(): ByteArray = ByteArray(size) { this[it].toByte() }

internal fun List<ByteArray>.concatChangesets(): ByteArray {
  val total = sumOf { it.size }
  val out = ByteArray(total)
  var offset = 0
  for (part in this) {
    part.copyInto(out, offset)
    offset += part.size
  }
  return out
}

internal fun encodeLengthPrefixedBlobs(blobs: List<ByteArray>): ByteArray {
  var total = 4
  for (blob in blobs) total += 4 + blob.size
  val out = ByteArray(total)

  fun putU32(offset: Int, value: Int) {
    out[offset] = (value and 0xFF).toByte()
    out[offset + 1] = ((value ushr 8) and 0xFF).toByte()
    out[offset + 2] = ((value ushr 16) and 0xFF).toByte()
    out[offset + 3] = ((value ushr 24) and 0xFF).toByte()
  }

  putU32(0, blobs.size)
  var offset = 4
  for (blob in blobs) {
    putU32(offset, blob.size)
    offset += 4
    blob.copyInto(out, offset)
    offset += blob.size
  }
  return out
}

internal inline fun <T> catchingNonCancellation(block: () -> T): Result<T> =
  try {
    Result.success(block())
  } catch (e: kotlinx.coroutines.CancellationException) {
    throw e
  } catch (e: Throwable) {
    Result.failure(e)
  }

internal fun co.typie.editor.Editor.asSyncEditor(): SyncEditor {
  val editor = this
  return object : SyncEditor {
    override suspend fun currentHeads(): ByteArray = editor.currentHeads()

    override suspend fun changesetIds(): List<String> = editor.changesetIds()

    override suspend fun missingChangesetsFor(confirmedHeads: ByteArray): MissingBytes =
      editor.missingChangesetsFor(confirmedHeads)

    override suspend fun partitionRemoteChangesets(payload: ByteArray): PartitionedBytes =
      editor.partitionRemoteChangesets(payload)

    override suspend fun splitChangesets(payload: ByteArray): List<SplitChangeset> =
      editor.splitChangesets(payload)

    override suspend fun receiveRemoteChangeset(payload: ByteArray) =
      editor.receiveRemoteChangeset(payload)
  }
}
