package co.typie.graphql.adapter

import com.apollographql.apollo.api.Adapter
import com.apollographql.apollo.api.AnyAdapter
import com.apollographql.apollo.api.CustomScalarAdapters
import com.apollographql.apollo.api.json.JsonReader
import com.apollographql.apollo.api.json.JsonWriter
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

val JsonElementAdapter =
  object : Adapter<JsonElement> {
    override fun fromJson(
      reader: JsonReader,
      customScalarAdapters: CustomScalarAdapters,
    ): JsonElement {
      return AnyAdapter.fromJson(reader, customScalarAdapters).toJsonElement()
    }

    override fun toJson(
      writer: JsonWriter,
      customScalarAdapters: CustomScalarAdapters,
      value: JsonElement,
    ) {
      AnyAdapter.toJson(writer, customScalarAdapters, value.toAny()!!)
    }
  }

private fun Any?.toJsonElement(): JsonElement =
  when (this) {
    null -> JsonNull
    is Map<*, *> -> JsonObject(map { (k, v) -> k as String to v.toJsonElement() }.toMap())
    is List<*> -> JsonArray(map { it.toJsonElement() })
    is Boolean -> JsonPrimitive(this)
    is Number -> JsonPrimitive(this)
    is String -> JsonPrimitive(this)
    else -> error("Unexpected JSON value: $this")
  }

private fun JsonElement.toAny(): Any? =
  when (this) {
    is JsonNull -> null
    is JsonObject -> mapValues { (_, v) -> v.toAny() }
    is JsonArray -> map { it.toAny() }
    is JsonPrimitive ->
      when {
        isString -> content
        else ->
          content.toBooleanStrictOrNull()
            ?: content.toLongOrNull()
            ?: content.toDoubleOrNull()
            ?: content
      }
  }
