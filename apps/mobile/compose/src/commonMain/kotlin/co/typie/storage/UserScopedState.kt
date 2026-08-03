package co.typie.storage

import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import co.touchlab.kermit.Logger
import co.typie.platform.PlatformModule
import eu.anifantakis.lib.ksafe.KSafeWriteMode

internal class UserScopedStorage<T>(
  val load: (String) -> T,
  val save: (String, T) -> Unit,
  val exists: (String) -> Boolean,
  val delete: (String) -> Unit,
)

internal class UserScopedState<T>(
  private val baseKey: String,
  private val defaultValue: T,
  private val storage: UserScopedStorage<T>,
) : MutableState<T> {
  private val delegate = mutableStateOf(defaultValue)
  private var storageKey: String? = null

  override var value: T
    get() = delegate.value
    set(newValue) {
      val old = delegate.value
      delegate.value = newValue
      if (old != newValue) storageKey?.let { storage.save(it, newValue) }
    }

  fun rebind(userId: String?) {
    if (userId == null) {
      storageKey = null
      delegate.value = defaultValue
      return
    }
    val userKey = "$baseKey@$userId"
    migrate(userKey)
    storageKey = userKey
    delegate.value =
      runCatching { storage.load(userKey) }
        .getOrElse {
          Logger.w(it) { "UserScopedState: load failed for $baseKey" }
          defaultValue
        }
  }

  private fun migrate(userKey: String) {
    if (!storage.exists(baseKey)) return
    if (!storage.exists(userKey)) {
      runCatching { storage.save(userKey, storage.load(baseKey)) }
        .onFailure { Logger.w(it) { "UserScopedState: migration failed for $baseKey" } }
    }
    storage.delete(baseKey)
  }

  override fun component1(): T = value

  override fun component2(): (T) -> Unit = { value = it }
}

internal class UserScopedRegistry {
  private val states = mutableListOf<UserScopedState<*>>()

  fun register(state: UserScopedState<*>) {
    states.add(state)
  }

  fun switchUser(userId: String?) {
    states.forEach { it.rebind(userId) }
  }
}

internal inline fun <reified T> userPrefs(key: String, defaultValue: T): UserScopedState<T> {
  val ksafe = PlatformModule.ksafePrefs
  return UserScopedState(
      baseKey = key,
      defaultValue = defaultValue,
      storage =
        UserScopedStorage(
          load = { k -> ksafe.getDirect(k, defaultValue) },
          save = { k, v -> ksafe.putDirect(k, v, mode = KSafeWriteMode.Plain) },
          exists = { k -> ksafe.getKeyInfo(k) != null },
          delete = { k -> ksafe.deleteDirect(k) },
        ),
    )
    .also { userScopedRegistry.register(it) }
}
