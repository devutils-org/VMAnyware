package dev.vmanyware.client

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import android.view.View
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
      navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT)
    )
    super.onCreate(savedInstanceState)
    window.setBackgroundDrawable(ColorDrawable(Color.rgb(5, 5, 7)))
    // Reserve keyboard space natively as well: older WebViews do not resize
    // their visual viewport for IME insets. Zero all handled types below so
    // newer WebViews do not apply those same dimensions again.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val handled = WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout() or WindowInsetsCompat.Type.ime()
      val bars = insets.getInsets(handled)
      view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      // Forward zero-valued updates instead of blocking inset notifications.
      WindowInsetsCompat.Builder(insets).setInsets(handled, Insets.NONE).build()
    }
    ViewCompat.requestApplyInsets(content)
  }
}
