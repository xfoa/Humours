package com.humours.client.plugin.surface

import android.graphics.PixelFormat
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.content.Context
import android.view.Surface

class SurfaceManager(private val context: Context) {
    private var overlayView: SurfaceView? = null
    private var overlaySurface: Surface? = null

    fun attachOverlay(parent: ViewGroup): SurfaceView {
        detachOverlay()
        val v = SurfaceView(context).apply {
            setZOrderOnTop(true)
            holder.setFormat(PixelFormat.TRANSLUCENT)
            holder.addCallback(object : SurfaceHolder.Callback {
                override fun surfaceCreated(holder: SurfaceHolder) {
                    overlaySurface = holder.surface
                    listeners.forEach { it.onOverlayAvailable(holder.surface) }
                }
                override fun surfaceChanged(holder: SurfaceHolder, format: Int, w: Int, h: Int) {
                    listeners.forEach { it.onOverlaySize(w, h) }
                }
                override fun surfaceDestroyed(holder: SurfaceHolder) {
                    overlaySurface = null
                    listeners.forEach { it.onOverlayDestroyed() }
                }
            })
        }
        val params = ViewGroup.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT,
        )
        parent.addView(v, 0, params)
        overlayView = v
        return v
    }

    fun overlaySurface(): Surface? = overlaySurface

    fun detachOverlay() {
        overlayView?.let { (it.parent as? ViewGroup)?.removeView(it) }
        overlayView = null
        overlaySurface = null
    }

    private val listeners = mutableListOf<OverlayListener>()

    fun addOverlayListener(l: OverlayListener) { listeners.add(l) }
    fun removeOverlayListener(l: OverlayListener) { listeners.remove(l) }

    interface OverlayListener {
        fun onOverlayAvailable(surface: Surface)
        fun onOverlaySize(width: Int, height: Int) {}
        fun onOverlayDestroyed() {}
    }
}