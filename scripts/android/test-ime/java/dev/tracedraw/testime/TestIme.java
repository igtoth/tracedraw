// The emulator check's keyboard (scripts/android/emulator_check.sh): an
// input method bound by the system like any on-screen keyboard, which
// types what adb broadcasts to it. Each broadcast of
// dev.tracedraw.testime.TYPE may carry
//   compose  the text to compose (setComposingText),
//   commit   the text to commit (commitText),
//   finish   true: end the composition (finishComposingText),
// each text as Unicode code points in hexadecimal separated by spaces
// ("4f60 597d" is 你好), since adb's shell does not pass every
// character through. The keyboard writes to the system log (tag
// tracedraw-ime) which editor it serves and what it typed.
package dev.tracedraw.testime;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.inputmethodservice.InputMethodService;
import android.os.Build;
import android.util.Log;
import android.view.View;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.widget.TextView;

public final class TestIme extends InputMethodService {
    private static final String TAG = "tracedraw-ime";
    private static final String ACTION = "dev.tracedraw.testime.TYPE";

    private BroadcastReceiver receiver;

    @Override
    public void onCreate() {
        super.onCreate();
        receiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context context, Intent intent) {
                type(intent);
            }
        };
        IntentFilter filter = new IntentFilter(ACTION);
        if (Build.VERSION.SDK_INT >= 33) {
            registerReceiver(receiver, filter, Context.RECEIVER_EXPORTED);
        } else {
            registerReceiver(receiver, filter);
        }
        Log.i(TAG, "created");
    }

    @Override
    public void onDestroy() {
        if (receiver != null) {
            unregisterReceiver(receiver);
        }
        super.onDestroy();
    }

    @Override
    public View onCreateInputView() {
        TextView view = new TextView(this);
        view.setText("TraceDraw test keyboard");
        view.setMinHeight(200);
        return view;
    }

    @Override
    public boolean onEvaluateFullscreenMode() {
        return false;
    }

    // Shown on request as on a phone without a keyboard, also when the
    // emulator reports a hardware keyboard (the system's default then
    // keeps the view hidden and onStartInputView never comes).
    @Override
    public boolean onEvaluateInputViewShown() {
        return true;
    }

    @Override
    public void onStartInput(EditorInfo info, boolean restarting) {
        super.onStartInput(info, restarting);
        Log.i(TAG, "input starts: " + describe(info) + (restarting ? ", restarting" : ""));
    }

    @Override
    public void onStartInputView(EditorInfo info, boolean restarting) {
        super.onStartInputView(info, restarting);
        Log.i(TAG, "keyboard shown for " + describe(info));
    }

    @Override
    public void onFinishInputView(boolean finishingInput) {
        super.onFinishInputView(finishingInput);
        Log.i(TAG, "keyboard hidden");
    }

    private static String describe(EditorInfo info) {
        if (info == null) {
            return "no editor";
        }
        return info.packageName + " (input type 0x" + Integer.toHexString(info.inputType) + ")";
    }

    private void type(Intent intent) {
        InputConnection ic = getCurrentInputConnection();
        String editor = describe(getCurrentInputEditorInfo());
        if (ic == null) {
            Log.w(TAG, "nothing to type into: no input connection (" + editor + ")");
            return;
        }
        String compose = text(intent.getStringExtra("compose"));
        String commit = text(intent.getStringExtra("commit"));
        boolean finish = "true".equals(intent.getStringExtra("finish"));
        boolean ok = true;
        if (compose != null) {
            ok &= ic.setComposingText(compose, 1);
        }
        if (commit != null) {
            ok &= ic.commitText(commit, 1);
        }
        if (finish) {
            ok &= ic.finishComposingText();
        }
        Log.i(TAG, "typed into " + editor + ": compose " + intent.getStringExtra("compose")
                + ", commit " + intent.getStringExtra("commit") + (finish ? ", finish" : "")
                + (ok ? "" : " (the editor refused)"));
    }

    /** Code points in hexadecimal separated by spaces, as text; null for none. */
    private static String text(String hex) {
        if (hex == null) {
            return null;
        }
        StringBuilder out = new StringBuilder();
        for (String part : hex.trim().split("[ ,]+")) {
            if (part.isEmpty()) {
                continue;
            }
            try {
                out.appendCodePoint(Integer.parseInt(part, 16));
            } catch (NumberFormatException e) {
                Log.w(TAG, "not a code point: " + part);
            }
        }
        return out.toString();
    }
}
