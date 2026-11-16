package com.n8vcl.roster_logger

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.core.content.FileProvider
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL

/**
 * Lets the page install a newer release: downloads the APK from this project's GitHub
 * releases and hands it to Android's installer, which asks the user to confirm and only
 * accepts an APK signed with the same key as the installed app.
 * Progress goes back to the page through window.hamnetApkProgress(state, text).
 */
class ApkUpdater(private val activity: Activity, private val webView: WebView) {
  companion object {
    const val RELEASES = "https://github.com/scottws70/Net-Roster-Logger/releases/download/"
  }

  @JavascriptInterface
  fun install(url: String) {
    if (!url.startsWith(RELEASES) || !url.endsWith(".apk")) {
      report("error", "That isn't a Net Roster Logger download.")
      return
    }
    if (Build.VERSION.SDK_INT >= 26 && !activity.packageManager.canRequestPackageInstalls()) {
      report("error", "Turn on \"Allow from this source\" in the screen that just opened, come back, and press Install Update again.")
      activity.runOnUiThread {
        activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + activity.packageName)))
      }
      return
    }
    Thread {
      try {
        val dir = File(activity.cacheDir, "updates")
        dir.mkdirs()
        dir.listFiles()?.forEach { it.delete() }
        val file = File(dir, "update.apk")
        // GitHub redirects downloads to its file servers; HttpURLConnection follows https→https redirects
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.instanceFollowRedirects = true
        conn.connectTimeout = 15000
        conn.readTimeout = 30000
        if (conn.responseCode != 200) throw IOException("the server answered ${conn.responseCode}")
        val total = conn.contentLengthLong
        conn.inputStream.use { input ->
          file.outputStream().use { out ->
            val buf = ByteArray(64 * 1024)
            var got = 0L
            var last = -1
            while (true) {
              val n = input.read(buf)
              if (n < 0) break
              out.write(buf, 0, n)
              got += n
              if (total > 0) {
                val pct = (got * 100 / total).toInt()
                if (pct != last) { last = pct; report("progress", "Downloading… $pct%") }
              }
            }
          }
        }
        val uri = FileProvider.getUriForFile(activity, activity.packageName + ".fileprovider", file)
        val intent = Intent(Intent.ACTION_VIEW)
          .setDataAndType(uri, "application/vnd.android.package-archive")
          .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        activity.runOnUiThread { activity.startActivity(intent) }
        report("done", "")
      } catch (e: Exception) {
        report("error", "Download failed: ${e.message}")
      }
    }.start()
  }

  private fun report(state: String, text: String) {
    val js = "window.hamnetApkProgress && window.hamnetApkProgress(${JSONObject.quote(state)}, ${JSONObject.quote(text)})"
    activity.runOnUiThread { webView.evaluateJavascript(js, null) }
  }
}
