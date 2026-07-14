# JNA resolves native symbols, struct fields, and callbacks via reflection.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-dontwarn java.awt.*

# uniffi-generated bindings: Library interface method names must match native symbol names.
-keep class uniffi.** { *; }

# Kakao SDK: https://developers.kakao.com/docs/ko/android/getting-started
-keep class com.kakao.sdk.**.model.* { <fields>; }
-keep interface com.kakao.sdk.**.*Api
-dontwarn org.bouncycastle.jsse.**
-dontwarn org.conscrypt.*
-dontwarn org.openjsse.**

# Retrofit with R8 full mode (bundled with the Kakao SDK guide above).
-if interface * { @retrofit2.http.* <methods>; }
-keep,allowobfuscation interface <1>
-if interface * { @retrofit2.http.* public *** *(...); }
-keep,allowoptimization,allowshrinking,allowobfuscation class <3>
-keep,allowobfuscation,allowshrinking class kotlin.coroutines.Continuation
-keep,allowobfuscation,allowshrinking class retrofit2.Response

# Retain line numbers so mapping.txt can deobfuscate crash reports.
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
