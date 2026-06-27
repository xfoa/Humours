package com.humours.client.plugin.gl

import android.opengl.GLES20

object GLUtils {
    fun compileShader(type: Int, source: String): Int {
        val shader = GLES20.glCreateShader(type)
        GLES20.glShaderSource(shader, source)
        GLES20.glCompileShader(shader)
        val status = IntArray(1)
        GLES20.glGetShaderiv(shader, GLES20.GL_COMPILE_STATUS, status, 0)
        if (status[0] == 0) {
            val log = GLES20.glGetShaderInfoLog(shader)
            GLES20.glDeleteShader(shader)
            throw RuntimeException("Shader compile failed: $log")
        }
        return shader
    }

    fun linkProgram(vertexSrc: String, fragmentSrc: String): Int {
        val vs = compileShader(GLES20.GL_VERTEX_SHADER, vertexSrc)
        val fs = compileShader(GLES20.GL_FRAGMENT_SHADER, fragmentSrc)
        val program = GLES20.glCreateProgram()
        GLES20.glAttachShader(program, vs)
        GLES20.glAttachShader(program, fs)
        GLES20.glLinkProgram(program)
        val status = IntArray(1)
        GLES20.glGetProgramiv(program, GLES20.GL_LINK_STATUS, status, 0)
        if (status[0] == 0) {
            val log = GLES20.glGetProgramInfoLog(program)
            GLES20.glDeleteProgram(program)
            throw RuntimeException("Program link failed: $log")
        }
        return program
    }
}

object Matrix {
    fun multiply(out: FloatArray, a: FloatArray, b: FloatArray) {
        val a00 = a[0]; val a01 = a[1]; val a02 = a[2]; val a03 = a[3]
        val a10 = a[4]; val a11 = a[5]; val a12 = a[6]; val a13 = a[7]
        val a20 = a[8]; val a21 = a[9]; val a22 = a[10]; val a23 = a[11]
        val a30 = a[12]; val a31 = a[13]; val a32 = a[14]; val a33 = a[15]
        for (col in 0 until 4) {
            val b0 = b[col * 4]; val b1 = b[col * 4 + 1]; val b2 = b[col * 4 + 2]; val b3 = b[col * 4 + 3]
            out[col * 4] = a00 * b0 + a10 * b1 + a20 * b2 + a30 * b3
            out[col * 4 + 1] = a01 * b0 + a11 * b1 + a21 * b2 + a31 * b3
            out[col * 4 + 2] = a02 * b0 + a12 * b1 + a22 * b2 + a32 * b3
            out[col * 4 + 3] = a03 * b0 + a13 * b1 + a23 * b2 + a33 * b3
        }
    }

    fun frustumM(out: FloatArray, left: Float, right: Float, bottom: Float, top: Float, near: Float, far: Float) {
        val rl = right - left
        val tb = top - bottom
        val nf = near - far
        out[0] = 2 * near / rl; out[1] = 0f; out[2] = 0f; out[3] = 0f
        out[4] = 0f; out[5] = 2 * near / tb; out[6] = 0f; out[7] = 0f
        out[8] = (right + left) / rl; out[9] = (top + bottom) / tb; out[10] = far / nf; out[11] = -1f
        out[12] = 0f; out[13] = 0f; out[14] = far * near / nf; out[15] = 0f
    }

    fun perspectiveM(out: FloatArray, fovyDeg: Float, aspect: Float, near: Float, far: Float) {
        val f = (1.0 / Math.tan(Math.toRadians(fovyDeg.toDouble()))).toFloat()
        out[0] = f / aspect; out[1] = 0f; out[2] = 0f; out[3] = 0f
        out[4] = 0f; out[5] = f; out[6] = 0f; out[7] = 0f
        out[8] = 0f; out[9] = 0f; out[10] = (far + near) / (near - far); out[11] = -1f
        out[12] = 0f; out[13] = 0f; out[14] = 2 * far * near / (near - far); out[15] = 0f
    }

    fun orthoM(out: FloatArray, left: Float, right: Float, bottom: Float, top: Float, near: Float, far: Float) {
        val rl = right - left
        val tb = top - bottom
        val nf = near - far
        out[0] = 2 / rl; out[1] = 0f; out[2] = 0f; out[3] = 0f
        out[4] = 0f; out[5] = 2 / tb; out[6] = 0f; out[7] = 0f
        out[8] = 0f; out[9] = 0f; out[10] = -2 / nf; out[11] = 0f
        out[12] = -(right + left) / rl; out[13] = -(top + bottom) / tb; out[14] = -(far + near) / nf; out[15] = 1f
    }

    fun setIdentity(out: FloatArray) {
        for (i in 0 until 16) out[i] = 0f
        out[0] = 1f; out[5] = 1f; out[10] = 1f; out[15] = 1f
    }

    fun translateM(out: FloatArray, m: FloatArray, x: Float, y: Float, z: Float) {
        for (i in 0 until 16) out[i] = m[i]
        out[12] = m[0] * x + m[4] * y + m[8] * z + m[12]
        out[13] = m[1] * x + m[5] * y + m[9] * z + m[13]
        out[14] = m[2] * x + m[6] * y + m[10] * z + m[14]
        out[15] = m[3] * x + m[7] * y + m[11] * z + m[15]
    }

    fun rotateM(out: FloatArray, m: FloatArray, angleDeg: Float, x: Float, y: Float, z: Float) {
        val r = FloatArray(16)
        setRotateM(r, angleDeg, x, y, z)
        multiply(out, m, r)
    }

    fun setRotateM(out: FloatArray, angleDeg: Float, x: Float, y: Float, z: Float) {
        val angle = Math.toRadians(angleDeg.toDouble())
        val s = Math.sin(angle).toFloat()
        val c = Math.cos(angle).toFloat()
        val len = Math.sqrt((x * x + y * y + z * z).toDouble()).toFloat()
        val nx = x / len; val ny = y / len; val nz = z / len
        out[0] = c + nx * nx * (1 - c); out[1] = ny * nx * (1 - c) + nz * s; out[2] = nz * nx * (1 - c) - ny * s; out[3] = 0f
        out[4] = nx * ny * (1 - c) - nz * s; out[5] = c + ny * ny * (1 - c); out[6] = nz * ny * (1 - c) + nx * s; out[7] = 0f
        out[8] = nx * nz * (1 - c) + ny * s; out[9] = ny * nz * (1 - c) - nx * s; out[10] = c + nz * nz * (1 - c); out[11] = 0f
        out[12] = 0f; out[13] = 0f; out[14] = 0f; out[15] = 1f
    }

    fun scaleM(out: FloatArray, m: FloatArray, sx: Float, sy: Float, sz: Float) {
        out[0] = m[0] * sx; out[1] = m[1] * sx; out[2] = m[2] * sx; out[3] = m[3] * sx
        out[4] = m[4] * sy; out[5] = m[5] * sy; out[6] = m[6] * sy; out[7] = m[7] * sy
        out[8] = m[8] * sz; out[9] = m[9] * sz; out[10] = m[10] * sz; out[11] = m[11] * sz
        for (i in 12 until 16) out[i] = m[i]
    }
}