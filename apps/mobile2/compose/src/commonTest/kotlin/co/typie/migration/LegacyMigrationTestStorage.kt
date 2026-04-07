package co.typie.migration

import co.typie.storage.Prefs
import co.typie.storage.Vault
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy
import kotlin.random.Random

internal fun createLegacyMigrationTestPrefs(): Prefs {
  return Prefs(
    KSafe(
      fileName = "legacy_migration_prefs_${Random.nextInt(1_000_000)}",
      memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT,
    ),
  )
}

internal fun createLegacyMigrationTestVault(): Vault {
  return Vault(
    KSafe(
      fileName = "legacy_migration_vault_${Random.nextInt(1_000_000)}",
      memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT,
    ),
  )
}
