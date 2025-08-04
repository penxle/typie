package co.typie

import android.app.Application
import co.ab180.airbridge.flutter.AirbridgeFlutter

class MainApplication : Application() {
  override fun onCreate() {
    super.onCreate()
    AirbridgeFlutter.initializeSDK(this, "typie", "cee38499c2ba42cc834503cd819573ac")
  }
}
