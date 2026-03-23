package co.typie.storage

import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeWriteMode
import eu.anifantakis.lib.ksafe.invoke
import org.koin.core.annotation.Named
import org.koin.core.annotation.Single
import kotlin.properties.ReadWriteProperty
import kotlin.reflect.KProperty

@PublishedApi
internal fun <T> ReadWriteProperty<Any?, T>.withComposeState(): ReadWriteProperty<Any?, T> {
  val persisted = this
  return object : ReadWriteProperty<Any?, T> {
    private var state: MutableState<T>? = null

    override fun getValue(thisRef: Any?, property: KProperty<*>): T {
      val s = state ?: mutableStateOf(persisted.getValue(thisRef, property)).also { state = it }
      return s.value
    }

    override fun setValue(thisRef: Any?, property: KProperty<*>, value: T) {
      persisted.setValue(thisRef, property, value)
      val s = state ?: mutableStateOf(value).also { state = it }
      s.value = value
    }
  }
}

@Single
class Prefs(@Named("ksafe.prefs") val prefs: KSafe) {
  inline operator fun <reified T> invoke(
    key: String,
    defaultValue: T,
  ): ReadWriteProperty<Any?, T> =
    prefs.invoke(defaultValue, key, mode = KSafeWriteMode.Plain).withComposeState()
}

@Single
class Vault(@Named("ksafe.vault") val vault: KSafe) {
  inline operator fun <reified T> invoke(
    key: String,
    defaultValue: T,
  ): ReadWriteProperty<Any?, T> =
    vault.invoke(defaultValue, key).withComposeState()
}
