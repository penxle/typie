package co.typie.editor.sync

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.native.NativeSqliteDriver
import co.typie.sync.db.SyncDatabase

actual fun createSyncDatabaseDriver(): SqlDriver =
  NativeSqliteDriver(SyncDatabase.Schema, "typie-sync.db")
