package co.typie.migration

object LegacyHiveBoxReader {
  fun readBox(bytes: ByteArray): Map<String, Any?> =
    readFrames(bytes = bytes, keyCrc = 0, decrypt = null)

  fun readEncryptedBox(
    bytes: ByteArray,
    keyCrc: Long,
    decrypt: (ByteArray) -> ByteArray,
  ): Map<String, Any?> = readFrames(bytes = bytes, keyCrc = keyCrc, decrypt = decrypt)

  private fun readFrames(
    bytes: ByteArray,
    keyCrc: Long,
    decrypt: ((ByteArray) -> ByteArray)?,
  ): Map<String, Any?> {
    val values = linkedMapOf<String, Any?>()
    var offset = 0

    while (offset < bytes.size) {
      val frameLength = readUInt32(bytes, offset).toInt()
      require(frameLength >= FRAME_MIN_LENGTH) { "Invalid Hive frame length: $frameLength" }

      val frameEnd = offset + frameLength
      require(frameEnd <= bytes.size) { "Hive frame exceeds box size." }

      val crcOffset = frameEnd - CRC_LENGTH
      val expectedCrc = readUInt32(bytes, crcOffset)
      val actualCrc = calculateLegacyCrc32(bytes, offset, frameLength - CRC_LENGTH, keyCrc)
      require(actualCrc == expectedCrc) { "Hive frame CRC mismatch." }

      var cursor = offset + FRAME_LENGTH_BYTES
      val keyType = bytes[cursor++].toInt() and BYTE_MASK
      require(keyType == FRAME_KEY_UTF8_STRING) { "Unsupported Hive frame key type: $keyType" }

      val keyLength = bytes[cursor++].toInt() and BYTE_MASK
      val keyEnd = cursor + keyLength
      require(keyEnd <= crcOffset) { "Hive frame key overruns payload boundary." }
      val key = bytes.decodeToString(cursor, keyEnd)
      cursor = keyEnd

      val rawValueBytes = bytes.copyOfRange(cursor, crcOffset)
      val value =
        when {
          rawValueBytes.isEmpty() -> null
          decrypt != null -> decodeValue(decrypt(rawValueBytes))
          else -> decodeValue(rawValueBytes)
        }

      values[key] = value
      offset = frameEnd
    }

    return values
  }

  private fun decodeValue(bytes: ByteArray): Any? {
    val cursor = ByteCursor(bytes)
    val value =
      when (val typeId = cursor.readByte()) {
        VALUE_TYPE_NULL -> null
        VALUE_TYPE_INT -> cursor.readDouble().toLong()
        VALUE_TYPE_DOUBLE -> cursor.readDouble()
        VALUE_TYPE_BOOL -> cursor.readByte() != 0
        VALUE_TYPE_STRING -> {
          val length = cursor.readUInt32().toInt()
          cursor.readString(length)
        }

        else -> error("Unsupported Hive value type: $typeId")
      }

    require(cursor.isAtEnd()) { "Hive value payload has trailing bytes." }
    return value
  }

  private class ByteCursor(private val bytes: ByteArray) {
    private var offset = 0

    fun readByte(): Int {
      require(offset < bytes.size) { "Not enough bytes available." }
      return bytes[offset++].toInt() and BYTE_MASK
    }

    fun readUInt32(): Long {
      val value = readUInt32(bytes, offset)
      offset += FRAME_LENGTH_BYTES
      return value
    }

    fun readDouble(): Double {
      require(offset + DOUBLE_LENGTH <= bytes.size) { "Not enough bytes available." }

      var bits = 0L
      for (index in 0 until DOUBLE_LENGTH) {
        bits = bits or ((bytes[offset + index].toLong() and BYTE_MASK.toLong()) shl (index * 8))
      }
      offset += DOUBLE_LENGTH
      return Double.fromBits(bits)
    }

    fun readString(length: Int): String {
      require(offset + length <= bytes.size) { "Not enough bytes available." }
      val value = bytes.decodeToString(offset, offset + length)
      offset += length
      return value
    }

    fun isAtEnd(): Boolean = offset == bytes.size
  }

  private const val BYTE_MASK = 0xFF
  private const val FRAME_LENGTH_BYTES = 4
  private const val CRC_LENGTH = 4
  private const val FRAME_MIN_LENGTH = 8
  private const val DOUBLE_LENGTH = 8

  private const val FRAME_KEY_UTF8_STRING = 1

  private const val VALUE_TYPE_NULL = 0
  private const val VALUE_TYPE_INT = 1
  private const val VALUE_TYPE_DOUBLE = 2
  private const val VALUE_TYPE_BOOL = 3
  private const val VALUE_TYPE_STRING = 4
}

internal fun calculateLegacyCrc32(
  bytes: ByteArray,
  offset: Int,
  length: Int,
  seed: Long = 0,
): Long {
  var crc = seed.toInt() xor -1
  val end = offset + length

  for (index in offset until end) {
    val tableIndex = (crc xor (bytes[index].toInt() and 0xFF)) and 0xFF
    crc = LEGACY_CRC32_TABLE[tableIndex] xor (crc ushr 8)
  }

  return (crc xor -1).toLong() and 0xFFFF_FFFFL
}

private fun readUInt32(bytes: ByteArray, offset: Int): Long {
  require(offset + 4 <= bytes.size) { "Not enough bytes available." }
  return (bytes[offset].toLong() and 0xFF) or
    ((bytes[offset + 1].toLong() and 0xFF) shl 8) or
    ((bytes[offset + 2].toLong() and 0xFF) shl 16) or
    ((bytes[offset + 3].toLong() and 0xFF) shl 24)
}

private val LEGACY_CRC32_TABLE =
  IntArray(256) { index ->
    var value = index
    repeat(8) {
      value =
        if ((value and 1) != 0) {
          0xEDB8_8320.toInt() xor (value ushr 1)
        } else {
          value ushr 1
        }
    }
    value
  }
