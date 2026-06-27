package com.humours.client.data.repository

import com.humours.client.data.model.CatalogMessage
import com.humours.client.data.model.CatalogMetric
import com.humours.client.data.model.DataMessage
import com.humours.client.data.model.MetricNumber
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.concurrent.ConcurrentHashMap

data class LatestValue(
    val value: MetricNumber,
    val unit: String,
    val updatedAtMs: Long,
)

class MetricRepository(
    historyBufferDurationSeconds: Int = 120,
) {
    private val lock = Any()
    private val _latest: MutableMap<String, LatestValue> = ConcurrentHashMap()
    private val _catalog = MutableStateFlow<CatalogMessage>(CatalogMessage("catalog", emptyList()))
    val catalog: StateFlow<CatalogMessage> = _catalog.asStateFlow()

    private val historySeconds: Int = historyBufferDurationSeconds.coerceAtLeast(1)
    private val historyBufferDurationMs: Long = historySeconds * 1000L
    private val _history: MutableList<HistoryEntry> = mutableListOf()
    val history: List<HistoryEntry> get() = synchronized(lock) { _history.toList() }

    data class HistoryEntry(val timeMs: Long, val id: String, val value: MetricNumber, val unit: String)

    fun onCatalog(message: CatalogMessage) {
        _catalog.value = message
    }

    @Synchronized
    fun onData(message: DataMessage) {
        for (m in message.metrics) {
            _latest[m.id] = LatestValue(m.value, m.unit, message.timestamp)
        }
        val now = message.timestamp
        val cutoff = now - historyBufferDurationMs
        for (m in message.metrics) {
            _history.add(HistoryEntry(now, m.id, m.value, m.unit))
        }
        val iter = _history.iterator()
        while (iter.hasNext()) {
            if (iter.next().timeMs < cutoff) iter.remove() else break
        }
    }

    fun getLatest(id: String): LatestValue? = _latest[id]

    fun getLatestString(id: String): String? = _latest[id]?.value?.asString()

    fun getLatestFloat(id: String): Float? = _latest[id]?.value?.asF64()

    fun getCatalogEntry(id: String): CatalogMetric? =
        _catalog.value.metrics.firstOrNull { it.id == id }

    @Synchronized
    fun clear() {
        _latest.clear()
        _history.clear()
        _catalog.value = CatalogMessage("catalog", emptyList())
    }

    fun getHistoryFor(id: String): List<HistoryEntry> =
        synchronized(lock) { _history.filter { it.id == id } }
}