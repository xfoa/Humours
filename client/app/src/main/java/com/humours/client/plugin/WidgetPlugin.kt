package com.humours.client.plugin

import android.view.Surface

data class WidgetMetadata(
    val id: String,
    val name: String,
    val author: String,
    val supportedMetrics: List<String>,
    val defaultSize: Pair<Int, Int>,
)

interface WidgetPlugin {
    val metadata: WidgetMetadata

    val isInitialized: Boolean

    fun onCreate(cellSurface: Surface, overlaySurface: Surface?, metrics: List<String>)
    fun onResize(width: Int, height: Int)
    fun onDestroy()
}