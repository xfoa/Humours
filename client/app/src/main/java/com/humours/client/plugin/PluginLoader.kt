package com.humours.client.plugin

import android.content.Context
import dalvik.system.DexClassLoader
import java.io.File

class PluginLoader(private val context: Context) {
    private val builtinRegistry: Map<String, () -> WidgetPlugin> = mutableMapOf()
    private val builtin by lazy { builtinRegistry as Map<String, () -> WidgetPlugin> }

    fun registerBuiltin(className: String, factory: () -> WidgetPlugin) {
        (builtinRegistry as MutableMap)[className] = factory
    }

    fun load(className: String, jarPath: String? = null): WidgetPlugin? {
        builtin[className]?.let { return it() }
        if (jarPath == null) return null
        return runCatching {
            val optimizedDir = context.codeCacheDir
            optimizedDir.mkdirs()
            val optimizedPath = File(optimizedDir, "plugin_dex").absolutePath
            val classLoader = DexClassLoader(
                jarPath,
                optimizedPath,
                null,
                context.classLoader,
            )
            val cls = classLoader.loadClass(className)
            val ctor = cls.getDeclaredConstructor()
            ctor.isAccessible = true
            ctor.newInstance() as WidgetPlugin
        }.getOrElse {
            android.util.Log.e("PluginLoader", "Failed to load plugin $className from $jarPath", it)
            null
        }
    }

    fun metadataFor(className: String): WidgetMetadata? {
        return runCatching {
            builtin[className]?.let { it().metadata }
        }.getOrNull()
    }

    fun availablePlugins(): List<String> = builtin.keys.toList()
}