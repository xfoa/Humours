package com.humours.client.plugin.gl

import android.opengl.EGLSurface
import android.os.Handler
import android.os.HandlerThread
import android.os.Message
import android.view.Surface
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class GLThread(name: String = "HumoursGLThread") : HandlerThread(name) {
    var core: EglCore? = null
        private set
    private lateinit var handlerImpl: Handler

    class SurfaceResult {
        private val latch = CountDownLatch(1)
        @Volatile var surface: EGLSurface? = null
            private set
        @Volatile var error: Throwable? = null
        fun set(s: EGLSurface) { surface = s; latch.countDown() }
        fun fail() { latch.countDown() }
        fun await(timeoutMs: Long = 5000L): EGLSurface {
            latch.await(timeoutMs, TimeUnit.MILLISECONDS)
            error?.let { throw RuntimeException("surface creation failed", it) }
            return surface ?: throw RuntimeException("surface creation timed out")
        }
    }

    fun startAndPrepare() {
        start()
        val ready = CountDownLatch(1)
        handlerImpl = object : Handler(looper) {
            override fun handleMessage(msg: Message) {
                when (msg.what) {
                    MSG_CREATE_SURFACE -> {
                        val args = msg.obj as SurfaceArgs
                        val c = core ?: return
                        runCatching { c.surface(args.surface) }
                            .onSuccess { args.result.set(it) }
                            .onFailure { args.result.error = it; args.result.fail() }
                    }
                    MSG_DESTROY_SURFACE -> {
                        val c = core ?: return
                        val s = msg.obj as EGLSurface
                        c.makeNothingCurrent()
                        c.destroySurface(s)
                    }
                    MSG_RENDER -> {
                        val c = core ?: return
                        val args = msg.obj as RenderArgs
                        runCatching {
                            c.makeCurrent(args.surface)
                            args.draw()
                            c.swapBuffers(args.surface)
                            c.makeNothingCurrent()
                        }.onFailure { android.util.Log.e(TAG, "render failed", it) }
                    }
                    MSG_QUIT -> quitSafely()
                }
            }
        }
        handlerImpl.post {
            core = EglCore().also { ready.countDown() }
        }
        ready.await(5, TimeUnit.SECONDS)
    }

    fun handler(): Handler = handlerImpl

    fun createSurfaceAsync(surface: Surface, result: SurfaceResult) {
        val args = SurfaceArgs(surface, result)
        handlerImpl.sendMessage(handlerImpl.obtainMessage(MSG_CREATE_SURFACE, args))
    }

    fun destroySurfaceAsync(surface: EGLSurface) {
        handlerImpl.sendMessage(handlerImpl.obtainMessage(MSG_DESTROY_SURFACE, surface))
    }

    fun renderTo(surface: EGLSurface, draw: () -> Unit) {
        val args = RenderArgs(surface, draw)
        handlerImpl.sendMessage(handlerImpl.obtainMessage(MSG_RENDER, args))
    }

    fun destroyGl() {
        handlerImpl.sendEmptyMessage(MSG_QUIT)
    }

    private data class SurfaceArgs(val surface: Surface, val result: SurfaceResult)
    private data class RenderArgs(val surface: EGLSurface, val draw: () -> Unit)

    companion object {
        private const val TAG = "GLThread"
        private const val MSG_CREATE_SURFACE = 1
        private const val MSG_DESTROY_SURFACE = 2
        private const val MSG_RENDER = 3
        private const val MSG_QUIT = 4
    }
}