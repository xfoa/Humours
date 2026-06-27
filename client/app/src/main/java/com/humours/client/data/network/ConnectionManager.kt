package com.humours.client.data.network

import android.util.Log
import com.humours.client.data.model.CatalogMessage
import com.humours.client.data.model.DataMessage
import com.humours.client.data.model.ErrorMessage
import com.humours.client.data.model.SubscribeEntry
import com.humours.client.data.model.SubscribeMessage
import com.humours.client.data.repository.MetricRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString

class ConnectionManager(
    private val httpClient: OkHttpClient,
    private val repository: MetricRepository,
    private val requestedMetrics: () -> Set<String>,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var webSocket: WebSocket? = null
    private var autoSubscribeJob: Job? = null
    @Volatile private var closing = false
    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    private val _state = MutableStateFlow<ConnectionState>(ConnectionState.Idle)
    val state: StateFlow<ConnectionState> = _state

    fun connect(host: String, port: Int, useTls: Boolean, token: String) {
        disconnect()
        val scheme = if (useTls) "wss" else "ws"
        val auth = if (token.isEmpty()) "" else "?token=$token"
        val url = "$scheme://$host:$port/ws$auth"
        _state.value = ConnectionState.Connecting(url)
        val request = Request.Builder().url(url).build()
        val ws = httpClient.newWebSocket(request, HumoursWebSocketListener(url))
        webSocket = ws
    }

    fun disconnect() {
        closing = true
        autoSubscribeJob?.cancel()
        autoSubscribeJob = null
        webSocket?.close(1000, "client disconnect")
        webSocket = null
        repository.clear()
        _state.value = ConnectionState.Idle
    }

    fun sendSubscribe(entries: List<SubscribeEntry>) {
        val msg = SubscribeMessage(msgType = "subscribe", metrics = entries)
        val text = json.encodeToString(SubscribeMessage.serializer(), msg)
        webSocket?.send(text)
    }

    private inner class HumoursWebSocketListener(private val url: String) : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            closing = false
            Log.i(TAG, "WebSocket connected to $url")
            _state.value = ConnectionState.Connected(url)
            autoSubscribeJob = scope.launch {
                repository.catalog.collect { catalog ->
                    if (catalog.metrics.isNotEmpty()) sendAutoSubscribe(catalog)
                    autoSubscribeJob?.cancel()
                }
            }
        }

        private fun sendAutoSubscribe(catalog: CatalogMessage) {
            val want = requestedMetrics()
            val entries = catalog.metrics
                .filter { it.id in want || want.isEmpty() }
                .map { m ->
                    if (m.isStatic) SubscribeEntry(id = m.id)
                    else SubscribeEntry(
                        id = m.id,
                        refreshRateMs = 1000,
                        unit = if (m.defaultUnit.isNotEmpty()) m.defaultUnit else null,
                    )
                }
            sendSubscribe(entries)
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            val typeField = runCatching {
                val raw = json.parseToJsonElement(text) as? kotlinx.serialization.json.JsonObject
                raw?.get("type")?.jsonPrimitive?.contentOrNull
            }.getOrNull()
            when (typeField) {
                "catalog" -> parseCatalog(text)
                "data" -> parseData(text)
                "error" -> parseError(text)
                else -> Log.w(TAG, "Unknown message type: $typeField")
            }
        }

        private fun parseCatalog(text: String) {
            runCatching {
                val msg = json.decodeFromString(CatalogMessage.serializer(), text)
                repository.onCatalog(msg)
            }.onFailure { Log.e(TAG, "Failed to parse catalog", it) }
        }

        private fun parseData(text: String) {
            runCatching {
                val msg = json.decodeFromString(DataMessage.serializer(), text)
                val ids = msg.metrics.joinToString(",") { it.id }
                Log.d(TAG, "Data msg: ${msg.metrics.size} metrics: $ids")
                repository.onData(msg)
            }.onFailure { Log.e(TAG, "Failed to parse data", it) }
        }

        private fun parseError(text: String) {
            runCatching {
                val msg = json.decodeFromString(ErrorMessage.serializer(), text)
                _state.value = ConnectionState.Failed(msg.message)
            }.onFailure { Log.e(TAG, "Failed to parse error", it) }
        }

        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {}

        override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
            webSocket.close(code, reason)
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            Log.w(TAG, "WebSocket closed: $code $reason")
            if (!closing) _state.value = ConnectionState.Failed("Closed: $reason ($code)")
            closing = false
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            Log.e(TAG, "WebSocket failure", t)
            if (!closing) _state.value = ConnectionState.Failed(t.message ?: "connection error")
            closing = false
        }
    }

    companion object {
        private const val TAG = "ConnectionManager"
    }
}