package com.humours.client.data.local

import android.content.Context
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.dataStore by preferencesDataStore(name = "humours_settings")

data class ConnectionSettings(
    val host: String,
    val port: Int,
    val useTls: Boolean,
    val token: String,
)

class SettingsStore(private val context: Context) {
    private val keyHost = stringPreferencesKey("host")
    private val keyPort = stringPreferencesKey("port")
    private val keyTls = stringPreferencesKey("use_tls")
    private val keyToken = stringPreferencesKey("auth_token")

    val settings: Flow<ConnectionSettings> = context.dataStore.data.map { p ->
        ConnectionSettings(
            host = p[keyHost] ?: "",
            port = (p[keyPort] ?: "8443").toIntOrNull() ?: 8443,
            useTls = (p[keyTls] ?: "true").toBooleanStrictOrNull() ?: true,
            token = p[keyToken] ?: "changeme",
        )
    }

    suspend fun save(settings: ConnectionSettings) {
        context.dataStore.edit { p ->
            p[keyHost] = settings.host
            p[keyPort] = settings.port.toString()
            p[keyTls] = settings.useTls.toString()
            p[keyToken] = settings.token
        }
    }
}