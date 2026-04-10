package co.typie.platform

import android.app.Activity
import androidx.activity.compose.LocalActivity
import androidx.compose.runtime.Composable

actual typealias ActivityContext = Activity

@Composable actual fun activityContext(): ActivityContext = LocalActivity.current!!
