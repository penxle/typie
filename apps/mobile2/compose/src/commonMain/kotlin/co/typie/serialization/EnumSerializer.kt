package co.typie.serialization

import kotlinx.serialization.KSerializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.buildSerialDescriptor
import kotlinx.serialization.descriptors.SerialKind
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlin.enums.EnumEntries

@OptIn(kotlinx.serialization.InternalSerializationApi::class)
open class EnumSerializer<T : Enum<T>>(
  private val entries: EnumEntries<T>,
  private val convert: (String) -> String,
) : KSerializer<T> {
  override val descriptor: SerialDescriptor = buildSerialDescriptor(
    entries.first()::class.qualifiedName ?: "Enum",
    SerialKind.ENUM,
  )

  override fun serialize(encoder: Encoder, value: T) =
    encoder.encodeString(convert(value.name))

  override fun deserialize(decoder: Decoder): T {
    val raw = decoder.decodeString()
    return entries.first { convert(it.name) == raw }
  }
}
