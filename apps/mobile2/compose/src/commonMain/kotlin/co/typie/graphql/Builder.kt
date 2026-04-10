package co.typie.graphql

import co.typie.graphql.builder.resolver.DefaultFakeResolver
import co.typie.graphql.builder.resolver.adaptToJson
import com.apollographql.apollo.api.CompiledField
import com.apollographql.apollo.api.DataBuilderScope
import com.apollographql.apollo.api.FakeResolver
import com.apollographql.apollo.api.FakeResolverContext
import kotlinx.serialization.json.JsonNull
import kotlin.math.absoluteValue
import kotlin.random.Random
import kotlin.time.Clock
import kotlin.time.Duration.Companion.days

object PlaceholderResolver : FakeResolver {
  private val delegate = DefaultFakeResolver()

  override fun resolveLeaf(context: FakeResolverContext): Any {
    return when (context.mergedField.type.rawType().name) {
      "BigInt" -> context.id.hashCode().absoluteValue.toLong() % 1000000L
      "Binary" -> "AA=="
      "DateTime" -> context.adaptToJson(Clock.System.now() - (context.id.hashCode().absoluteValue % 30).days)
      "JSON" -> JsonNull
      else -> delegate.resolveLeaf(context)
    }
  }

  override fun resolveListSize(context: FakeResolverContext): Int {
    return delegate.resolveListSize(context)
  }

  override fun resolveMaybeNull(context: FakeResolverContext): Boolean {
    return delegate.resolveMaybeNull(context)
  }

  override fun resolveTypename(context: FakeResolverContext): String {
    return delegate.resolveTypename(context)
  }

  override fun stableIdForObject(obj: Map<String, Any?>, mergedField: CompiledField): String {
    return Random.nextInt().toString()
  }
}

private const val FILLER = '\uAC00'

fun DataBuilderScope.text(length: IntRange, lines: Int = 1): String =
  (1..lines).joinToString("\n") { FILLER.toString().repeat(length.random()) }
