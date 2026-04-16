package co.typie.graphql.adapter

import com.apollographql.apollo.api.Adapter
import com.apollographql.apollo.api.CustomScalarAdapters
import com.apollographql.apollo.api.json.JsonReader
import com.apollographql.apollo.api.json.JsonWriter
import kotlin.io.encoding.Base64

val BinaryAdapter =
  object : Adapter<ByteArray> {
    override fun fromJson(
      reader: JsonReader,
      customScalarAdapters: CustomScalarAdapters,
    ): ByteArray {
      return Base64.decode(reader.nextString()!!)
    }

    override fun toJson(
      writer: JsonWriter,
      customScalarAdapters: CustomScalarAdapters,
      value: ByteArray,
    ) {
      writer.value(Base64.encode(value))
    }
  }
