# kotlinx.serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.**
-keepclassmembers class **$$serializer { *; }
-keepclasseswithmembers class * {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclass class com.humours.client.**$$serializer { *; }
-keepclassmembers class com.humours.client.** {
    *** Companion;
}
-keepclasseswithmembers class com.humours.client.** {
    kotlinx.serialization.KSerializer serializer(...);
}