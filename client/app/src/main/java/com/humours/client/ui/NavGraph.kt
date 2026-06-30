package com.humours.client.ui

import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController

object Routes {
    const val GRID = "grid"
    const val CONNECTION = "connection"
    const val SETTINGS = "settings"
    const val ABOUT = "about"
}

@Composable
fun NavGraph() {
    val navController = rememberNavController()
    NavHost(navController, startDestination = Routes.GRID) {
        composable(Routes.GRID) {
            GridScreen(
                onDisconnect = {
                    navController.navigate(Routes.CONNECTION) {
                        popUpTo(Routes.GRID) { inclusive = true }
                    }
                },
                onNavigate = { route ->
                    navController.navigate(route)
                },
            )
        }
        composable(Routes.CONNECTION) {
            ConnectionScreen(
                onConnected = {
                    navController.navigate(Routes.GRID) {
                        popUpTo(Routes.CONNECTION) { inclusive = true }
                    }
                },
                onBack = {
                    navController.popBackStack(Routes.GRID, inclusive = false)
                },
            )
        }
        composable(Routes.SETTINGS) {
            SettingsScreen(onBack = { navController.popBackStack() })
        }
        composable(Routes.ABOUT) {
            AboutScreen(onBack = { navController.popBackStack() })
        }
    }
}