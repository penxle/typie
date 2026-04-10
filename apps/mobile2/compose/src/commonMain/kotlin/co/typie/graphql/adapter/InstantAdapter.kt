package co.typie.graphql.adapter

import com.apollographql.apollo.api.Adapter
import com.apollographql.apollo.api.CustomScalarAdapters
import com.apollographql.apollo.api.json.JsonReader
import com.apollographql.apollo.api.json.JsonWriter
import kotlin.time.Instant

val InstantAdapter =
  object : Adapter<Instant> {
    override fun fromJson(reader: JsonReader, customScalarAdapters: CustomScalarAdapters): Instant {
      return Instant.parse(reader.nextString()!!)
    }

    override fun toJson(
      writer: JsonWriter,
      customScalarAdapters: CustomScalarAdapters,
      value: Instant,
    ) {
      writer.value(value.toString())
    }
  }
