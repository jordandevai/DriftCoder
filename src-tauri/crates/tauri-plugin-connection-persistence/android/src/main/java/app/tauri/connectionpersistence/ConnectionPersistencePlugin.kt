package app.tauri.connectionpersistence

import android.app.Activity
import android.content.Intent
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import app.tauri.Logger
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import kotlin.math.max

@InvokeArg
class ActiveArgs {
  var active: Boolean = false
}

@TauriPlugin
class ConnectionPersistencePlugin(private val activity: Activity) : Plugin(activity) {
  private var lastEmittedBottomPx: Int = -1

  private fun setNativeKeyboardInset(webView: android.webkit.WebView, bottomPx: Int) {
    val clamped = max(0, bottomPx)
    if (clamped == lastEmittedBottomPx) return
    lastEmittedBottomPx = clamped

    val js = """
      try {
        document.documentElement.style.setProperty('--native-keyboard-inset-bottom', '${clamped}px');
        window.dispatchEvent(new CustomEvent('native-ime-insets', { detail: { bottomPx: ${clamped} } }));
      } catch (e) {}
    """.trimIndent()
    webView.post { webView.evaluateJavascript(js, null) }
  }

  private fun prefs() =
    activity.getSharedPreferences(PREFS_NAME, Activity.MODE_PRIVATE)

  private fun captureDisconnectIntent(intent: Intent?) {
    if (intent?.action != ACTION_DISCONNECT_ALL) return
    prefs().edit().putBoolean(KEY_DISCONNECT_REQUESTED, true).apply()
    // Clear the action so we don't re-run disconnect on future resumes.
    intent.action = null
    activity.intent = intent
  }

  override fun load(webView: android.webkit.WebView) {
    super.load(webView)
    captureDisconnectIntent(activity.intent)

    // Track IME (soft keyboard) height and expose it to the web layer via a CSS variable.
    try {
      val root = activity.window?.decorView ?: webView

      ViewCompat.setOnApplyWindowInsetsListener(root) { _, insets ->
        val imeBottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
        // Subtract system bars so we only report the keyboard portion
        val systemBottom = insets.getInsets(WindowInsetsCompat.Type.systemBars()).bottom
        val keyboardBottom = max(0, imeBottom - systemBottom)

        setNativeKeyboardInset(webView, keyboardBottom)
        insets
      }
      root.post { ViewCompat.requestApplyInsets(root) }
    } catch (e: Exception) {
      Logger.error("Failed to install IME insets listener: ${e.message}")
    }
  }

  override fun onNewIntent(intent: Intent) {
    captureDisconnectIntent(intent)
  }

  override fun onPause() {
    // Activity is leaving foreground; if there are active sessions, start the FGS to keep sockets alive.
    val active = prefs().getBoolean(KEY_ACTIVE, false)
    if (!active) return
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      ContextCompat.startForegroundService(activity, intent)
    } catch (e: Exception) {
      Logger.error("Failed to start background persistence service: ${e.message}")
    }
  }

  override fun onResume() {
    captureDisconnectIntent(activity.intent)
    // Always stop the FGS when returning to foreground.
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      activity.stopService(intent)
    } catch (_: Exception) {
      // ignore
    }
  }

  @Command
  fun start(invoke: Invoke) {
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      ContextCompat.startForegroundService(activity, intent)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to start background persistence")
    }
  }

  @Command
  fun stop(invoke: Invoke) {
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      activity.stopService(intent)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to stop background persistence")
    }
  }

  @Command
  fun setActive(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ActiveArgs::class.java)
      prefs().edit().putBoolean(KEY_ACTIVE, args.active).apply()
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to update background persistence state")
    }
  }

  @Command
  fun consumeDisconnectRequest(invoke: Invoke) {
    try {
      val p = prefs()
      val requested = p.getBoolean(KEY_DISCONNECT_REQUESTED, false)
      if (requested) {
        p.edit().remove(KEY_DISCONNECT_REQUESTED).apply()
      }
      val out = JSObject()
      out.put("requested", requested)
      invoke.resolve(out)
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to read disconnect request")
    }
  }

  companion object {
    private const val PREFS_NAME = "driftcode_connection_persistence"
    private const val KEY_DISCONNECT_REQUESTED = "disconnect_requested"
    private const val KEY_ACTIVE = "active"
    private const val ACTION_DISCONNECT_ALL = "app.tauri.connectionpersistence.action.DISCONNECT_ALL"
  }
}
