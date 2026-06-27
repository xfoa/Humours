package com.humours.client.data.network

sealed class ConnectionState {
    data object Idle : ConnectionState()
    data class Connecting(val url: String) : ConnectionState()
    data class Connected(val url: String) : ConnectionState()
    data class Failed(val message: String) : ConnectionState()
}