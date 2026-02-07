allprojects {
  repositories {
    google()
    mavenCentral()
  }
}

val newBuildDir: Directory = rootProject.layout.buildDirectory.dir("../../build").get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
  val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
  project.layout.buildDirectory.value(newSubprojectBuildDir)
}

subprojects {
  project.evaluationDependsOn(":app")
}

subprojects {
  plugins.withId("com.android.library") {
    val androidComponents = extensions.getByType(
      com.android.build.api.variant.LibraryAndroidComponentsExtension::class.java
    )
    androidComponents.finalizeDsl { ext ->
      if (ext.compileSdk != null && ext.compileSdk!! < 36) {
        ext.compileSdk = 36
      }
    }
  }
}

tasks.register<Delete>("clean") {
  delete(rootProject.layout.buildDirectory)
}
