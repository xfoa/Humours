package com.humours.client.plugin.gl

import android.opengl.EGL14
import android.opengl.EGLConfig
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLExt
import android.opengl.EGLSurface
import android.view.Surface

class EglCore(shareContext: EGLContext = EGL14.EGL_NO_CONTEXT) {
    private var display: EGLDisplay = EGL14.EGL_NO_DISPLAY
    private var context: EGLContext = EGL14.EGL_NO_CONTEXT
    private var config: EGLConfig? = null

    init {
        val disp = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
        if (disp === EGL14.EGL_NO_DISPLAY) throw RuntimeException("no EGL display")
        display = disp
        val version = IntArray(2)
        if (!EGL14.eglInitialize(display, version, 0, version, 1)) {
            throw RuntimeException("eglInitialize failed")
        }
        val configAttribs = intArrayOf(
            EGL14.EGL_RED_SIZE, 8,
            EGL14.EGL_GREEN_SIZE, 8,
            EGL14.EGL_BLUE_SIZE, 8,
            EGL14.EGL_ALPHA_SIZE, 8,
            EGL14.EGL_DEPTH_SIZE, 16,
            EGL14.EGL_STENCIL_SIZE, 0,
            EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
            EGL14.EGL_SURFACE_TYPE, EGL14.EGL_WINDOW_BIT,
            EGL14.EGL_NONE,
        )
        val configs = arrayOfNulls<EGLConfig>(1)
        val numConfigs = IntArray(1)
        EGL14.eglChooseConfig(display, configAttribs, 0, configs, 0, 1, numConfigs, 0)
        if (numConfigs[0] == 0) throw RuntimeException("eglChooseConfig failed")
        config = configs[0]

        val contextAttribs = intArrayOf(
            EGL14.EGL_CONTEXT_CLIENT_VERSION, 2,
            EGL14.EGL_NONE,
        )
        context = EGL14.eglCreateContext(display, config, shareContext, contextAttribs, 0)
        if (context === EGL14.EGL_NO_CONTEXT) throw RuntimeException("eglCreateContext failed")
    }

    fun surface(surface: Surface): EGLSurface {
        val surfaceAttribs = intArrayOf(EGL14.EGL_NONE)
        val s = EGL14.eglCreateWindowSurface(display, config, surface, surfaceAttribs, 0)
        if (s == null || s === EGL14.EGL_NO_SURFACE) throw RuntimeException("eglCreateWindowSurface failed")
        return s
    }

    fun makeCurrent(eglSurface: EGLSurface) {
        if (!EGL14.eglMakeCurrent(display, eglSurface, eglSurface, context)) {
            throw RuntimeException("eglMakeCurrent failed")
        }
    }

    fun makeNothingCurrent() {
        EGL14.eglMakeCurrent(
            display,
            EGL14.EGL_NO_SURFACE,
            EGL14.EGL_NO_SURFACE,
            EGL14.EGL_NO_CONTEXT,
        )
    }

    fun swapBuffers(eglSurface: EGLSurface) {
        EGL14.eglSwapBuffers(display, eglSurface)
    }

    fun destroySurface(eglSurface: EGLSurface) {
        EGL14.eglDestroySurface(display, eglSurface)
    }

    fun release() {
        if (context !== EGL14.EGL_NO_CONTEXT) {
            EGL14.eglDestroyContext(display, context)
            context = EGL14.EGL_NO_CONTEXT
        }
        if (display !== EGL14.EGL_NO_DISPLAY) {
            EGL14.eglTerminate(display)
            display = EGL14.EGL_NO_DISPLAY
        }
    }

    fun eglContext(): EGLContext = context
    fun eglDisplay(): EGLDisplay = display
    fun eglConfig(): EGLConfig? = config
}