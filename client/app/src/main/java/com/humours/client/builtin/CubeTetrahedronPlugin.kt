package com.humours.client.builtin

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Typeface
import android.opengl.GLES20
import android.view.Surface
import com.humours.client.data.repository.MetricRepository
import com.humours.client.plugin.GlDrawer
import com.humours.client.plugin.WidgetMetadata
import com.humours.client.plugin.WidgetPlugin
import com.humours.client.plugin.gl.GLUtils as GlHelper
import com.humours.client.plugin.gl.Matrix
import java.nio.ByteBuffer
import java.nio.ByteOrder

class CubeTetrahedronPlugin : WidgetPlugin, GlDrawer {
    override val metadata = WidgetMetadata(
        id = "cube_tetrahedron",
        name = "Cube + Tetrahedron",
        author = "Humours",
        supportedMetrics = CUBE_METRICS,
        defaultSize = 2 to 2,
    )

    override val isInitialized: Boolean get() = glInitialized

    private var cellSurface: Surface? = null
    private var overlaySurface: Surface? = null
    private var metrics: List<String> = emptyList()
    private var width = 1
    private var height = 1

    private var cubeProgram = 0
    private var tetraProgram = 0

    private var cubeBuffers = IntArray(0)
    private var cubeColorBuffer = 0
    private var cubeUvBuffer = 0
    private var tetraBuffers = IntArray(0)
    private var tetraColorBuffer = 0

    private val faceTextures = IntArray(6)
    private val lastFaceValues = arrayOfNulls<String>(6)
    private val faceTextureSizes = Array(6) { 0 to 0 }

    private var overlayWidth = 1
    private var overlayHeight = 1

    private var startTime = System.currentTimeMillis()
    private var glInitialized = false

    @Volatile private var lastFlightStart = 0L
    private val flightDurationMs = 1500f
    private val flightIntervalMs = 5000L

    private val mvpMatrix = FloatArray(16)
    private val modelMatrix = FloatArray(16)
    private val viewMatrix = FloatArray(16)
    private val projectionMatrix = FloatArray(16)
    private val tempMatrix = FloatArray(16)

    override fun onCreate(cellSurface: Surface, overlaySurface: Surface?, metrics: List<String>) {
        this.cellSurface = cellSurface
        this.overlaySurface = overlaySurface
        this.metrics = metrics
        if (glInitialized) return
        glInitialized = true
        startTime = System.currentTimeMillis()
        initGl()
        for (i in 0 until 6) lastFaceValues[i] = null
    }

    override fun onResize(width: Int, height: Int) {
        this.width = width.coerceAtLeast(1)
        this.height = height.coerceAtLeast(1)
    }

    override fun onDestroy() {
        cellSurface = null
        overlaySurface = null
        val tex = faceTextures
        if (tex.any { it != 0 }) GLES20.glDeleteTextures(6, tex, 0)
        if (cubeBuffers.isNotEmpty()) GLES20.glDeleteBuffers(2, cubeBuffers, 0)
        if (cubeColorBuffer != 0) GLES20.glDeleteBuffers(1, intArrayOf(cubeColorBuffer), 0)
        if (cubeUvBuffer != 0) GLES20.glDeleteBuffers(1, intArrayOf(cubeUvBuffer), 0)
        if (tetraBuffers.isNotEmpty()) GLES20.glDeleteBuffers(1, tetraBuffers, 0)
        if (tetraColorBuffer != 0) GLES20.glDeleteBuffers(1, intArrayOf(tetraColorBuffer), 0)
        if (cubeProgram != 0) GLES20.glDeleteProgram(cubeProgram)
        if (tetraProgram != 0) GLES20.glDeleteProgram(tetraProgram)
        for (i in 0 until 6) faceTextures[i] = 0
        cubeBuffers = IntArray(0)
        cubeColorBuffer = 0
        cubeUvBuffer = 0
        tetraBuffers = IntArray(0)
        tetraColorBuffer = 0
        cubeProgram = 0; tetraProgram = 0
        glInitialized = false
    }

    private fun initGl() {
        cubeProgram = GlHelper.linkProgram(CUBE_VERT, CUBE_FRAG)
        tetraProgram = GlHelper.linkProgram(TETRA_VERT, TETRA_FRAG)
        buildCube()
        buildTetra()
        GLES20.glGenTextures(6, faceTextures, 0)
        for (i in 0 until 6) {
            GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, faceTextures[i])
            GLES20.glTexParameterf(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_LINEAR.toFloat())
            GLES20.glTexParameterf(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_LINEAR.toFloat())
            GLES20.glTexParameterf(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_CLAMP_TO_EDGE.toFloat())
            GLES20.glTexParameterf(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_WRAP_T, GLES20.GL_CLAMP_TO_EDGE.toFloat())
        }
    }

    private fun buildCube() {
        val verts = floatArrayOf(
            -1f,-1f, 1f,  1f,-1f, 1f,  1f, 1f, 1f, -1f, 1f, 1f,
            -1f,-1f,-1f, -1f, 1f,-1f,  1f, 1f,-1f,  1f,-1f,-1f,
            -1f, 1f,-1f, -1f, 1f, 1f,  1f, 1f, 1f,  1f, 1f,-1f,
            -1f,-1f,-1f,  1f,-1f,-1f,  1f,-1f, 1f, -1f,-1f, 1f,
             1f,-1f,-1f,  1f, 1f,-1f,  1f, 1f, 1f,  1f,-1f, 1f,
            -1f,-1f,-1f, -1f,-1f, 1f, -1f, 1f, 1f, -1f, 1f,-1f,
        )
        val normals = FloatArray(72)
        for (f in 0 until 6) {
            val base = f * 4
            val n = when (f) { 0 -> floatArrayOf(0f,0f,1f); 1 -> floatArrayOf(0f,0f,-1f);
                2 -> floatArrayOf(0f,1f,0f); 3 -> floatArrayOf(0f,-1f,0f);
                4 -> floatArrayOf(1f,0f,0f); else -> floatArrayOf(-1f,0f,0f) }
            for (v in 0..3) {
                val o = (base + v) * 3
                normals[o] = n[0]; normals[o+1] = n[1]; normals[o+2] = n[2]
            }
        }
        cubeBuffers = IntArray(2)
        GLES20.glGenBuffers(2, cubeBuffers, 0)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, cubeBuffers[0])
        val vbb = ByteBuffer.allocateDirect(verts.size * 4).order(ByteOrder.nativeOrder())
        vbb.asFloatBuffer().put(verts)
        GLES20.glBufferData(GLES20.GL_ARRAY_BUFFER, verts.size * 4, vbb, GLES20.GL_STATIC_DRAW)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, 0)

        cubeColorBuffer = IntArray(1).also { GLES20.glGenBuffers(1, it, 0) }[0]
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, cubeColorBuffer)
        val cv = ByteBuffer.allocateDirect(normals.size * 4).order(ByteOrder.nativeOrder())
        cv.asFloatBuffer().put(normals)
        GLES20.glBufferData(GLES20.GL_ARRAY_BUFFER, normals.size * 4, cv, GLES20.GL_STATIC_DRAW)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, 0)

        cubeUvBuffer = IntArray(1).also { GLES20.glGenBuffers(1, it, 0) }[0]
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, cubeUvBuffer)
        val uvs = floatArrayOf(
            0f,1f, 1f,1f, 1f,0f, 0f,0f,
            0f,1f, 1f,1f, 1f,0f, 0f,0f,
            0f,1f, 1f,1f, 1f,0f, 0f,0f,
            0f,1f, 1f,1f, 1f,0f, 0f,0f,
            0f,1f, 1f,1f, 1f,0f, 0f,0f,
            0f,1f, 1f,1f, 1f,0f, 0f,0f,
        )
        val uvBb = ByteBuffer.allocateDirect(uvs.size * 4).order(ByteOrder.nativeOrder())
        uvBb.asFloatBuffer().put(uvs)
        GLES20.glBufferData(GLES20.GL_ARRAY_BUFFER, uvs.size * 4, uvBb, GLES20.GL_STATIC_DRAW)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, 0)

        GLES20.glBindBuffer(GLES20.GL_ELEMENT_ARRAY_BUFFER, cubeBuffers[1])
        val indices = shortArrayOf(
            0,1,2, 0,2,3,
            4,5,6, 4,6,7,
            8,9,10, 8,10,11,
            12,13,14, 12,14,15,
            16,17,18, 16,18,19,
            20,21,22, 20,22,23,
        )
        val ib = ByteBuffer.allocateDirect(indices.size * 2).order(ByteOrder.nativeOrder())
        ib.asShortBuffer().put(indices)
        GLES20.glBufferData(GLES20.GL_ELEMENT_ARRAY_BUFFER, indices.size * 2, ib, GLES20.GL_STATIC_DRAW)
        GLES20.glBindBuffer(GLES20.GL_ELEMENT_ARRAY_BUFFER, 0)
    }

    private fun buildTetra() {
        val r = 1f
        val a = floatArrayOf( r, r, r)
        val b = floatArrayOf( r,-r,-r)
        val c = floatArrayOf(-r, r,-r)
        val d = floatArrayOf(-r,-r, r)
        val verts = floatArrayOf(
            a[0],a[1],a[2], c[0],c[1],c[2], b[0],b[1],b[2],
            a[0],a[1],a[2], b[0],b[1],b[2], d[0],d[1],d[2],
            a[0],a[1],a[2], d[0],d[1],d[2], c[0],c[1],c[2],
            b[0],b[1],b[2], c[0],c[1],c[2], d[0],d[1],d[2],
        )
        val colors = FloatArray(48)
        for (i in 0 until 4) {
            val hue = (i / 4f)
            val hsv = floatArrayOf(hue * 360f, 0.85f, 1f)
            val rgb = android.graphics.Color.HSVToColor(hsv)
            for (v in 0..2) {
                val o = (i * 3 + v) * 4
                colors[o] = Color.red(rgb) / 255f
                colors[o+1] = Color.green(rgb) / 255f
                colors[o+2] = Color.blue(rgb) / 255f
                colors[o+3] = 1f
            }
        }
        tetraBuffers = IntArray(1).also { GLES20.glGenBuffers(1, it, 0) }
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, tetraBuffers[0])
        val vBb = ByteBuffer.allocateDirect(verts.size * 4).order(ByteOrder.nativeOrder())
        vBb.asFloatBuffer().put(verts)
        GLES20.glBufferData(GLES20.GL_ARRAY_BUFFER, verts.size * 4, vBb, GLES20.GL_STATIC_DRAW)

        tetraColorBuffer = IntArray(1).also { GLES20.glGenBuffers(1, it, 0) }[0]
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, tetraColorBuffer)
        val cBb = ByteBuffer.allocateDirect(colors.size * 4).order(ByteOrder.nativeOrder())
        cBb.asFloatBuffer().put(colors)
        GLES20.glBufferData(GLES20.GL_ARRAY_BUFFER, colors.size * 4, cBb, GLES20.GL_STATIC_DRAW)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, 0)
    }


    override fun onGlDraw(repository: MetricRepository) {
        drawCube(repository)
        if (overlaySurface != null) drawOverlay(repository)
    }

    private fun drawCube(repository: MetricRepository) {
        GLES20.glViewport(0, 0, width, height)
        GLES20.glEnable(GLES20.GL_DEPTH_TEST)
        GLES20.glEnable(GLES20.GL_CULL_FACE)
        GLES20.glCullFace(GLES20.GL_BACK)
        GLES20.glClearColor(0.02f, 0.02f, 0.02f, 1f)
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT or GLES20.GL_DEPTH_BUFFER_BIT)

        GLES20.glUseProgram(cubeProgram)
        val posLoc = GLES20.glGetAttribLocation(cubeProgram, "aPos")
        val norLoc = GLES20.glGetAttribLocation(cubeProgram, "aNormal")
        val uvLoc = GLES20.glGetAttribLocation(cubeProgram, "aUv")
        val matLoc = GLES20.glGetUniformLocation(cubeProgram, "uMvp")
        val modelLoc = GLES20.glGetUniformLocation(cubeProgram, "uModel")
        val lightLoc = GLES20.glGetUniformLocation(cubeProgram, "uLightDir")
        val texLoc = GLES20.glGetUniformLocation(cubeProgram, "uTexture")

        val aspect = width.toFloat() / height.toFloat().coerceAtLeast(1f)
        Matrix.perspectiveM(projectionMatrix, 45f, aspect, 0.1f, 100f)
        Matrix.setIdentity(viewMatrix)
        Matrix.translateM(viewMatrix, viewMatrix, 0f, 0f, -6f)

        val t = (System.currentTimeMillis() - startTime) / 1000f
        Matrix.setIdentity(modelMatrix)
        Matrix.rotateM(modelMatrix, modelMatrix, t * 30f, 0.4f, 1f, 0f)
        Matrix.rotateM(modelMatrix, modelMatrix, t * 20f, 1f, 0.3f, 0f)
        Matrix.scaleM(modelMatrix, modelMatrix, 1.4f, 1.4f, 1.4f)

        Matrix.multiply(tempMatrix, projectionMatrix, viewMatrix)
        Matrix.multiply(mvpMatrix, tempMatrix, modelMatrix)

        GLES20.glUniformMatrix4fv(matLoc, 1, false, mvpMatrix, 0)
        GLES20.glUniformMatrix4fv(modelLoc, 1, false, modelMatrix, 0)
        GLES20.glUniform3f(lightLoc, 0.5f, 0.7f, 0.8f)
        GLES20.glUniform1i(texLoc, 0)

        GLES20.glActiveTexture(GLES20.GL_TEXTURE0)

        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, cubeBuffers[0])
        GLES20.glEnableVertexAttribArray(posLoc)
        GLES20.glVertexAttribPointer(posLoc, 3, GLES20.GL_FLOAT, false, 0, 0)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, cubeColorBuffer)
        GLES20.glEnableVertexAttribArray(norLoc)
        GLES20.glVertexAttribPointer(norLoc, 3, GLES20.GL_FLOAT, false, 0, 0)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, cubeUvBuffer)
        GLES20.glEnableVertexAttribArray(uvLoc)
        GLES20.glVertexAttribPointer(uvLoc, 2, GLES20.GL_FLOAT, false, 0, 0)

        GLES20.glBindBuffer(GLES20.GL_ELEMENT_ARRAY_BUFFER, cubeBuffers[1])
        for (i in 0 until 6) {
            updateFaceTexture(repository, i)
            GLES20.glDrawElements(GLES20.GL_TRIANGLES, 6, GLES20.GL_UNSIGNED_SHORT, i * 12)
        }
        GLES20.glDisableVertexAttribArray(posLoc)
        GLES20.glDisableVertexAttribArray(norLoc)
        GLES20.glDisableVertexAttribArray(uvLoc)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, 0)
        GLES20.glBindBuffer(GLES20.GL_ELEMENT_ARRAY_BUFFER, 0)
    }

    private fun drawOverlay(repository: MetricRepository) {
        if (overlaySurface == null) return
        val now = System.currentTimeMillis()
        if (lastFlightStart == 0L) lastFlightStart = now
        val elapsed = now - lastFlightStart
        val progress = (elapsed / flightDurationMs).coerceIn(0f, 1f)
        if (elapsed < flightIntervalMs && progress < 1f) {
            drawTetraFlight(progress)
        } else if (progress >= 1f && elapsed < flightIntervalMs) {
            clearOverlay()
        } else if (elapsed >= flightIntervalMs) {
            lastFlightStart = now
        }
    }

    private fun drawTetraFlight(progress: Float) {
        GLES20.glViewport(0, 0, overlayWidth, overlayHeight)
        GLES20.glDisable(GLES20.GL_DEPTH_TEST)
        GLES20.glDisable(GLES20.GL_CULL_FACE)
        GLES20.glEnable(GLES20.GL_BLEND)
        GLES20.glBlendFunc(GLES20.GL_SRC_ALPHA, GLES20.GL_ONE_MINUS_SRC_ALPHA)
        GLES20.glClearColor(0f, 0f, 0f, 0f)
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)

        val aspect = overlayWidth.toFloat() / overlayHeight.toFloat().coerceAtLeast(1f)
        Matrix.perspectiveM(projectionMatrix, 60f, aspect, 0.1f, 100f)
        Matrix.setIdentity(viewMatrix)
        Matrix.translateM(viewMatrix, viewMatrix, 0f, 0f, -6f)

        val x = -4f + 8f * progress
        val y = (Math.sin((progress * Math.PI).toDouble())).toFloat() * 1.2f
        Matrix.setIdentity(modelMatrix)
        Matrix.translateM(modelMatrix, modelMatrix, x, y, 0f)
        Matrix.rotateM(modelMatrix, modelMatrix, progress * 720f, 0f, 1f, 0f)
        Matrix.scaleM(modelMatrix, modelMatrix, 0.7f, 0.7f, 0.7f)

        Matrix.multiply(tempMatrix, projectionMatrix, viewMatrix)
        Matrix.multiply(mvpMatrix, tempMatrix, modelMatrix)

        val hueShift = (System.currentTimeMillis() % 1000) / 1000f

        GLES20.glUseProgram(tetraProgram)
        val posLoc = GLES20.glGetAttribLocation(tetraProgram, "aPos")
        val colLoc = GLES20.glGetAttribLocation(tetraProgram, "aColor")
        val matLoc = GLES20.glGetUniformLocation(tetraProgram, "uMvp")
        val hueLoc = GLES20.glGetUniformLocation(tetraProgram, "uHueShift")
        GLES20.glUniformMatrix4fv(matLoc, 1, false, mvpMatrix, 0)
        GLES20.glUniform1f(hueLoc, hueShift)

        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, tetraBuffers[0])
        GLES20.glEnableVertexAttribArray(posLoc)
        GLES20.glVertexAttribPointer(posLoc, 3, GLES20.GL_FLOAT, false, 0, 0)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, tetraColorBuffer)
        GLES20.glEnableVertexAttribArray(colLoc)
        GLES20.glVertexAttribPointer(colLoc, 4, GLES20.GL_FLOAT, false, 0, 0)
        GLES20.glDrawArrays(GLES20.GL_TRIANGLES, 0, 12)
        GLES20.glDisableVertexAttribArray(posLoc)
        GLES20.glDisableVertexAttribArray(colLoc)
        GLES20.glBindBuffer(GLES20.GL_ARRAY_BUFFER, 0)
        GLES20.glDisable(GLES20.GL_BLEND)
    }

    private fun clearOverlay() {
        GLES20.glViewport(0, 0, overlayWidth, overlayHeight)
        GLES20.glDisable(GLES20.GL_DEPTH_TEST)
        GLES20.glClearColor(0f, 0f, 0f, 0f)
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)
    }

    private fun formatValue(metricId: String, latest: com.humours.client.data.repository.LatestValue?): String {
        if (latest == null) return "--"
        val valueText = if (metricId == "sys.uptime" && latest.value is com.humours.client.data.model.FloatValue) {
            "${latest.value.asF64().toInt()}"
        } else {
            latest.value.asString()
        }
        val unit = latest.unit
        return if (unit.isNotEmpty()) "$valueText $unit" else valueText
    }

    private fun updateFaceTexture(repository: MetricRepository, faceIndex: Int) {
        val metricId = CUBE_METRICS.getOrNull(faceIndex) ?: return
        val latest = repository.getLatest(metricId)
        if (latest == null) {
            android.util.Log.d("CubePlugin", "face=$faceIndex metric=$metricId value=null")
        }
        val text = formatValue(metricId, latest)
        if (text == lastFaceValues[faceIndex] && faceTextures[faceIndex] != 0) {
            GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, faceTextures[faceIndex])
            return
        }
        lastFaceValues[faceIndex] = text
        val bitmap = renderFaceBitmap(faceIndex, text)
        GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, faceTextures[faceIndex])
        android.opengl.GLUtils.texImage2D(GLES20.GL_TEXTURE_2D, 0, bitmap, 0)
        faceTextureSizes[faceIndex] = bitmap.width to bitmap.height
        bitmap.recycle()
    }


    private fun renderFaceBitmap(faceIndex: Int, text: String): Bitmap {
        val size = 256
        val bmp = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bmp)
        val bg = Paint().apply { color = FACE_BG_COLORS[faceIndex]; style = Paint.Style.FILL }
        canvas.drawRect(0f, 0f, size.toFloat(), size.toFloat(), bg)
        val labelPaint = Paint().apply {
            color = Color.WHITE
            textSize = 28f
            typeface = Typeface.DEFAULT_BOLD
            isAntiAlias = true
            textAlign = Paint.Align.CENTER
        }
        val valuePaint = Paint().apply {
            color = Color.WHITE
            textSize = 34f
            typeface = Typeface.DEFAULT_BOLD
            isAntiAlias = true
            textAlign = Paint.Align.CENTER
        }
        canvas.drawText(CUBE_METRICS[faceIndex], size / 2f, 60f, labelPaint)
        canvas.drawText(text, size / 2f, size / 2f, valuePaint)
        return bmp
    }

    companion object {
        val CUBE_METRICS = listOf(
            "cpu.usage", "sys.uptime", "mem.free",
            "os.version", "os.hostname", "cpu.temp",
        )
        val FACE_BG_COLORS = intArrayOf(
            Color.rgb(70, 40, 120), Color.rgb(20, 80, 110), Color.rgb(110, 50, 30),
            Color.rgb(30, 90, 50), Color.rgb(120, 90, 20), Color.rgb(120, 30, 60),
        )

        const val CUBE_VERT = """
            attribute vec3 aPos;
            attribute vec3 aNormal;
            attribute vec2 aUv;
            uniform mat4 uMvp;
            uniform mat4 uModel;
            varying vec3 vNormal;
            varying vec2 vUv;
            void main() {
                vNormal = mat3(uModel) * aNormal;
                vUv = aUv;
                gl_Position = uMvp * vec4(aPos, 1.0);
            }
        """
        const val CUBE_FRAG = """
            precision mediump float;
            varying vec3 vNormal;
            varying vec2 vUv;
            uniform vec3 uLightDir;
            uniform sampler2D uTexture;
            void main() {
                vec3 n = normalize(vNormal);
                float d = max(dot(n, normalize(uLightDir)), 0.15);
                vec4 tex = texture2D(uTexture, vUv);
                gl_FragColor = vec4(tex.rgb * d, 1.0);
            }
        """
        const val TETRA_VERT = """
            attribute vec3 aPos;
            attribute vec4 aColor;
            uniform mat4 uMvp;
            varying vec4 vColor;
            void main() {
                vColor = aColor;
                gl_Position = uMvp * vec4(aPos, 1.0);
            }
        """
        const val TETRA_FRAG = """
            precision mediump float;
            varying vec4 vColor;
            uniform float uHueShift;
            void main() {
                vec3 c = vColor.rgb;
                float h = fract(vColor.r + uHueShift);
                vec3 shifted = vec3(h, vColor.g, vColor.b);
                gl_FragColor = vec4(mix(vColor.rgb, shifted, 0.5), 0.9);
            }
        """
    }
}