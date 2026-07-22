@file:OptIn(kotlin.concurrent.atomics.ExperimentalAtomicApi::class)

package co.typie.platform

import kotlin.concurrent.atomics.AtomicReference
import kotlin.coroutines.CoroutineContext
import kotlin.coroutines.EmptyCoroutineContext
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext

enum class IncomingContentMode {
  Rich,
  PlainTextOnly,
}

data class IncomingContentItem(val kind: Kind, val file: PickedFile) {
  enum class Kind {
    Image,
    File,
  }
}

internal suspend fun materializeIncomingContentCandidates(
  html: String?,
  text: String?,
  loadItems: suspend () -> LoadedIncomingContentItems,
): IncomingContentCandidates? {
  val nonemptyHtml = html?.takeIf(String::isNotEmpty)
  if (nonemptyHtml != null) {
    return IncomingContentCandidates(html = nonemptyHtml, text = text)
  }

  val loaded = loadItems()
  if (text.isNullOrEmpty() && loaded.items.isEmpty() && loaded.unreadableItemCount == 0) return null
  return IncomingContentCandidates(
    text = text,
    items = loaded.items,
    unreadableItemCount = loaded.unreadableItemCount,
  )
}

internal data class LoadedIncomingContentItems(
  val items: List<IncomingContentItem>,
  val unreadableItemCount: Int = 0,
) {
  init {
    require(unreadableItemCount >= 0)
  }
}

internal suspend fun loadOwnedIncomingContentItems(
  loaders: List<suspend () -> IncomingContentItem?>,
  loaderContext: CoroutineContext = EmptyCoroutineContext,
): LoadedIncomingContentItems {
  val owners = List(loaders.size) { OwnedIncomingContentItem() }
  return try {
    val unreadableItemCount =
      coroutineScope {
          loaders.mapIndexed { index, load ->
            async(loaderContext) {
              try {
                load()?.let(owners[index]::store)
                false
              } catch (error: kotlinx.coroutines.CancellationException) {
                throw error
              } catch (_: Throwable) {
                true
              }
            }
          }
        }
        .awaitAll()
        .count { it }
    currentCoroutineContext().ensureActive()
    LoadedIncomingContentItems(
      items = owners.mapNotNull(OwnedIncomingContentItem::take),
      unreadableItemCount = unreadableItemCount,
    )
  } finally {
    owners.forEach(OwnedIncomingContentItem::close)
  }
}

internal suspend fun loadOwnedIncomingContentCandidates(
  loaderContext: CoroutineContext,
  load: suspend () -> IncomingContentCandidates?,
): IncomingContentCandidates? {
  val owner = AtomicReference<IncomingContentCandidates?>(null)
  return try {
    withContext(loaderContext) {
      load()?.let { candidates ->
        check(owner.compareAndSet(expectedValue = null, newValue = candidates))
      }
    }
    currentCoroutineContext().ensureActive()
    owner.exchange(null)
  } finally {
    owner.exchange(null)?.close()
  }
}

private class OwnedIncomingContentItem {
  private val item = AtomicReference<IncomingContentItem?>(null)

  fun store(value: IncomingContentItem) {
    check(item.compareAndSet(expectedValue = null, newValue = value))
  }

  fun take(): IncomingContentItem? = item.exchange(null)

  fun close() {
    item.exchange(null)?.file?.close()
  }
}

class IncomingContentCandidates(
  val html: String? = null,
  val text: String? = null,
  val items: List<IncomingContentItem> = emptyList(),
  val unreadableItemCount: Int = 0,
) {
  private var selected = false

  init {
    require(unreadableItemCount >= 0)
  }

  fun select(mode: IncomingContentMode): SelectedIncomingContent {
    check(!selected) { "Incoming content has already been selected" }
    selected = true

    if (mode == IncomingContentMode.PlainTextOnly) {
      releaseItems()
      return text?.takeIf(String::isNotEmpty)?.let {
        SelectedIncomingContent.RichText(html = null, text = it)
      } ?: SelectedIncomingContent.None
    }

    val nonemptyHtml = html?.takeIf(String::isNotEmpty)
    if (nonemptyHtml != null) {
      releaseItems()
      return SelectedIncomingContent.RichText(html = nonemptyHtml, text = text.orEmpty())
    }

    if (items.isNotEmpty() || unreadableItemCount > 0) {
      return SelectedIncomingContent.Attachments(items, unreadableItemCount)
    }

    return text?.takeIf(String::isNotEmpty)?.let {
      SelectedIncomingContent.RichText(html = null, text = it)
    } ?: SelectedIncomingContent.None
  }

  fun close() {
    if (selected) return
    selected = true
    releaseItems()
  }

  private fun releaseItems() {
    items.forEach { it.file.close() }
  }
}

sealed interface SelectedIncomingContent {
  data class RichText(val html: String?, val text: String) : SelectedIncomingContent

  class Attachments(val items: List<IncomingContentItem>, val unreadableItemCount: Int = 0) :
    SelectedIncomingContent {
    fun close() {
      items.forEach { it.file.close() }
    }
  }

  data object None : SelectedIncomingContent
}
