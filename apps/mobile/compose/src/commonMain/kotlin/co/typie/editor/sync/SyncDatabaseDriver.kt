package co.typie.editor.sync

import app.cash.sqldelight.db.SqlDriver

expect fun createSyncDatabaseDriver(): SqlDriver
