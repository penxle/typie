package co.typie.auth.sso

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

@Composable
actual fun activityContext(): Any? = LocalContext.current
