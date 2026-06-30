package com.humours.client.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.humours.client.data.network.ConnectionState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConnectionScreen(
    onConnected: () -> Unit,
    onBack: () -> Unit = {},
) {
    val vm: ConnectionViewModel = viewModel()
    val settings by vm.settings.collectAsState()
    val state by vm.connectionState.collectAsState()

    val host = remember(settings.host) { mutableStateOf(settings.host) }
    val port = remember(settings.port) { mutableStateOf(settings.port.toString()) }
    val token = remember(settings.token) { mutableStateOf(settings.token) }
    var navigated by remember { mutableStateOf(false) }

    LaunchedEffect(state) {
        if (state is ConnectionState.Connected && !navigated) {
            navigated = true
            onConnected()
        }
        if (state !is ConnectionState.Connected) navigated = false
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Humours") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).padding(16.dp).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            OutlinedTextField(host.value, { host.value = it }, label = { Text("Server host") }, modifier = Modifier.fillMaxWidth())
            OutlinedTextField(
                port.value,
                { port.value = it.filter { c -> c.isDigit() } },
                label = { Text("Port") },
                modifier = Modifier.fillMaxWidth()
            )
            OutlinedTextField(token.value, { token.value = it }, label = { Text("Auth token") }, modifier = Modifier.fillMaxWidth())
            Button(
                onClick = {
                    val p = port.value.toIntOrNull() ?: 8443
                    vm.connect(host.value.trim(), p, token.value.trim())
                },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Connect") }
            Spacer(Modifier.height(8.dp))
            when (val s = state) {
                is ConnectionState.Idle -> Text("Idle")
                is ConnectionState.Connecting -> Text("Connecting to ${s.url}...")
                is ConnectionState.Connected -> Text("Connected to ${s.url}")
                is ConnectionState.Failed -> Text("Failed: ${s.message}")
            }
        }
    }
}