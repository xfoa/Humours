package com.humours.client.plugin

import android.opengl.EGLSurface
import android.os.Handler
import android.os.Looper
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import com.humours.client.plugin.gl.GLThread
import com.humours.client.plugin.surface.SurfaceManager
import com.humours.client.data.repository.MetricRepository
import java.util.concurrent.ConcurrentHashMap

data class PluginInstance(
    val widgetId: String,
    val plugin: WidgetPlugin,
    val metrics: List<String>,
)

class PluginInstanceManager(
    private val glThread: GLThread,
    private val surfaceManager: SurfaceManager,
    private val repository: MetricRepository,
) {
    private val instances = ConcurrentHashMap<String, PluginInstance>()
    private val cellSurfaces = ConcurrentHashMap<String, EGLSurface>()
    private val renderHandlers = ConcurrentHashMap<String, Handler>()

    fun register(instance: PluginInstance) {
        instances[instance.widgetId] = instance
    }

    fun startRendering(
        widgetId: String,
        surfaceView: SurfaceView,
        onInvalidate: () -> Unit,
    ) {
        val instance = instances[widgetId] ?: return
        val key = widgetId
        surfaceView.holder.addCallback(object : SurfaceHolder.Callback {
            @Volatile private var started = false

            override fun surfaceCreated(holder: SurfaceHolder) {
                if (!started) {
                    val w = surfaceView.width.coerceAtLeast(1)
                    val h = surfaceView.height.coerceAtLeast(1)
                    start(holder.surface, w, h)
                }
            }

            override fun surfaceChanged(holder: SurfaceHolder, format: Int, w: Int, h: Int) {
                if (!started && w > 0 && h > 0) {
                    start(holder.surface, w, h)
                    return
                }
                if (started) {
                    instance.plugin.onResize(w, h)
                }
            }

            override fun surfaceDestroyed(holder: SurfaceHolder) {
                stopRenderLoop(key)
                val surface = cellSurfaces.remove(key)
                if (surface != null) {
                    glThread.destroySurfaceAsync(surface)
                }
                started = false
            }

            private fun start(surface: Surface, w: Int, h: Int) {
                started = true
                val result = GLThread.SurfaceResult()
                glThread.createSurfaceAsync(surface, result)
                val eglSurface = result.await()
                cellSurfaces[key] = eglSurface
                val overlay = surfaceManager.overlaySurface()
                val plugin = instance.plugin
                val needsInit = !plugin.isInitialized
                glThread.renderTo(eglSurface) {
                    if (needsInit) {
                        plugin.onCreate(surface, overlay, instance.metrics)
                    }
                    plugin.onResize(w, h)
                }
                startRenderLoop(key, eglSurface, instance, onInvalidate)
            }
        })
    }

    private fun startRenderLoop(
        key: String,
        surface: EGLSurface,
        instance: PluginInstance,
        onInvalidate: () -> Unit,
    ) {
        val handler = Handler(Looper.getMainLooper())
        renderHandlers[key] = handler
        val ticker = object : Runnable {
            override fun run() {
                if (!renderHandlers.containsKey(key)) return
                glThread.renderTo(surface) {
                    val drawer = instance.plugin as? GlDrawer
                    drawer?.onGlDraw(repository)
                }
                onInvalidate()
                handler.postDelayed(this, 16L)
            }
        }
        handler.post(ticker)
    }

    private fun stopRenderLoop(key: String) {
        renderHandlers.remove(key)
    }

    fun destroy(widgetId: String) {
        stopRenderLoop(widgetId)
        val instance = instances.remove(widgetId)
        val surface = cellSurfaces.remove(widgetId)
        if (surface != null) {
            glThread.renderTo(surface) {
                if (instance != null) {
                    instance.plugin.onDestroy()
                }
            }
            glThread.destroySurfaceAsync(surface)
        } else {
            instance?.plugin?.onDestroy()
        }
    }

    fun shutdown() {
        for (id in instances.keys.toList()) destroy(id)
    }
}

interface GlDrawer {
    fun onGlDraw(repository: MetricRepository)
}