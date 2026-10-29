// The Android app's activity (docs/behavior/android.md): the system's
// NativeActivity, which loads libtracedraw.so and hands it the window and
// the input, plus the few things the editor can only do from Java: the
// system document picker (Storage Access Framework), reading and writing
// the documents it returns, and the system clipboard. The Rust side calls
// the public methods through JNI from its own thread; a picked document
// comes back through the native method nativeDocumentResult. Built by
// scripts/android/build.sh with javac and d8 into classes.dex.
package dev.tracedraw.app;

import android.app.NativeActivity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.util.Log;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.concurrent.Callable;
import java.util.concurrent.FutureTask;
import java.util.concurrent.TimeUnit;

public class TraceDrawActivity extends NativeActivity {
    private static final String TAG = "tracedraw";

    static {
        // The same library NativeActivity loads (android.app.lib_name), so
        // that the native methods below resolve to its symbols.
        System.loadLibrary("tracedraw");
    }

    /**
     * The documents picked for the request `request` (a code the Rust side
     * chose), as newline-separated lists of the same length: the content
     * URIs, their display names and their MIME types (an empty entry when
     * unknown). All three are null when the picker was cancelled or failed.
     * Implemented in Rust (android.rs).
     */
    private native void nativeDocumentResult(int request, String uris, String names, String mimes);

    // ----- the document picker -----------------------------------------------

    /**
     * Show the system's document picker for one document (or several when
     * `multiple`) of any type; the editor tells the type from the name.
     * The result comes through nativeDocumentResult.
     */
    public void openDocument(final int request, final boolean multiple) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                try {
                    Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                    intent.addCategory(Intent.CATEGORY_OPENABLE);
                    intent.setType("*/*");
                    intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, multiple);
                    startActivityForResult(intent, request);
                } catch (Exception e) {
                    Log.w(TAG, "the document picker could not open", e);
                    nativeDocumentResult(request, null, null, null);
                }
            }
        });
    }

    /**
     * Show the system's picker for a place to save a new document named
     * `name` of type `mime`. The result comes through nativeDocumentResult.
     */
    public void createDocument(final int request, final String mime, final String name) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                try {
                    Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                    intent.addCategory(Intent.CATEGORY_OPENABLE);
                    intent.setType(mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
                    intent.putExtra(Intent.EXTRA_TITLE, name);
                    startActivityForResult(intent, request);
                } catch (Exception e) {
                    Log.w(TAG, "the save picker could not open", e);
                    nativeDocumentResult(request, null, null, null);
                }
            }
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (resultCode != RESULT_OK || data == null) {
            nativeDocumentResult(requestCode, null, null, null);
            return;
        }
        ArrayList<Uri> uris = new ArrayList<Uri>();
        if (data.getData() != null) {
            uris.add(data.getData());
        }
        ClipData clip = data.getClipData();
        if (clip != null) {
            for (int i = 0; i < clip.getItemCount(); i++) {
                Uri u = clip.getItemAt(i).getUri();
                if (u != null && !uris.contains(u)) {
                    uris.add(u);
                }
            }
        }
        if (uris.isEmpty()) {
            nativeDocumentResult(requestCode, null, null, null);
            return;
        }
        // Keep the right to read (and write, when granted) the document
        // after the process restarts, so the recent list reopens it.
        int flags = data.getFlags()
            & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        StringBuilder u = new StringBuilder();
        StringBuilder n = new StringBuilder();
        StringBuilder m = new StringBuilder();
        for (int i = 0; i < uris.size(); i++) {
            Uri uri = uris.get(i);
            try {
                if (flags != 0) {
                    getContentResolver().takePersistableUriPermission(uri, flags);
                }
            } catch (Exception e) {
                Log.i(TAG, "no persistable permission for " + uri);
            }
            if (i > 0) {
                u.append('\n');
                n.append('\n');
                m.append('\n');
            }
            u.append(uri.toString());
            n.append(displayName(uri).replace('\n', ' '));
            String mime = null;
            try {
                mime = getContentResolver().getType(uri);
            } catch (Exception e) {
                // Unknown type: the editor goes by the name.
            }
            m.append(mime == null ? "" : mime.replace('\n', ' '));
        }
        nativeDocumentResult(requestCode, u.toString(), n.toString(), m.toString());
    }

    /** The name the document's provider shows for it; its last path segment when it has none. */
    private String displayName(Uri uri) {
        Cursor cursor = null;
        try {
            cursor = getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null);
            if (cursor != null && cursor.moveToFirst()) {
                int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (column >= 0) {
                    String name = cursor.getString(column);
                    if (name != null && !name.isEmpty()) {
                        return name;
                    }
                }
            }
        } catch (Exception e) {
            Log.i(TAG, "no display name for " + uri);
        } finally {
            if (cursor != null) {
                cursor.close();
            }
        }
        String last = uri.getLastPathSegment();
        if (last == null || last.isEmpty()) {
            return "document";
        }
        // Document ids look like "primary:Pictures/a.png": the name is the
        // part after the last separator.
        int cut = Math.max(last.lastIndexOf('/'), last.lastIndexOf(':'));
        return cut >= 0 && cut + 1 < last.length() ? last.substring(cut + 1) : last;
    }

    // ----- reading and writing documents --------------------------------------

    /** The whole content of the document at `uri`; null when it cannot be read. */
    public byte[] readDocument(String uri) {
        InputStream in = null;
        try {
            in = getContentResolver().openInputStream(Uri.parse(uri));
            if (in == null) {
                return null;
            }
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buffer = new byte[64 * 1024];
            int got;
            while ((got = in.read(buffer)) != -1) {
                out.write(buffer, 0, got);
            }
            return out.toByteArray();
        } catch (Exception e) {
            Log.w(TAG, "cannot read " + uri + ": " + e);
            return null;
        } finally {
            close(in);
        }
    }

    /**
     * Replace the content of the document at `uri` with `data`; false when
     * it cannot be written. The document is truncated first ("wt"); a
     * provider without that mode gets a plain write.
     */
    public boolean writeDocument(String uri, byte[] data) {
        ContentResolver resolver = getContentResolver();
        OutputStream out = null;
        try {
            try {
                out = resolver.openOutputStream(Uri.parse(uri), "wt");
            } catch (Exception e) {
                out = resolver.openOutputStream(Uri.parse(uri), "w");
            }
            if (out == null) {
                return false;
            }
            out.write(data);
            out.flush();
            return true;
        } catch (Exception e) {
            Log.w(TAG, "cannot write " + uri + ": " + e);
            return false;
        } finally {
            close(out);
        }
    }

    private static void close(java.io.Closeable c) {
        if (c != null) {
            try {
                c.close();
            } catch (Exception e) {
                // Nothing left to do with it.
            }
        }
    }

    // ----- the system clipboard -----------------------------------------------

    /** The clipboard's text, or null when it holds none (or the app is not in front). */
    public String getClipboardText() {
        return onUiThread(new Callable<String>() {
            @Override
            public String call() {
                ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard == null || !clipboard.hasPrimaryClip()) {
                    return null;
                }
                ClipData clip = clipboard.getPrimaryClip();
                if (clip == null || clip.getItemCount() == 0) {
                    return null;
                }
                CharSequence text = clip.getItemAt(0).coerceToText(TraceDrawActivity.this);
                return text == null || text.length() == 0 ? null : text.toString();
            }
        }, null);
    }

    /** Put `text` on the clipboard; whether it went there. */
    public boolean setClipboardText(final String text) {
        Boolean done = onUiThread(new Callable<Boolean>() {
            @Override
            public Boolean call() {
                ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard == null) {
                    return Boolean.FALSE;
                }
                clipboard.setPrimaryClip(ClipData.newPlainText("TraceDraw", text));
                return Boolean.TRUE;
            }
        }, Boolean.FALSE);
        return done != null && done.booleanValue();
    }

    /**
     * Run `task` on the UI thread and wait for its result (two seconds at
     * most); `fallback` when it fails or takes longer. The caller is the
     * editor's own thread, so the UI thread is free to run it.
     */
    private <T> T onUiThread(Callable<T> task, T fallback) {
        FutureTask<T> future = new FutureTask<T>(task);
        runOnUiThread(future);
        try {
            return future.get(2, TimeUnit.SECONDS);
        } catch (Exception e) {
            Log.w(TAG, "UI thread call failed: " + e);
            return fallback;
        }
    }
}
