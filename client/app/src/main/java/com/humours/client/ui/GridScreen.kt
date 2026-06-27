package com.humours.client.ui

import android.view.SurfaceView
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.humours.client.HumoursApplication
import com.humours.client.builtin.CubeTetrahedronPlugin
import com.humours.client.plugin.PluginInstance

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GridScreen(onDisconnect: () -> Unit) {
    val context = LocalContext.current
    val app = remember { context.applicationContext as HumoursApplication }
    var cubesVisible by remember { mutableStateOf(true) }

    fun doDisconnect() {
        cubesVisible = false
        app.pluginInstanceManager.shutdown()
        app.connectionManager.disconnect()
        onDisconnect()
    }

    BackHandler(enabled = true) { doDisconnect() }

    DisposableEffect(Unit) {
        onDispose {
            app.pluginInstanceManager.shutdown()
            app.surfaceManager.detachOverlay()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Grid") },
                actions = {
                    TextButton(onClick = { doDisconnect() }) { Text("Disconnect") }
                }
            )
        }
    ) { padding ->
        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            modifier = Modifier.fillMaxSize().padding(padding).padding(8.dp),
        ) {
            if (cubesVisible) {
                items(listOf("widget-cube")) { widgetId ->
                    WidgetGridCell(widgetId)
                }
            }
        }
    }
}

@Composable
private fun WidgetGridCell(widgetId: String) {
    val context = LocalContext.current
    val app = remember { context.applicationContext as HumoursApplication }
    DisposableEffect(widgetId) {
        onDispose {
            app.pluginInstanceManager.destroy(widgetId)
        }
    }
    AndroidView(
        factory = { ctx ->
            FrameLayout(ctx).apply {
                val sv = SurfaceView(ctx)
                sv.holder.setFormat(android.graphics.PixelFormat.TRANSLUCENT)
                sv.visibility = android.view.View.INVISIBLE
                addView(
                    sv,
                    FrameLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.MATCH_PARENT,
                    )
                )
                val metrics = CubeTetrahedronPlugin.CUBE_METRICS
                val plugin = app.pluginLoader.load(app.builtInPluginClassName)
                if (plugin != null) {
                    app.pluginInstanceManager.register(
                        PluginInstance(widgetId, plugin, metrics)
                    )
                    app.pluginInstanceManager.startRendering(widgetId, sv) {}
                }
                postDelayed({ sv.visibility = android.view.View.VISIBLE }, 300)
                this
            }
        },
        modifier = Modifier
            .fillMaxWidth()
            .height(360.dp),
    )
}
