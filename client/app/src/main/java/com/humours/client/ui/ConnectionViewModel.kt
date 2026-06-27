package com.humours.client.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.humours.client.HumoursApplication
import com.humours.client.data.local.ConnectionSettings
import com.humours.client.data.network.ConnectionManager
import com.humours.client.data.network.ConnectionState
import com.humours.client.data.repository.MetricRepository
import com.humours.client.plugin.PluginInstance
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

class ConnectionViewModel : ViewModel() {
    private val app = HumoursApplication.get()
    private val store = app.settingsStore
    private val connectionManager = app.connectionManager
    private val repository = app.repository

    private val _settings = MutableStateFlow(ConnectionSettings("", 8443, true, ""))
    val settings: StateFlow<ConnectionSettings> = _settings

    val connectionState: StateFlow<ConnectionState> = connectionManager.state
    val catalog = repository.catalog

    init {
        viewModelScope.launch {
            store.settings.collect { _settings.value = it }
        }
    }

    fun connect(host: String, port: Int, token: String) {
        viewModelScope.launch {
            store.save(ConnectionSettings(host, port, true, token))
            connectionManager.connect(host, port, true, token)
        }
    }

    fun disconnect() {
        connectionManager.disconnect()
    }

    fun registerBuiltInWidget(widgetId: String) {
        val plugin = app.pluginLoader.load(app.builtInPluginClassName) ?: return
        app.pluginInstanceManager.register(
            PluginInstance(widgetId, plugin, com.humours.client.builtin.CubeTetrahedronPlugin.CUBE_METRICS)
        )
    }
}