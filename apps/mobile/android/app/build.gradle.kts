plugins {
  id("com.android.application")
  id("com.google.gms.google-services")
  id("kotlin-android")
  id("dev.flutter.flutter-gradle-plugin")
}

android {
  namespace = "co.typie"
  compileSdk = 36
  ndkVersion = "27.0.12077973"

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
  }

  kotlinOptions {
    jvmTarget = JavaVersion.VERSION_21.toString()
  }

  defaultConfig {
    applicationId = "co.typie"
    minSdk = 31
    targetSdk = 36
    versionCode = flutter.versionCode
    versionName = flutter.versionName
  }

  signingConfigs {
    create("release") {
      storeFile = file("../keystore.jks")
      storePassword = System.getenv("KEYSTORE_PASSWORD")
      keyAlias = "co.typie"
      keyPassword = System.getenv("KEYSTORE_PASSWORD")
    }
  }

  buildTypes {
    release {
      signingConfig = signingConfigs.getByName("release")
    }
  }
}

flutter {
  source = "../.."
}
