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

    isCoreLibraryDesugaringEnabled = true
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

      isMinifyEnabled = true;
      isShrinkResources = true;

      proguardFiles(
        getDefaultProguardFile("proguard-android-optimize.txt"),
        "proguard-rules.pro"
      )
    }
  }
}

flutter {
  source = "../.."
}

dependencies {
  implementation("androidx.window:window:1.3.0")
  implementation("androidx.window:window-java:1.3.0")
  coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.5")
}