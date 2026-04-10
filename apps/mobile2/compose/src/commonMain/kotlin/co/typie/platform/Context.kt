package co.typie.platform

import androidx.compose.runtime.Composable

expect class ActivityContext

@Composable
expect fun activityContext(): ActivityContext
