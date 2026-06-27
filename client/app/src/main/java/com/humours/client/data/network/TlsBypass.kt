package com.humours.client.data.network

import okhttp3.OkHttpClient
import java.security.SecureRandom
import java.security.cert.X509Certificate
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager

object TlsBypass {
    private val trustAllManager = object : X509TrustManager {
        override fun checkClientTrusted(chain: Array<X509Certificate>, authType: String) {}
        override fun checkServerTrusted(chain: Array<X509Certificate>, authType: String) {}
        override fun getAcceptedIssuers(): Array<X509Certificate> = arrayOf()
    }

    fun trustAllSslContext(): SSLContext {
        val ctx = SSLContext.getInstance("TLS")
        ctx.init(null, arrayOf<TrustManager>(trustAllManager), SecureRandom())
        return ctx
    }

    val trustManager: X509TrustManager = trustAllManager
}

fun unsafeOkHttpClient(): OkHttpClient {
    val sslContext = TlsBypass.trustAllSslContext()
    return OkHttpClient.Builder()
        .sslSocketFactory(sslContext.socketFactory, TlsBypass.trustManager)
        .hostnameVerifier { _, _ -> true }
        .pingInterval(java.time.Duration.ofSeconds(20))
        .build()
}