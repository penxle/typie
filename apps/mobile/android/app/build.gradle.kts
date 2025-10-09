import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
  id("com.android.application")
  id("com.google.gms.google-services")
  id("kotlin-android")
  id("dev.flutter.flutter-gradle-plugin")
}

android {
  namespace = "co.typie"
  compileSdk = flutter.compileSdkVersion
  ndkVersion = flutter.ndkVersion

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21

    isCoreLibraryDesugaringEnabled = true
  }

  defaultConfig {
    applicationId = "co.typie"
    minSdk = 26
    targetSdk = flutter.targetSdkVersion
    versionCode = flutter.versionCode
    versionName = flutter.versionName

    multiDexEnabled = true
  }

  signingConfigs {
    getByName("debug") {
      storeFile = file("../keystore-debug.jks")
      storePassword = "password"
      keyAlias = "co.typie"
      keyPassword = "password"
    }

    create("release") {
      storeFile = file("../keystore-release.jks")
      storePassword = System.getenv("KEYSTORE_PASSWORD")
      keyAlias = "co.typie"
      keyPassword = System.getenv("KEYSTORE_PASSWORD")
    }
  }

  buildTypes {
    getByName("debug") {
      signingConfig = signingConfigs.getByName("debug")
    }

    getByName("release") {
      signingConfig = signingConfigs.getByName("release")

      isMinifyEnabled = true
      isShrinkResources = true

      proguardFiles(
        getDefaultProguardFile("proguard-android-optimize.txt"),
        "proguard-rules.pro"
      )
    }
  }
}

kotlin {
  compilerOptions {
    jvmTarget = JvmTarget.JVM_21
  }
}

flutter {
  source = "../.."
}

dependencies {
  implementation("androidx.core:core-splashscreen:1.0.1")
  implementation("androidx.window:window:1.5.0")
  implementation("androidx.window:window-java:1.5.0")
  implementation("com.squareup.moshi:moshi:1.15.2")
  coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.5")
}
