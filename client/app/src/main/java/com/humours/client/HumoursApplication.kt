package com.humours.client

import android.app.Application
import com.humours.client.builtin.CubeTetrahedronPlugin
import com.humours.client.data.local.SettingsStore
import com.humours.client.data.network.ConnectionManager
import com.humours.client.data.network.unsafeOkHttpClient
import com.humours.client.data.repository.MetricRepository
import com.humours.client.plugin.PluginInstanceManager
import com.humours.client.plugin.PluginLoader
import com.humours.client.plugin.gl.GLThread
import com.humours.client.plugin.surface.SurfaceManager
import okhttp3.OkHttpClient

class HumoursApplication : Application() {
    lateinit var httpClient: OkHttpClient
    lateinit var repository: MetricRepository
    lateinit var settingsStore: SettingsStore
    lateinit var connectionManager: ConnectionManager
    lateinit var pluginLoader: PluginLoader
    lateinit var glThread: GLThread
    lateinit var surfaceManager: SurfaceManager
    lateinit var pluginInstanceManager: PluginInstanceManager

    val builtInPluginClassName = "com.humours.client.builtin.CubeTetrahedronPlugin"

    override fun onCreate() {
        super.onCreate()
        instance = this
        httpClient = unsafeOkHttpClient()
        repository = MetricRepository()
        settingsStore = SettingsStore(this)
        pluginLoader = PluginLoader(this).apply {
            registerBuiltin(builtInPluginClassName) { CubeTetrahedronPlugin() }
        }
        glThread = GLThread().also { it.startAndPrepare() }
        surfaceManager = SurfaceManager(this)
        val requested = { CubeTetrahedronPlugin.CUBE_METRICS.toSet() }
        connectionManager = ConnectionManager(httpClient, repository, requested)
        pluginInstanceManager = PluginInstanceManager(glThread, surfaceManager, repository)
    }

    companion object {
        @Volatile private var instance: HumoursApplication? = null
        fun get(): HumoursApplication = instance!!
    }
}